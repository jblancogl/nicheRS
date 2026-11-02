//! Feedback forwarding to the operator's GitLab (ADR-0110) against a local fake GitLab:
//! attachments are uploaded and embedded, the issue is opened once, failures are kept and
//! retried. Offline.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use niche_app::dto::{FeedbackKind, FeedbackRequest, FeedbackUpload};
use niche_app::feedback::{self, Author, GitLabProject};
use niche_app::store::Store;
use niche_data::Http;

/// (method + path, body) of each request; answers 500 while `fail` is set.
type Seen = Arc<Mutex<Vec<(String, String)>>>;

fn fake_gitlab(fail: Arc<Mutex<bool>>) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("addr"));
    let seen: Seen = Arc::default();
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut stream = stream;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let target = line
                    .split_whitespace()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ");
                let mut len = 0usize;
                let mut token_ok = false;
                loop {
                    let mut h = String::new();
                    reader.read_line(&mut h).expect("header");
                    let h = h.trim_end();
                    if h.is_empty() {
                        break;
                    }
                    let lower = h.to_ascii_lowercase();
                    if let Some(v) = lower.strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    token_ok |= lower == "private-token: secret";
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).expect("body");
                log.lock()
                    .expect("log")
                    .push((target.clone(), String::from_utf8_lossy(&body).into_owned()));
                let (status, answer) = if *fail.lock().expect("fail") || !token_ok {
                    ("500 Internal Server Error", "{}".to_string())
                } else if target.ends_with("/uploads") {
                    (
                        "201 Created",
                        r#"{"markdown":"![voice.webm](/uploads/abc/voice.webm)"}"#.into(),
                    )
                } else {
                    (
                        "201 Created",
                        r#"{"iid":7,"web_url":"http://gitlab/issues/7"}"#.into(),
                    )
                };
                let resp = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{answer}",
                    answer.len()
                );
                if stream.write_all(resp.as_bytes()).is_err() {
                    break;
                }
            }
        }
    });
    (url, seen)
}

#[test]
fn feedback_is_forwarded_once_with_its_attachments() {
    let dir = std::env::temp_dir().join(format!("niche-feedback-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    let store = Store::open(&dir.join("niche.db")).expect("store");
    let org = store.org_create("Client A", 5).expect("org");
    let fail = Arc::new(Mutex::new(true));
    let (url, seen) = fake_gitlab(fail.clone());
    let gl = GitLabProject {
        url,
        project: "ns/nichens".into(),
        token: "secret".into(),
        labels: vec!["feedback".into()],
    };
    let http = Http::new();
    let id = store
        .feedback_create(
            &Author {
                org_id: &org.id,
                user_id: "u1",
            },
            &FeedbackRequest {
                kind: FeedbackKind::Problem,
                message: "/close\n@all the map is grey".into(),
                page: Some("/jobs/1/results".into()),
                context: serde_json::json!({"lang": "fr"}),
                attachments: vec![FeedbackUpload {
                    name: "voice.webm".into(),
                    mime: "audio/webm".into(),
                    data: "GkXfow==".into(),
                }],
            },
        )
        .expect("create");

    // GitLab down: the error is kept and the item stays pending.
    assert!(feedback::forward(&store, &http, &gl, &id).is_err());
    assert_eq!(
        store.feedback_unforwarded().expect("pending"),
        vec![id.clone()]
    );

    *fail.lock().expect("fail") = false;
    seen.lock().expect("seen").clear();
    assert_eq!(
        feedback::forward_pending(&store, &http, &gl).expect("forward"),
        1
    );
    assert!(store.feedback_unforwarded().expect("pending").is_empty());
    // Sent once: a second round has nothing to do.
    assert_eq!(
        feedback::forward_pending(&store, &http, &gl).expect("again"),
        0
    );
    assert_eq!(
        feedback::forward(&store, &http, &gl, &id).expect("again"),
        None
    );

    let seen = seen.lock().expect("seen").clone();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0].0, "POST /api/v4/projects/ns%2Fnichens/uploads");
    assert!(seen[0].1.contains("filename=\"voice.webm\""));
    assert_eq!(seen[1].0, "POST /api/v4/projects/ns%2Fnichens/issues");
    let issue: serde_json::Value = serde_json::from_str(&seen[1].1).expect("json");
    assert_eq!(issue["title"], "[Problem] /close");
    assert_eq!(issue["labels"], "feedback,feedback::problem");
    let d = issue["description"].as_str().expect("description");
    // The user's text is fenced: no quick action, no mention.
    assert!(
        d.starts_with("~~~text\n/close\n@all the map is grey\n~~~"),
        "{d}"
    );
    assert!(d.contains("![voice.webm](/uploads/abc/voice.webm)"));
    assert!(d.contains("| Organisation | `Client A` |"));
    assert!(d.contains(&format!("<!-- niche-feedback:{id} -->")));
}
