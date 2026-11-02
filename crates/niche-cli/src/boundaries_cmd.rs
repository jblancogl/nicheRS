//! `niche boundaries`: build, install and inspect administrative boundary packs (`.nrsb`).

use std::io::BufRead;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use niche_geo::boundaries::{BoundaryPack, Source, UnitIn, build};

use crate::app_cmd::Common;

#[derive(Args)]
pub struct BoundariesArgs {
    #[command(flatten)]
    pub common: Common,
    #[command(subcommand)]
    pub action: BoundariesAction,
}

#[derive(Subcommand)]
pub enum BoundariesAction {
    /// Encode the NDJSON written by scripts/build_boundaries.py into a pack.
    Build {
        /// NDJSON units (one administrative unit per line).
        #[arg(long)]
        input: PathBuf,
        /// Sources JSON written next to it (`*.sources.json`).
        #[arg(long)]
        sources: PathBuf,
        /// Pack name recorded with every study using it, e.g. fr-es-2026.1.
        #[arg(long)]
        pack: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Check a pack and install it in <data dir>/boundaries/ (replaces a pack of the same name).
    Install { file: PathBuf },
    /// Make <data dir>/boundaries/ hold the declared packs: download those missing (checked
    /// against their sha256) and, with --prune, remove the others. Run before the server starts
    /// (Kubernetes init container); a pack already present is never downloaded again.
    Sync {
        /// Packs as `<url>#sha256=<hex>`, comma-separated in the environment variable.
        #[arg(env = "NICHE_BOUNDARY_PACKS", value_delimiter = ',', required = true)]
        packs: Vec<String>,
        /// Remove installed packs that are not declared.
        #[arg(long)]
        prune: bool,
        /// Fail when a pack cannot be installed (by default the error is reported and the
        /// packs already installed are kept, so the server still starts).
        #[arg(long)]
        strict: bool,
        /// Basic-auth user for the pack URLs (e.g. a GitLab deploy token).
        #[arg(long, env = "NICHE_PACKS_USERNAME", hide_env_values = true)]
        username: Option<String>,
        #[arg(long, env = "NICHE_PACKS_PASSWORD", hide_env_values = true)]
        password: Option<String>,
    },
    /// List installed packs.
    Info,
    /// Search installed packs by name (as the study-area search does).
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

/// Directory the server loads packs from.
pub fn packs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("boundaries")
}

fn read_sources(path: &Path) -> Result<Vec<Source>> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let mut out = Vec::new();
    for s in v["sources"].as_array().context("sources[] missing")? {
        let country = if s["name"].as_str().unwrap_or("").contains("España") {
            "ES"
        } else {
            "FR"
        };
        out.push(Source {
            country: s
                .get("country")
                .and_then(|c| c.as_str())
                .unwrap_or(country)
                .to_string(),
            name: s["name"].as_str().unwrap_or_default().to_string(),
            licence: s["licence"].as_str().unwrap_or_default().to_string(),
            attribution: s["attribution"].as_str().unwrap_or_default().to_string(),
            url: s["url"].as_str().unwrap_or_default().to_string(),
            edition: s["edition"].as_str().unwrap_or_default().to_string(),
            sha256: s["sha256"].as_str().unwrap_or_default().to_string(),
        });
    }
    Ok(out)
}

fn summary(p: &BoundaryPack, bytes: usize) {
    let mut by: std::collections::BTreeMap<(String, String), usize> = Default::default();
    for x in p.places() {
        *by.entry((x.country.clone(), format!("{:?}", x.level).to_lowercase()))
            .or_default() += 1;
    }
    println!(
        "pack {} — {:.1} MB, {} units",
        p.pack(),
        bytes as f64 / 1e6,
        p.places().len()
    );
    for ((c, l), n) in by {
        println!("  {c} {l:<17} {n}");
    }
    for s in p.sources() {
        println!("  source: {} — {} ({})", s.name, s.licence, s.edition);
    }
}

