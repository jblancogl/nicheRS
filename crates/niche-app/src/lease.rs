//! Leader lease (ADR-0115): during a deployment two server processes share the data directory
//! for a while — the new one already answers requests while the old one finishes. Only the
//! holder of this lease runs the background work (studies, data preparation, the regional GBIF
//! mirror, retention, database copies); the other only serves the API.
//!
//! The lease is an exclusive lock on `<data_dir>/leader.lock` (`flock` on Unix): the kernel
//! releases it when the holder exits, also when it is killed, so a crashed leader never blocks
//! its successor.

use std::fs::{File, TryLockError};
use std::path::Path;

use crate::AppError;

/// Held while this process is the leader; dropping it releases the lease.
#[derive(Debug)]
pub struct Lease {
    _file: File,
}

impl Lease {
    /// Take the lease if no other process holds it (never blocks).
    pub fn try_acquire(data_dir: &Path) -> Result<Option<Self>, AppError> {
        let path = data_dir.join("leader.lock");
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| AppError::Storage(format!("{}: {e}", path.display())))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(e)) => {
                Err(AppError::Storage(format!("{}: {e}", path.display())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_holder_at_a_time_and_released_on_drop() {
        let dir = std::env::temp_dir().join(format!("niche-lease-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let a = Lease::try_acquire(&dir).expect("io").expect("first holder");
        assert!(
            Lease::try_acquire(&dir).expect("io").is_none(),
            "second process waits"
        );
        drop(a);
        assert!(
            Lease::try_acquire(&dir).expect("io").is_some(),
            "released on drop"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
