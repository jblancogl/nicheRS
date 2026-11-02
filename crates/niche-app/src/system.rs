//! Instance state for administrators (storage, database copies) and the daily database copy.
//!
//! The volume itself is backed up by the deployment (e.g. Longhorn); these copies of the SQLite
//! database — accounts, organisations, the state and log of every study — are a second, local
//! line of defence against a bad migration or a mistake, restorable without the platform.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::dto::{DbBackups, DiskSpace};

/// Daily database copies kept.
pub const DB_BACKUPS_KEEP: usize = 7;
const DB_BACKUP_EVERY: Duration = Duration::from_secs(23 * 3600);

pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

/// Bytes under `p` (files only, symbolic links not followed, unreadable entries skipped).
pub fn dir_size(p: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(p) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    std::fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| dir_size(&e.path())).sum())
        .unwrap_or(0)
}

/// Size and free space of the file system holding `p`.
#[cfg(unix)]
pub fn disk_space(p: &Path) -> Option<DiskSpace> {
    let s = nix::sys::statvfs::statvfs(p).ok()?;
    let unit = s.fragment_size() as u64;
    Some(DiskSpace {
        total_bytes: (s.blocks() as u64).saturating_mul(unit),
        available_bytes: (s.blocks_available() as u64).saturating_mul(unit),
    })
}

#[cfg(not(unix))]
pub fn disk_space(_p: &Path) -> Option<DiskSpace> {
    None
}

/// Database copies, newest first, with their modification times.
fn copies(dir: &Path) -> Vec<(PathBuf, SystemTime)> {
    let mut v: Vec<(PathBuf, SystemTime)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("niche-") && n.ends_with(".db"))
                })
                .filter_map(|p| {
                    let t = std::fs::metadata(&p).and_then(|m| m.modified()).ok()?;
                    Some((p, t))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    v
}

pub fn db_backups(dir: &Path) -> DbBackups {
    let c = copies(dir);
    DbBackups {
        last_at: c
            .first()
            .and_then(|(_, t)| jiff::Timestamp::try_from(*t).ok())
            .map(|t| t.to_string()),
        count: c.len(),
        keep: DB_BACKUPS_KEEP,
    }
}

/// No copy yet, or the newest is older than a day.
pub fn backup_due(dir: &Path) -> bool {
    copies(dir)
        .first()
        .and_then(|(_, t)| t.elapsed().ok())
        .is_none_or(|age| age >= DB_BACKUP_EVERY)
}

/// Keep the newest `keep` copies.
pub fn prune(dir: &Path, keep: usize) {
    for (p, _) in copies(dir).into_iter().skip(keep) {
        if let Err(e) = std::fs::remove_file(&p) {
            tracing::warn!(file = %p.display(), error = %e, "removing old database copy");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_are_listed_newest_first_and_pruned() {
        let dir = std::env::temp_dir().join(format!("niche-backups-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        assert!(backup_due(&dir));
        assert_eq!(db_backups(&dir).count, 0);
        for i in 0..3 {
            std::fs::write(dir.join(format!("niche-2026101{i}.db")), b"x").expect("write");
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::write(dir.join("other.txt"), b"x").expect("write");
        assert!(!backup_due(&dir));
        assert_eq!(db_backups(&dir).count, 3);
        prune(&dir, 2);
        let left: Vec<String> = copies(&dir)
            .iter()
            .map(|(p, _)| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()
            })
            .collect();
        assert_eq!(left, ["niche-20261012.db", "niche-20261011.db"]);
        assert!(
            dir.join("other.txt").exists(),
            "only database copies are pruned"
        );
        assert!(dir_size(&dir) >= 3);
        let _ = std::fs::remove_dir_all(dir);
    }
}