pub fn run(a: BoundariesArgs) -> Result<()> {
    match a.action {
        BoundariesAction::Build {
            input,
            sources,
            pack,
            out,
        } => {
            let f = std::io::BufReader::new(
                std::fs::File::open(&input).with_context(|| input.display().to_string())?,
            );
            let mut units = Vec::new();
            for (i, line) in f.lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                units.push(
                    serde_json::from_str::<UnitIn>(&line)
                        .with_context(|| format!("{}:{}", input.display(), i + 1))?,
                );
            }
            let bytes = build(
                units,
                read_sources(&sources)?,
                &pack,
                &jiff::Timestamp::now().to_string(),
            )?;
            std::fs::write(&out, &bytes)?;
            summary(&BoundaryPack::open(&bytes)?, bytes.len());
            println!("written {}", out.display());
        }
        BoundariesAction::Install { file } => {
            let bytes = std::fs::read(&file).with_context(|| file.display().to_string())?;
            let p = BoundaryPack::open(&bytes).context("not a valid boundary pack")?;
            let engine_dir = crate::app_cmd::data_dir(&a.common)?;
            let dir = packs_dir(&engine_dir);
            std::fs::create_dir_all(&dir)?;
            let dest = dir.join(format!("{}.nrsb", p.pack()));
            let tmp = dest.with_extension("nrsb.tmp");
            std::fs::write(&tmp, &bytes)?;
            std::fs::rename(&tmp, &dest)?;
            summary(&p, bytes.len());
            println!(
                "installed {} (restart the server to load it)",
                dest.display()
            );
        }
        BoundariesAction::Sync {
            packs,
            prune,
            strict,
            username,
            password,
        } => {
            let dir = packs_dir(&crate::app_cmd::data_dir(&a.common)?);
            let credentials = username.as_deref().zip(password.as_deref());
            let failed = sync(&dir, &packs, prune, credentials)?;
            if failed > 0 && strict {
                bail!("{failed} boundary pack(s) could not be installed");
            }
        }
        BoundariesAction::Search { query, limit } => {
            for p in installed(&a.common)? {
                for x in p.search(&query, None, limit) {
                    let lineage: Vec<&str> = p
                        .lineage(&x.id)
                        .iter()
                        .skip(1)
                        .map(|a| a.name.as_str())
                        .collect();
                    println!(
                        "{:<12} {:<17} {:<40} {:>10.1} km²  {}",
                        x.id,
                        format!("{:?}", x.level).to_lowercase(),
                        x.name,
                        x.area_km2,
                        lineage.join(" › ")
                    );
                }
            }
        }
        BoundariesAction::Info => {
            let dir = packs_dir(&crate::app_cmd::data_dir(&a.common)?);
            let mut any = false;
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                if e.path().extension().is_some_and(|x| x == "nrsb") {
                    let bytes = std::fs::read(e.path())?;
                    summary(&BoundaryPack::open(&bytes)?, bytes.len());
                    any = true;
                }
            }
            if !any {
                bail!("no boundary pack installed in {}", dir.display());
            }
        }
    }
    Ok(())
}

/// Largest pack accepted from a URL.
const MAX_PACK_BYTES: u64 = 1 << 30;

/// `<url>#sha256=<hex>` → (url, lowercase hex).
fn parse_pack_spec(spec: &str) -> Result<(&str, String)> {
    let (url, sha) = spec
        .trim()
        .split_once("#sha256=")
        .with_context(|| format!("{spec:?}: expected <url>#sha256=<hex>"))?;
    let sha = sha.trim().to_ascii_lowercase();
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("{spec:?}: sha256 must be 64 hexadecimal characters");
    }
    Ok((url.trim(), sha))
}

/// Installed `.nrsb` files with their sha256.
fn installed_files(dir: &Path) -> Vec<(PathBuf, String)> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "nrsb"))
        .filter_map(|p| {
            let bytes = std::fs::read(&p).ok()?;
            Some((p, niche_app::artifacts::sha256(&bytes)))
        })
        .collect()
}

/// Install the declared packs in `dir`; returns how many could not be installed.
fn sync(
    dir: &Path,
    specs: &[String],
    prune: bool,
    credentials: Option<(&str, &str)>,
) -> Result<usize> {
    let wanted = specs
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| parse_pack_spec(s))
        .collect::<Result<Vec<_>>>()?;
    std::fs::create_dir_all(dir)?;
    let mut have = installed_files(dir);
    let http = niche_data::http::Http::new();
    let mut keep: Vec<PathBuf> = Vec::new();
    let mut failed = 0;
    for (url, sha) in &wanted {
        if let Some((p, _)) = have.iter().find(|(_, h)| h == sha) {
            println!("up to date: {}", p.display());
            keep.push(p.clone());
            continue;
        }
        match install_from_url(&http, dir, url, sha, credentials) {
            Ok(p) => {
                keep.push(p.clone());
                have.push((p, sha.clone()));
            }
            Err(e) => {
                eprintln!("boundary pack not installed: {url}: {e:#}");
                failed += 1;
            }
        }
    }
    // Without every declared pack in place nothing is pruned: a failed download never leaves
    // the server with fewer packs than before.
    if prune && failed == 0 {
        for (p, _) in &have {
            if !keep.contains(p) {
                std::fs::remove_file(p).with_context(|| p.display().to_string())?;
                println!("removed {}", p.display());
            }
        }
    }
    Ok(failed)
}

