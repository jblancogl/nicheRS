//! Feedback from users during the alpha (ADR-0110): a message, screenshots and voice
//! recordings, stored with the organisation that sent it. Administrators read all of it,
//! organisation managers their organisation's. When the operator configures a GitLab project,
//! each item is also forwarded there as an issue; nothing of that is shown in the interface.

use jiff::{SignedDuration, Timestamp};
use niche_data::Http;
pub use niche_data::gitlab::GitLabProject;
use rusqlite::{Connection, OptionalExtension, params};

use crate::AppError;
use crate::accounts::{Principal, new_id};
use crate::dto::{
    Feedback, FeedbackFile, FeedbackKind, FeedbackRequest, FeedbackStatus, FeedbackUpload,
};
use crate::store::{Store, db, now};

pub const MAX_MESSAGE_CHARS: usize = 10_000;
pub const MAX_FILES: usize = 6;
/// Decoded bytes of all attachments of one item (a few screenshots and minutes of voice).
pub const MAX_FILE_BYTES: usize = 12 << 20;
const MAX_CONTEXT_BYTES: usize = 8 << 10;
const MAX_PAGE_CHARS: usize = 500;
/// A forwarding claim older than this is considered abandoned (server stopped mid-way).
const CLAIM_MINUTES: i64 = 10;

/// Attachment types accepted. SVG and HTML are excluded: files are served back from the
/// application's origin.
const MIMES: [&str; 10] = [
    "image/png",
    "image/jpeg",
    "image/webp",
    "image/gif",
    "audio/webm",
    "audio/ogg",
    "audio/mp4",
    "audio/mpeg",
    "audio/wav",
    "video/webm",
];

/// Who wrote it, decided by the server (never by the request).
pub struct Author<'a> {
    pub org_id: &'a str,
    pub user_id: &'a str,
}

/// An attachment as stored.
pub struct StoredFile {
    pub name: String,
    pub mime: String,
    pub data: Vec<u8>,
}

/// Standard base64 (RFC 4648), padding optional, whitespace ignored.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// `audio/webm;codecs=opus` → `audio/webm`, if accepted.
fn accepted_mime(mime: &str) -> Option<&'static str> {
    let base = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let base = if base == "audio/x-wav" || base == "audio/wave" {
        "audio/wav".to_string()
    } else {
        base
    };
    MIMES.into_iter().find(|m| *m == base)
}

fn file_name(name: &str, index: usize) -> String {
    let n: String = name
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '/' | '\\'))
        .take(120)
        .collect();
    let n = n.trim().to_string();
    if n.is_empty() {
        format!("attachment-{}", index + 1)
    } else {
        n
    }
}

fn decode_files(uploads: &[FeedbackUpload]) -> Result<Vec<StoredFile>, AppError> {
    if uploads.len() > MAX_FILES {
        return Err(AppError::Invalid(format!(
            "at most {MAX_FILES} attachments per message"
        )));
    }
    let mut total = 0usize;
    uploads
        .iter()
        .enumerate()
        .map(|(i, u)| {
            let mime = accepted_mime(&u.mime).ok_or_else(|| {
                AppError::Invalid(format!(
                    "attachment type {} is not accepted (images and audio only)",
                    u.mime
                ))
            })?;
            // Rejected before decoding: base64 is 4/3 of the bytes.
            if u.data.len() / 4 * 3 > MAX_FILE_BYTES {
                return Err(too_large());
            }
            let data = base64_decode(&u.data)
                .ok_or_else(|| AppError::Invalid("attachment is not valid base64".into()))?;
            total += data.len();
            if total > MAX_FILE_BYTES {
                return Err(too_large());
            }
            if data.is_empty() {
                return Err(AppError::Invalid("empty attachment".into()));
            }
            Ok(StoredFile {
                name: file_name(&u.name, i),
                mime: mime.into(),
                data,
            })
        })
        .collect()
}

fn too_large() -> AppError {
    AppError::Invalid(format!(
        "attachments are limited to {} MB per message",
        MAX_FILE_BYTES >> 20
    ))
}

/// May `p` read this item? Administrators: all; managers: their organisation's; anyone: their own.
pub fn can_read(p: &Principal, org_id: Option<&str>, user_id: Option<&str>) -> bool {
    p.is_admin()
        || (p.manages_org() && org_id == Some(p.org_id.as_str()))
        || user_id == Some(p.user_id.as_str())
}

