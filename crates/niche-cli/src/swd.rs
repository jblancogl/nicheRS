//! Minimal SWD CSV reader (MaxEnt "samples with data" format: species,x,y,var1,...).
//!
//! Follows MaxEnt 3.4.4 semantics: background rows with any NODATA value are skipped
//! (`GridSetFromFile`); presence rows with missing data are dropped (`allowpartialdata=false`);
//! duplicate presence coordinates are removed (`removeduplicates=true`, SWD case).

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use niche_maxent::{SwdData, VarKind, Variable};

pub struct Training {
    pub species: String,
    pub swd: SwdData,
    pub bg_coords: Vec<(String, String)>,
    pub samples_dropped: usize,
    pub background_dropped: usize,
}

struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn read_table(path: &Path) -> Result<Table> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header: Vec<String> = lines
        .next()
        .with_context(|| format!("{} is empty", path.display()))?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    if header.len() < 4 {
        bail!(
            "{}: expected species,x,y and at least one variable column",
            path.display()
        );
    }
    let rows = lines
        .map(|l| {
            l.split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (i, r) in rows.iter().enumerate() {
        if r.len() != header.len() {
            bail!(
                "{}: row {} has {} fields, expected {}",
                path.display(),
                i + 2,
                r.len(),
                header.len()
            );
        }
    }
    Ok(Table { header, rows })
}

fn parse(s: &str, path: &Path) -> Result<f64> {
    s.parse::<f64>()
        .with_context(|| format!("{}: invalid number {s:?}", path.display()))
}

pub fn load_training(
    samples: &Path,
    background: &Path,
    categorical: &[String],
    nodata: f64,
) -> Result<Training> {
    let st = read_table(samples)?;
    let bt = read_table(background)?;
    let var_names: Vec<String> = bt.header[3..].to_vec();
    let s_idx: Vec<usize> = var_names
        .iter()
        .map(|v| {
            st.header
                .iter()
                .position(|h| h == v)
                .with_context(|| format!("samples file missing variable {v}"))
        })
        .collect::<Result<_>>()?;
    let vars: Vec<Variable> = var_names
        .iter()
        .map(|n| Variable {
            name: n.clone(),
            kind: if categorical.contains(n) {
                VarKind::Categorical
            } else {
                VarKind::Continuous
            },
        })
        .collect();

    let nv = vars.len();
    let mut bg: Vec<Vec<f64>> = vec![Vec::new(); nv];
    let mut bg_coords = Vec::new();
    let mut background_dropped = 0;
    for r in &bt.rows {
        let vals: Vec<f64> = r[3..]
            .iter()
            .map(|s| parse(s, background))
            .collect::<Result<_>>()?;
        if vals.contains(&nodata) {
            background_dropped += 1;
            continue;
        }
        for (c, v) in bg.iter_mut().zip(vals) {
            c.push(v);
        }
        bg_coords.push((r[1].clone(), r[2].clone()));
    }

    let species = st
        .rows
        .first()
        .map(|r| r[0].clone())
        .context("no presence rows")?;
    let mut sm: Vec<Vec<f64>> = vec![Vec::new(); nv];
    let mut seen = HashSet::new();
    let mut samples_dropped = 0;
    for r in &st.rows {
        if r[0] != species {
            bail!(
                "multiple species in samples file is not supported yet ({} and {})",
                species,
                r[0]
            );
        }
        if !seen.insert((r[1].clone(), r[2].clone())) {
            samples_dropped += 1;
            continue;
        }
        let vals: Vec<f64> = s_idx
            .iter()
            .map(|&i| parse(&r[i], samples))
            .collect::<Result<_>>()?;
        if vals.contains(&nodata) {
            samples_dropped += 1;
            continue;
        }
        for (c, v) in sm.iter_mut().zip(vals) {
            c.push(v);
        }
    }
    Ok(Training {
        species,
        swd: SwdData {
            vars,
            background: bg,
            samples: sm,
        },
        bg_coords,
        samples_dropped,
        background_dropped,
    })
}

pub struct Points {
    pub vars: Vec<String>,
    pub rows: Vec<Vec<f64>>,
    pub coords: Vec<(String, String)>,
}

pub fn load_points(path: &Path) -> Result<Points> {
    let t = read_table(path)?;
    let vars = t.header[3..].to_vec();
    let mut rows = Vec::new();
    let mut coords = Vec::new();
    for r in &t.rows {
        rows.push(
            r[3..]
                .iter()
                .map(|s| parse(s, path))
                .collect::<Result<Vec<_>>>()?,
        );
        coords.push((r[1].clone(), r[2].clone()));
    }
    Ok(Points { vars, rows, coords })
}