fn install_from_url(
    http: &niche_data::http::Http,
    dir: &Path,
    url: &str,
    sha: &str,
    credentials: Option<(&str, &str)>,
) -> Result<PathBuf> {
    let tmp = dir.join(".sync-download");
    let _ = std::fs::remove_file(&tmp);
    http.download_to_file_basic(url, &tmp, MAX_PACK_BYTES, credentials)?;
    let bytes = std::fs::read(&tmp)?;
    let _ = std::fs::remove_file(&tmp);
    let got = niche_app::artifacts::sha256(&bytes);
    if got != sha {
        bail!("sha256 mismatch (expected {sha}, got {got})");
    }
    let p = BoundaryPack::open(&bytes).context("not a valid boundary pack")?;
    let dest = dir.join(format!("{}.nrsb", p.pack()));
    let part = dest.with_extension("nrsb.tmp");
    std::fs::write(&part, &bytes)?;
    std::fs::rename(&part, &dest)?;
    summary(&p, bytes.len());
    println!("installed {}", dest.display());
    Ok(dest)
}

fn installed(c: &Common) -> Result<Vec<BoundaryPack>> {
    let dir = packs_dir(&crate::app_cmd::data_dir(c)?);
    let mut out = Vec::new();
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        if e.path().extension().is_some_and(|x| x == "nrsb") {
            out.push(BoundaryPack::open(&std::fs::read(e.path())?)?);
        }
    }
    if out.is_empty() {
        bail!("no boundary pack installed in {}", dir.display());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// Serve `body` to `n` HTTP requests on a local port; returns the base URL.
    fn serve(body: Vec<u8>, n: usize) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        std::thread::spawn(move || {
            for stream in listener.incoming().take(n) {
                let Ok(mut s) = stream else { continue };
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = s.write_all(&body);
            }
        });
        format!("http://{addr}")
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("niche-sync-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn pack_specs_need_a_sha256() {
        let sha = "AB".repeat(32);
        let spec = format!(" https://x/p.nrsb#sha256={sha} ");
        let (url, h) = parse_pack_spec(&spec).expect("ok");
        assert_eq!(url, "https://x/p.nrsb");
        assert_eq!(h, "ab".repeat(32));
        assert!(parse_pack_spec("https://x/p.nrsb").is_err());
        assert!(parse_pack_spec("https://x/p.nrsb#sha256=abc").is_err());
    }

    #[test]
    fn declared_packs_are_downloaded_once_checked_and_pruned() {
        let dir = tmp("ok");
        let bytes = niche_geo::boundaries::build(vec![], vec![], "fr-test-1", "x").expect("pack");
        let sha = niche_app::artifacts::sha256(&bytes);
        // One request only: the second sync must not download again.
        let url = serve(bytes, 1);
        let spec = vec![format!("{url}/fr-test-1.nrsb#sha256={sha}")];
        std::fs::create_dir_all(&dir).expect("dir");
        let old = dir.join("fr-test-0.nrsb");
        std::fs::write(&old, b"older edition").expect("old");

        assert_eq!(sync(&dir, &spec, true, None).expect("sync"), 0);
        assert!(dir.join("fr-test-1.nrsb").exists());
        assert!(!old.exists(), "undeclared pack pruned");
        assert_eq!(sync(&dir, &spec, true, None).expect("again"), 0);

        // A wrong checksum installs nothing and prunes nothing.
        let bad = vec![
            spec[0].clone(),
            format!(
                "{}/other.nrsb#sha256={}",
                serve(b"junk".to_vec(), 7),
                "0".repeat(64)
            ),
        ];
        std::fs::write(&old, b"older edition").expect("old");
        assert_eq!(sync(&dir, &bad, true, None).expect("sync"), 1);
        assert!(old.exists());
        assert_eq!(installed_files(&dir).len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