const SELECT: &str =
    "SELECT f.id, f.created_at, f.kind, f.message, f.page, f.context, u.name, u.email,
     o.name, f.status FROM feedback f LEFT JOIN users u ON u.id = f.user_id
     LEFT JOIN orgs o ON o.id = f.org_id";

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Feedback> {
    let context: String = r.get(5)?;
    Ok(Feedback {
        id: r.get(0)?,
        created_at: r.get(1)?,
        kind: FeedbackKind::parse(&r.get::<_, String>(2)?),
        message: r.get(3)?,
        page: r.get(4)?,
        context: serde_json::from_str(&context).unwrap_or_default(),
        author: r.get(6)?,
        author_email: r.get(7)?,
        org_name: r.get(8)?,
        status: FeedbackStatus::parse(&r.get::<_, String>(9)?),
        files: vec![],
    })
}

fn files_of(c: &Connection, id: &str) -> Result<Vec<FeedbackFile>, AppError> {
    let mut st = c
        .prepare(
            "SELECT id, name, mime, size FROM feedback_files WHERE feedback_id=?1 ORDER BY seq",
        )
        .map_err(db)?;
    st.query_map([id], |r| {
        Ok(FeedbackFile {
            id: r.get(0)?,
            name: r.get(1)?,
            mime: r.get(2)?,
            size: r.get::<_, i64>(3)?.max(0) as u64,
        })
    })
    .map_err(db)?
    .collect::<Result<_, _>>()
    .map_err(db)
}

