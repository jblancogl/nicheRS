//! Blocking HTTP client (ureq, rustls) with timeouts, retries and a polite User-Agent.

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use niche_geo::geotiff::{ByteSource, TiffError};

use crate::DataError;

const USER_AGENT: &str = concat!(
    "NicheRS/",
    env!("CARGO_PKG_VERSION"),
    " (species distribution modelling; Natural Solutions)"
);
pub(crate) const MAX_BODY: u64 = 256 << 20;

/// Outcome of one attempt.
enum Transient {
    Retry {
        msg: String,
        after: Option<Duration>,
    },
    Fatal(DataError),
}

fn retry_after(resp: &ureq::http::Response<ureq::Body>) -> Option<Duration> {
    resp.headers()
        .get("retry-after")?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

#[derive(Clone)]
pub struct Http {
    pub(crate) agent: ureq::Agent,
    /// Body bytes received by range requests (remote GeoTIFF reads), shared by clones.
    range_bytes: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Http {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(120)))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            range_bytes: Default::default(),
        }
    }

    /// Bytes received by range requests since this client was created (all clones together):
    /// what reading remote GeoTIFF windows (CHELSA, DEM, WorldCover) cost in transfer.
    pub fn range_bytes_received(&self) -> u64 {
        self.range_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Retry transient failures (network errors, HTTP 429/5xx) with exponential backoff, honouring
    /// `Retry-After` when the server sends it (GBIF rate limiting). Up to ~2 minutes in total.
    fn with_retry<T>(
        &self,
        url: &str,
        mut f: impl FnMut() -> Result<T, Transient>,
    ) -> Result<T, DataError> {
        let mut last = String::new();
        for attempt in 0..7u32 {
            match f() {
                Ok(v) => return Ok(v),
                Err(Transient::Retry { msg, after }) => {
                    last = msg;
                    let wait = after
                        .unwrap_or_else(|| Duration::from_millis(1000 * (1 << attempt)))
                        .min(Duration::from_secs(60));
                    tracing::debug!(%url, attempt, ?wait, "transient error, retrying");
                    std::thread::sleep(wait);
                }
                Err(Transient::Fatal(e)) => return Err(e),
            }
        }
        Err(DataError::Network {
            url: url.into(),
            msg: format!("{last} (gave up after retries)"),
        })
    }

    pub fn get_json(&self, url: &str) -> Result<serde_json::Value, DataError> {
        self.with_retry(url, || {
            let retry = |msg: String| Transient::Retry { msg, after: None };
            let mut resp = self
                .agent
                .get(url)
                .call()
                .map_err(|e| retry(e.to_string()))?;
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                return Err(Transient::Retry {
                    msg: format!("HTTP {status}"),
                    after: retry_after(&resp),
                });
            }
            if status != 200 {
                return Err(Transient::Fatal(DataError::Format {
                    url: url.into(),
                    msg: format!("HTTP {status}"),
                }));
            }
            let text = resp
                .body_mut()
                .with_config()
                .limit(MAX_BODY)
                .read_to_string()
                .map_err(|e| retry(e.to_string()))?;
            serde_json::from_str(&text).map_err(|e| {
                Transient::Fatal(DataError::Format {
                    url: url.into(),
                    msg: e.to_string(),
                })
            })
        })
    }

    /// POST a JSON body with HTTP basic authentication; returns the response body as text.
    /// Credentials are never logged or included in errors.
    pub fn post_json_basic(
        &self,
        url: &str,
        body: &str,
        user: &str,
        password: &str,
    ) -> Result<String, DataError> {
        let auth = format!("Basic {}", base64(format!("{user}:{password}").as_bytes()));
        self.with_retry(url, || {
            let retry = |msg: String| Transient::Retry { msg, after: None };
            let mut resp = self
                .agent
                .post(url)
                .header("Authorization", &auth)
                .header("Content-Type", "application/json")
                .send(body)
                .map_err(|e| retry(e.to_string()))?;
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                return Err(Transient::Retry {
                    msg: format!("HTTP {status}"),
                    after: retry_after(&resp),
                });
            }
            let text = resp
                .body_mut()
                .with_config()
                .limit(MAX_BODY)
                .read_to_string()
                .map_err(|e| retry(e.to_string()))?;
            if !(200..300).contains(&status) {
                let hint = if status == 401 || status == 403 {
                    " (check GBIF credentials)"
                } else {
                    ""
                };
                return Err(Transient::Fatal(DataError::Format {
                    url: url.into(),
                    msg: format!("HTTP {status}{hint}"),
                }));
            }
            Ok(text)
        })
    }

    /// HTTP status of a GET with basic authentication, one attempt (health checks: is an account
    /// accepted?). Credentials are never logged or included in errors.
    pub fn status_basic(&self, url: &str, user: &str, password: &str) -> Result<u16, DataError> {
        let auth = format!("Basic {}", base64(format!("{user}:{password}").as_bytes()));
        let resp = self
            .agent
            .get(url)
            .header("Authorization", &auth)
            .config()
            .http_status_as_error(false)
            .build()
            .call()
            .map_err(|e| DataError::Network {
                url: url.into(),
                msg: e.to_string(),
            })?;
        Ok(resp.status().as_u16())
    }

    /// Stream `url` to `path` (atomic: temp file then rename). Follows redirects.
    pub fn download_to_file(
        &self,
        url: &str,
        path: &std::path::Path,
        max_bytes: u64,
    ) -> Result<u64, DataError> {
        self.download_to_file_basic(url, path, max_bytes, None)
    }

    /// [`Self::download_to_file`] with optional HTTP basic authentication (user, password).
    /// Credentials are never logged or included in errors.
    pub fn download_to_file_basic(
        &self,
        url: &str,
        path: &std::path::Path,
        max_bytes: u64,
        credentials: Option<(&str, &str)>,
    ) -> Result<u64, DataError> {
        let auth = credentials.map(|(user, password)| {
            format!("Basic {}", base64(format!("{user}:{password}").as_bytes()))
        });
        self.with_retry(url, || {
            let retry = |msg: String| Transient::Retry { msg, after: None };
            let mut req = self.agent.get(url);
            if let Some(a) = &auth {
                req = req.header("Authorization", a);
            }
            let mut resp = req.call().map_err(|e| retry(e.to_string()))?;
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                return Err(Transient::Retry {
                    msg: format!("HTTP {status}"),
                    after: retry_after(&resp),
                });
            }
            if status != 200 {
                return Err(Transient::Fatal(DataError::Format {
                    url: url.into(),
                    msg: format!("HTTP {status}"),
                }));
            }
            let tmp = path.with_extension("partial");
            let mut f = std::fs::File::create(&tmp)
                .map_err(|e| Transient::Fatal(DataError::Cache(e.to_string())))?;
            let mut reader = resp.body_mut().with_config().limit(max_bytes).reader();
            let n = std::io::copy(&mut reader, &mut f).map_err(|e| retry(e.to_string()))?;
            f.sync_all()
                .map_err(|e| Transient::Fatal(DataError::Cache(e.to_string())))?;
            std::fs::rename(&tmp, path)
                .map_err(|e| Transient::Fatal(DataError::Cache(e.to_string())))?;
            Ok(n)
        })
    }

    /// Download a large file (gigabytes) in range requests of `chunk` bytes, appending to
    /// `<path>.partial` and renaming it to `path` when complete. An interrupted download resumes
    /// from the bytes already on disk (also after a restart). Each request has its own timeout,
    /// so a stalled connection is retried instead of hanging. Returns the file size.
    pub fn download_resumable(
        &self,
        url: &str,
        path: &std::path::Path,
        chunk: u64,
        cancelled: &dyn Fn() -> bool,
        progress: &dyn Fn(u64, u64),
    ) -> Result<u64, DataError> {
        use std::io::Write as _;
        let cache = |e: std::io::Error| DataError::Cache(format!("{}: {e}", path.display()));
        let tmp = path.with_extension("partial");
        // One chunk must arrive within this time; 256 MiB at 1 MB/s takes ~4.5 min.
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(15 * 60)))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .build()
            .into();
        let mut total: Option<u64> = None;
        loop {
            if cancelled() {
                return Err(DataError::Cancelled);
            }
            let have = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
            if let Some(t) = total
                && have >= t
            {
                std::fs::rename(&tmp, path).map_err(cache)?;
                return Ok(t);
            }
            let range = format!("bytes={}-{}", have, have + chunk - 1);
            let got = self.with_retry(url, || {
                let retry = |msg: String| Transient::Retry { msg, after: None };
                let mut resp = agent
                    .get(url)
                    .header("Range", &range)
                    .call()
                    .map_err(|e| retry(e.to_string()))?;
                let status = resp.status().as_u16();
                if status == 429 || status >= 500 {
                    return Err(Transient::Retry {
                        msg: format!("HTTP {status}"),
                        after: retry_after(&resp),
                    });
                }
                // 416: nothing left past `have` (the previous chunk ended the file exactly).
                if status == 416 {
                    return Ok((0, Some(have)));
                }
                if status != 206 {
                    return Err(Transient::Fatal(DataError::Format {
                        url: url.into(),
                        msg: format!("expected 206 Partial Content, got {status}"),
                    }));
                }
                let size = resp
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.rsplit('/').next())
                    .and_then(|v| v.trim().parse::<u64>().ok());
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&tmp)
                    .map_err(|e| Transient::Fatal(cache(e)))?;
                // ureq refuses a body that reaches its limit: allow the chunk exactly.
                let mut reader = resp.body_mut().with_config().limit(chunk + 1).reader();
                let n = std::io::copy(&mut reader, &mut f);
                // Keep whatever arrived (the next attempt resumes after it), then report.
                f.flush().map_err(|e| Transient::Fatal(cache(e)))?;
                f.sync_data().map_err(|e| Transient::Fatal(cache(e)))?;
                match n {
                    Ok(n) => Ok((n, size)),
                    Err(e) => Err(retry(e.to_string())),
                }
            });
            match got {
                Ok((_, size)) => {
                    total = size.or(total);
                    let have = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
                    progress(have, total.unwrap_or(0));
                    if total.is_none() {
                        return Err(DataError::Format {
                            url: url.into(),
                            msg: "no Content-Range total in the response".into(),
                        });
                    }
                }
                // A retry that exhausted itself: the bytes on disk stay for the next call.
                Err(e) => return Err(e),
            }
        }
    }

    /// Bytes [offset, offset+len) of `url`.
    pub fn get_range(&self, url: &str, offset: u64, len: usize) -> Result<Vec<u8>, DataError> {
        self.range(url, offset, len, false)
    }

    /// Bytes [offset, offset+len) of `url`, or fewer when the file ends earlier (the server's
    /// `Content-Range` total says where it ends). Used for header prefixes of small files.
    pub fn get_range_upto(&self, url: &str, offset: u64, len: usize) -> Result<Vec<u8>, DataError> {
        self.range(url, offset, len, true)
    }

    fn range(
        &self,
        url: &str,
        offset: u64,
        len: usize,
        allow_eof: bool,
    ) -> Result<Vec<u8>, DataError> {
        if len == 0 {
            return Ok(Vec::new());
        }
        self.with_retry(url, || {
            let retry = |msg: String| Transient::Retry { msg, after: None };
            let range = format!("bytes={}-{}", offset, offset + len as u64 - 1);
            let mut resp = self
                .agent
                .get(url)
                .header("Range", &range)
                .call()
                .map_err(|e| retry(e.to_string()))?;
            let status = resp.status().as_u16();
            if status == 429 || status >= 500 {
                return Err(Transient::Retry {
                    msg: format!("HTTP {status}"),
                    after: retry_after(&resp),
                });
            }
            if status != 206 {
                return Err(Transient::Fatal(DataError::Format {
                    url: url.into(),
                    msg: format!("expected 206 Partial Content, got {status}"),
                }));
            }
            let total = resp
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.rsplit('/').next())
                .and_then(|v| v.trim().parse::<u64>().ok());
            let mut buf = Vec::with_capacity(len);
            resp.body_mut()
                .with_config()
                .limit(MAX_BODY)
                .reader()
                .read_to_end(&mut buf)
                .map_err(|e| retry(e.to_string()))?;
            self.range_bytes
                .fetch_add(buf.len() as u64, std::sync::atomic::Ordering::Relaxed);
            let at_eof = total == Some(offset + buf.len() as u64);
            if buf.len() != len && !(allow_eof && at_eof && buf.len() < len) {
                return Err(retry(format!(
                    "short range read: {} of {len} bytes",
                    buf.len()
                )));
            }
            Ok(buf)
        })
    }
}

