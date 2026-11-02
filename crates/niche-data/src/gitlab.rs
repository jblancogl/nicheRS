//! GitLab issues for user feedback (ADR-0110): attachments go to the project's uploads, the
//! issue embeds them. One attempt per call (a retried POST could open the same issue twice);
//! the token is never logged or included in errors.

use serde::Deserialize;

use crate::DataError;
use crate::http::{Http, MAX_BODY};

/// The GitLab project that receives feedback issues.
#[derive(Debug, Clone)]
pub struct GitLabProject {
    /// Instance root, e.g. `https://gitlab.example.org`.
    pub url: String,
    /// Project id or `group/project` path.
    pub project: String,
    /// Project access token (scope `api`, role Reporter or more).
    pub token: String,
    /// Labels added to every issue, besides the feedback kind.
    pub labels: Vec<String>,
}

/// An issue as created.
#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub iid: u64,
    pub web_url: String,
}

#[derive(Deserialize)]
struct Upload {
    markdown: String,
}

impl GitLabProject {
    fn api(&self, path: &str) -> String {
        let project: String = self
            .project
            .bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect();
        format!(
            "{}/api/v4/projects/{project}{path}",
            self.url.trim_end_matches('/')
        )
    }
}

/// `multipart/form-data` body with one `file` field.
fn multipart(boundary: &str, filename: &str, mime: &str, bytes: &[u8]) -> Vec<u8> {
    let safe: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{safe}\"\r\nContent-Type: {mime}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

impl Http {
    fn gitlab_post(
        &self,
        gl: &GitLabProject,
        url: &str,
        content_type: &str,
        body: &[u8],
    ) -> Result<String, DataError> {
        let mut resp = self
            .agent
            .post(url)
            .header("PRIVATE-TOKEN", &gl.token)
            .header("Content-Type", content_type)
            .send(body)
            .map_err(|e| DataError::Network {
                url: url.into(),
                msg: e.to_string(),
            })?;
        let status = resp.status().as_u16();
        let text = resp
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_string()
            .map_err(|e| DataError::Network {
                url: url.into(),
                msg: e.to_string(),
            })?;
        if !(200..300).contains(&status) {
            let hint = if status == 401 || status == 403 {
                " (check the GitLab token and its role)"
            } else {
                ""
            };
            return Err(DataError::Format {
                url: url.into(),
                msg: format!("HTTP {status}{hint}"),
            });
        }
        Ok(text)
    }

    /// Upload a file to the project; returns the Markdown that embeds it (`![name](/uploads/…)`).
    pub fn gitlab_upload(
        &self,
        gl: &GitLabProject,
        filename: &str,
        mime: &str,
        bytes: &[u8],
    ) -> Result<String, DataError> {
        let url = gl.api("/uploads");
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let boundary = format!("----niche-feedback-{nanos:x}-{:x}", bytes.len());
        let body = multipart(&boundary, filename, mime, bytes);
        let text = self.gitlab_post(
            gl,
            &url,
            &format!("multipart/form-data; boundary={boundary}"),
            &body,
        )?;
        let up: Upload = serde_json::from_str(&text).map_err(|e| DataError::Format {
            url: url.clone(),
            msg: e.to_string(),
        })?;
        Ok(up.markdown)
    }

    /// Open an issue with the project's labels plus `extra_labels`.
    pub fn gitlab_issue(
        &self,
        gl: &GitLabProject,
        title: &str,
        description: &str,
        extra_labels: &[String],
    ) -> Result<Issue, DataError> {
        let url = gl.api("/issues");
        let labels: Vec<&str> = gl
            .labels
            .iter()
            .chain(extra_labels)
            .map(String::as_str)
            .filter(|l| !l.is_empty())
            .collect();
        let body = serde_json::json!({
            "title": title,
            "description": description,
            "labels": labels.join(","),
        })
        .to_string();
        let text = self.gitlab_post(gl, &url, "application/json", body.as_bytes())?;
        serde_json::from_str(&text).map_err(|e| DataError::Format {
            url,
            msg: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_paths_are_encoded() {
        let gl = GitLabProject {
            url: "https://gitlab.example.org/".into(),
            project: "natural-solutions/poc/nichens".into(),
            token: String::new(),
            labels: vec![],
        };
        assert_eq!(
            gl.api("/issues"),
            "https://gitlab.example.org/api/v4/projects/natural-solutions%2Fpoc%2Fnichens/issues"
        );
    }

    #[test]
    fn multipart_has_one_sanitised_file_field() {
        let body = multipart("b", "my \"shot\".png", "image/png", b"PNG");
        let text = String::from_utf8(body).expect("utf8");
        assert_eq!(
            text,
            "--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"my__shot_.png\"\r\nContent-Type: image/png\r\n\r\nPNG\r\n--b--\r\n"
        );
    }
}