impl Store {
    /// Store one item. A message, an attachment, or both are required.
    pub fn feedback_create(
        &self,
        by: &Author<'_>,
        r: &FeedbackRequest,
    ) -> Result<String, AppError> {
        let message = r.message.trim();
        if message.chars().count() > MAX_MESSAGE_CHARS {
            return Err(AppError::Invalid(format!(
                "the message is limited to {MAX_MESSAGE_CHARS} characters"
            )));
        }
        let files = decode_files(&r.attachments)?;
        if message.is_empty() && files.is_empty() {
            return Err(AppError::Invalid(
                "write a message or attach an image or a recording".into(),
            ));
        }
        let context = if r.context.is_object() {
            serde_json::to_string(&r.context).map_err(|e| AppError::Internal(e.to_string()))?
        } else {
            "{}".into()
        };
        if context.len() > MAX_CONTEXT_BYTES {
            return Err(AppError::Invalid("context is too large".into()));
        }
        let page = r
            .page
            .as_deref()
            .map(|p| p.chars().take(MAX_PAGE_CHARS).collect::<String>());
        let id = new_id();
        self.with(|c| {
            let tx = c.unchecked_transaction().map_err(db)?;
            tx.execute(
                "INSERT INTO feedback(id, org_id, user_id, kind, message, page, context, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![id, by.org_id, by.user_id, r.kind.as_str(), message, page, context, now()],
            )
            .map_err(db)?;
            for (seq, f) in files.iter().enumerate() {
                tx.execute(
                    "INSERT INTO feedback_files(id, feedback_id, seq, name, mime, size, data)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        new_id(),
                        id,
                        seq as i64,
                        f.name,
                        f.mime,
                        f.data.len() as i64,
                        f.data
                    ],
                )
                .map_err(db)?;
            }
            tx.commit().map_err(db)
        })?;
        Ok(id)
    }

    /// Newest first: every organisation's (`None`) or one organisation's.
    pub fn feedback_list(&self, org_id: Option<&str>) -> Result<Vec<Feedback>, AppError> {
        self.with(|c| {
            let mut st = c
                .prepare(&format!(
                    "{SELECT} WHERE ?1 IS NULL OR f.org_id = ?1 ORDER BY f.created_at DESC, f.id DESC"
                ))
                .map_err(db)?;
            let mut items: Vec<Feedback> = st
                .query_map([org_id], row)
                .map_err(db)?
                .collect::<Result<_, _>>()
                .map_err(db)?;
            for f in &mut items {
                f.files = files_of(c, &f.id)?;
            }
            Ok(items)
        })
    }

    /// Organisation and author of an item (for access checks).
    pub fn feedback_owner(&self, id: &str) -> Result<(Option<String>, Option<String>), AppError> {
        self.with(|c| {
            c.query_row(
                "SELECT org_id, user_id FROM feedback WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db)?
            .ok_or_else(|| AppError::NotFound(format!("feedback {id}")))
        })
    }

    /// One attachment, if `p` may read the item.
    pub fn feedback_file(
        &self,
        p: &Principal,
        id: &str,
        file_id: &str,
    ) -> Result<StoredFile, AppError> {
        let (org, user) = self.feedback_owner(id)?;
        if !can_read(p, org.as_deref(), user.as_deref()) {
            return Err(AppError::NotFound(format!("feedback {id}")));
        }
        self.with(|c| {
            c.query_row(
                "SELECT name, mime, data FROM feedback_files WHERE feedback_id=?1 AND id=?2",
                [id, file_id],
                |r| {
                    Ok(StoredFile {
                        name: r.get(0)?,
                        mime: r.get(1)?,
                        data: r.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(db)?
            .ok_or_else(|| AppError::NotFound(format!("attachment {file_id}")))
        })
    }

    pub fn feedback_set_status(&self, id: &str, status: FeedbackStatus) -> Result<(), AppError> {
        let n = self.with(|c| {
            c.execute(
                "UPDATE feedback SET status=?2 WHERE id=?1",
                params![id, status.as_str()],
            )
            .map_err(db)
        })?;
        if n == 0 {
            return Err(AppError::NotFound(format!("feedback {id}")));
        }
        Ok(())
    }

    /// Delete an item and its attachments (spam, or personal data sent by mistake).
    pub fn feedback_delete(&self, id: &str) -> Result<(), AppError> {
        let n = self.with(|c| {
            c.execute("DELETE FROM feedback WHERE id=?1", [id])
                .map_err(db)
        })?;
        if n == 0 {
            return Err(AppError::NotFound(format!("feedback {id}")));
        }
        Ok(())
    }

    /// Items not yet forwarded and not being forwarded right now, oldest first.
    pub fn feedback_unforwarded(&self) -> Result<Vec<String>, AppError> {
        let stale = stale_claim()?;
        self.with(|c| {
            let mut st = c
                .prepare(
                    "SELECT id FROM feedback WHERE forwarded_at IS NULL
                     AND (forwarding_at IS NULL OR forwarding_at < ?1) ORDER BY created_at",
                )
                .map_err(db)?;
            st.query_map([stale], |r| r.get(0))
                .map_err(db)?
                .collect::<Result<_, _>>()
                .map_err(db)
        })
    }

    /// Claim an item for forwarding; `false` if it was forwarded or is being forwarded.
    fn feedback_claim(&self, id: &str) -> Result<bool, AppError> {
        let stale = stale_claim()?;
        self.with(|c| {
            c.execute(
                "UPDATE feedback SET forwarding_at=?2 WHERE id=?1 AND forwarded_at IS NULL
                 AND (forwarding_at IS NULL OR forwarding_at < ?3)",
                params![id, now(), stale],
            )
            .map(|n| n == 1)
            .map_err(db)
        })
    }

    fn feedback_forwarded(&self, id: &str, result: Result<&str, &str>) -> Result<(), AppError> {
        self.with(|c| {
            match result {
                Ok(reference) => c.execute(
                    "UPDATE feedback SET forwarded_at=?2, forward_ref=?3, forward_error=NULL,
                     forwarding_at=NULL WHERE id=?1",
                    params![id, now(), reference],
                ),
                Err(error) => c.execute(
                    "UPDATE feedback SET forward_error=?2, forwarding_at=NULL WHERE id=?1",
                    params![id, error],
                ),
            }
            .map_err(db)
        })?;
        Ok(())
    }

    /// Everything the issue needs: the item and its attachments with their bytes.
    fn feedback_full(&self, id: &str) -> Result<(Feedback, Vec<StoredFile>), AppError> {
        self.with(|c| {
            let f = c
                .query_row(&format!("{SELECT} WHERE f.id=?1"), [id], row)
                .optional()
                .map_err(db)?
                .ok_or_else(|| AppError::NotFound(format!("feedback {id}")))?;
            let mut st = c
                .prepare(
                    "SELECT name, mime, data FROM feedback_files WHERE feedback_id=?1 ORDER BY seq",
                )
                .map_err(db)?;
            let files = st
                .query_map([id], |r| {
                    Ok(StoredFile {
                        name: r.get(0)?,
                        mime: r.get(1)?,
                        data: r.get(2)?,
                    })
                })
                .map_err(db)?
                .collect::<Result<_, _>>()
                .map_err(db)?;
            Ok((f, files))
        })
    }
}

fn stale_claim() -> Result<String, AppError> {
    Timestamp::now()
        .checked_sub(SignedDuration::from_mins(CLAIM_MINUTES))
        .map(|t| t.to_string())
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// A value for a Markdown table cell, as inline code: no mentions, links or line breaks.
fn cell(v: &str) -> String {
    let v: String = v
        .chars()
        .map(|c| {
            if c == '`' || c == '|' || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .take(300)
        .collect();
    if v.trim().is_empty() {
        "—".into()
    } else {
        format!("`{}`", v.trim())
    }
}

/// The user's text in a fenced block longer than any run of `~` inside it, so that GitLab runs
/// no quick action (`/close`…) and notifies no mention from it.
fn fenced(text: &str) -> String {
    let longest = text.split(|c| c != '~').map(str::len).max().unwrap_or(0);
    let fence = "~".repeat(longest.max(2) + 1);
    format!("{fence}text\n{text}\n{fence}")
}

fn title(f: &Feedback) -> String {
    let kind = match f.kind {
        FeedbackKind::Problem => "Problem",
        FeedbackKind::Idea => "Idea",
        FeedbackKind::Other => "Feedback",
    };
    let first: String = f
        .message
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    let first = first.trim();
    let summary = if first.is_empty() {
        "(no text, see attachments)".to_string()
    } else if first.chars().count() > 80 {
        format!("{}…", first.chars().take(80).collect::<String>())
    } else {
        first.to_string()
    };
    format!("[{kind}] {summary}")
}

/// Issue description: the message, the attachments embedded, then the context.
fn description(f: &Feedback, attachments: &[String]) -> String {
    let mut d = String::new();
    if f.message.is_empty() {
        d.push_str("_No text: see the attachments._\n\n");
    } else {
        d.push_str(&fenced(&f.message));
        d.push_str("\n\n");
    }
    if !attachments.is_empty() {
        d.push_str("### Attachments\n\n");
        for a in attachments {
            d.push_str(a);
            d.push_str("\n\n");
        }
    }
    let ctx = |k: &str| {
        f.context
            .get(k)
            .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
            .unwrap_or_default()
    };
    d.push_str("### Context\n\n| | |\n|---|---|\n");
    for (k, v) in [
        ("Kind", f.kind.as_str().to_string()),
        ("Organisation", f.org_name.clone().unwrap_or_default()),
        ("Sent", f.created_at.clone()),
        ("Page", f.page.clone().unwrap_or_default()),
        ("Study", ctx("job_id")),
        ("Build", ctx("build")),
        ("Language", ctx("lang")),
        ("Screen", ctx("screen")),
        ("Browser", ctx("user_agent")),
        ("Reference", f.id.clone()),
    ] {
        d.push_str(&format!("| {k} | {} |\n", cell(&v)));
    }
    d.push_str(&format!(
        "\n_Sent from NicheRS. The author's account is visible in the administration (reference above)._\n\n<!-- niche-feedback:{} -->\n",
        f.id
    ));
    d
}

/// Forward one item to GitLab as an issue (attachments uploaded first). Returns the issue URL,
/// or `None` if another worker forwarded or is forwarding it. Failures are recorded on the item
/// and retried by [`forward_pending`].
pub fn forward(
    store: &Store,
    http: &Http,
    gl: &GitLabProject,
    id: &str,
) -> Result<Option<String>, AppError> {
    if !store.feedback_claim(id)? {
        return Ok(None);
    }
    let sent = (|| {
        let (f, files) = store.feedback_full(id)?;
        let attachments = files
            .iter()
            .map(|file| http.gitlab_upload(gl, &file.name, &file.mime, &file.data))
            .collect::<Result<Vec<_>, _>>()?;
        let labels = vec![format!("feedback::{}", f.kind.as_str())];
        Ok::<_, AppError>(http.gitlab_issue(
            gl,
            &title(&f),
            &description(&f, &attachments),
            &labels,
        )?)
    })();
    match sent {
        Ok(issue) => {
            store.feedback_forwarded(id, Ok(&format!("#{}", issue.iid)))?;
            Ok(Some(issue.web_url))
        }
        Err(e) => {
            store.feedback_forwarded(id, Err(&e.to_string()))?;
            Err(e)
        }
    }
}

/// Forward every item not forwarded yet; returns how many were sent. Stops at the first failure
/// (GitLab unreachable or token rejected: the next round tries again).
pub fn forward_pending(store: &Store, http: &Http, gl: &GitLabProject) -> Result<usize, AppError> {
    let mut sent = 0;
    for id in store.feedback_unforwarded()? {
        if let Some(url) = forward(store, http, gl, &id)? {
            tracing::info!(feedback_id = %id, issue = %url, "feedback forwarded");
            sent += 1;
        }
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        assert_eq!(base64_decode("Zm9vYmFy").as_deref(), Some(&b"foobar"[..]));
        assert_eq!(base64_decode("Zm8=").as_deref(), Some(&b"fo"[..]));
        assert_eq!(base64_decode("Zg").as_deref(), Some(&b"f"[..]));
        assert_eq!(base64_decode("Zm9v\nYmFy").as_deref(), Some(&b"foobar"[..]));
        assert!(base64_decode("Zm9v!").is_none());
    }

    #[test]
    fn mimes() {
        assert_eq!(accepted_mime("audio/webm;codecs=opus"), Some("audio/webm"));
        assert_eq!(accepted_mime("IMAGE/PNG"), Some("image/png"));
        assert_eq!(accepted_mime("image/svg+xml"), None);
        assert_eq!(accepted_mime("text/html"), None);
    }

    #[test]
    fn user_text_cannot_act_on_gitlab() {
        let text = "/close\n@all look ~~~ here";
        let f = fenced(text);
        assert!(f.starts_with("~~~~text\n/close"));
        assert!(f.ends_with("\n~~~~"));
        assert_eq!(cell("a|b`c\nd"), "`a b c d`");
        assert_eq!(cell("  "), "—");
    }
}