/// Remote GeoTIFF as a [`ByteSource`]: the first 512 KiB are fetched once (headers and offset
/// tables), later reads are range requests.
pub struct HttpSource {
    http: Http,
    url: String,
    prefix: OnceLock<Vec<u8>>,
}

const PREFIX: usize = 512 << 10;

impl HttpSource {
    pub fn new(http: Http, url: impl Into<String>) -> Self {
        Self {
            http,
            url: url.into(),
            prefix: OnceLock::new(),
        }
    }
}

impl ByteSource for HttpSource {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, TiffError> {
        if offset + len as u64 <= PREFIX as u64 {
            if self.prefix.get().is_none() {
                let p = self
                    .http
                    .get_range_upto(&self.url, 0, PREFIX)
                    .map_err(|e| TiffError::Io(e.to_string()))?;
                let _ = self.prefix.set(p);
            }
            if let Some(p) = self.prefix.get() {
                let s = offset as usize;
                return p
                    .get(s..s + len)
                    .map(<[u8]>::to_vec)
                    .ok_or_else(|| TiffError::Io("prefix bounds".into()));
            }
        }
        self.http
            .get_range(&self.url, offset, len)
            .map_err(|e| TiffError::Io(e.to_string()))
    }
}

/// Standard base64 (RFC 4648) for the Authorization header.
fn base64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_rfc4648_vectors() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
