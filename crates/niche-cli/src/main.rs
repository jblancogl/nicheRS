//! `niche` — NicheRS command-line interface.
//!
//! One executable for every mode (twelve-factor XII): `serve` (local/server UI + API), `run`
//! (headless batch from a job spec), `demo`, admin commands (`doctor`, `cache`), and the SWD
//! commands used to compare the kernel with `maxent.jar`.

mod app_cmd;
mod boundaries_cmd;

use niche_cli::swd;

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use niche_maxent::{FeatureClasses, MaxentModel, MaxentSettings, OutputFormat, train};

#[derive(Parser)]
#[command(
    name = "niche",
    version,
    about = "NicheRS species distribution modelling"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the web application and API (default: http://127.0.0.1:8080).
    Serve(app_cmd::ServeArgs),
    /// Run a job spec (JSON, same schema as POST /api/v1/jobs) headlessly and print a summary.
    Run(app_cmd::RunArgs),
    /// Run the bundled SYNTHETIC demonstration job headlessly (offline).
    Demo(app_cmd::Common),
    /// Check configuration, storage, and access to data sources.
    Doctor(app_cmd::Common),
    /// Hosted-mode administration: organisations, invitation links, accounts, usage.
    Admin(app_cmd::AdminArgs),
    /// Inspect or clean the download cache.
    Cache(app_cmd::CacheArgs),
    /// Regional GBIF mirror (NICHE_GBIF_MIRROR): show its state, or import an archive by hand.
    Mirror(app_cmd::MirrorArgs),
    /// Administrative boundary packs for study areas: build, install, list.
    Boundaries(boundaries_cmd::BoundariesArgs),
    /// Export cleaned presences + background of a job spec as SWD CSV (verification with maxent.jar).
    ExportSwd(app_cmd::ExportArgs),
    /// Train a MaxEnt-compatible model from SWD CSV files (species,x,y,var1,...).
    TrainSwd(TrainSwd),
    /// Predict a `.lambdas` model onto SWD-format points.
    PredictSwd(PredictSwd),
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Raw,
    Cloglog,
    Logistic,
}

impl From<Format> for OutputFormat {
    fn from(f: Format) -> Self {
        match f {
            Format::Raw => OutputFormat::Raw,
            Format::Cloglog => OutputFormat::Cloglog,
            Format::Logistic => OutputFormat::Logistic,
        }
    }
}

#[derive(Args)]
struct TrainSwd {
    /// Presence records (SWD CSV).
    #[arg(long)]
    samples: PathBuf,
    /// Background points (SWD CSV).
    #[arg(long)]
    background: PathBuf,
    /// Output directory.
    #[arg(long)]
    out: PathBuf,
    /// Variables to treat as categorical.
    #[arg(long, value_delimiter = ',')]
    categorical: Vec<String>,
    #[arg(long, default_value_t = 1.0)]
    beta_multiplier: f64,
    /// Comma-separated feature classes among l,q,p,t,h (default: MaxEnt defaults l,q,p,h).
    #[arg(long)]
    features: Option<String>,
    /// Disable automatic feature-class selection by sample size.
    #[arg(long)]
    no_auto_features: bool,
    #[arg(long, default_value_t = 500)]
    max_iterations: usize,
    #[arg(long, default_value_t = 1e-5)]
    convergence_threshold: f64,
    #[arg(long, value_enum, default_value_t = Format::Cloglog)]
    format: Format,
    /// NODATA marker in SWD files.
    #[arg(long, default_value_t = -9999.0)]
    nodata: f64,
}

#[derive(Args)]
struct PredictSwd {
    #[arg(long)]
    model: PathBuf,
    #[arg(long)]
    points: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, value_enum, default_value_t = Format::Cloglog)]
    format: Format,
    /// Disable clamping to the training range.
    #[arg(long)]
    no_clamp: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Serve(a) => app_cmd::serve(a),
        Command::Run(a) => app_cmd::run(a),
        Command::Demo(a) => app_cmd::demo(a),
        Command::Doctor(a) => app_cmd::doctor(a),
        Command::Cache(a) => app_cmd::cache(a),
        Command::Mirror(a) => app_cmd::mirror(a),
        Command::Boundaries(a) => boundaries_cmd::run(a),
        Command::Admin(a) => app_cmd::admin(a),
        Command::ExportSwd(a) => app_cmd::export_swd(a),
        Command::TrainSwd(a) => train_swd(a),
        Command::PredictSwd(a) => predict_swd(a),
    }
}

fn parse_features(s: &str) -> Result<FeatureClasses> {
    let mut fc = FeatureClasses {
        linear: false,
        quadratic: false,
        product: false,
        threshold: false,
        hinge: false,
    };
    for c in s.split(',').map(str::trim) {
        match c {
            "l" => fc.linear = true,
            "q" => fc.quadratic = true,
            "p" => fc.product = true,
            "t" => fc.threshold = true,
            "h" => fc.hinge = true,
            other => anyhow::bail!("unknown feature class {other:?} (expected l,q,p,t,h)"),
        }
    }
    Ok(fc)
}

fn train_swd(a: TrainSwd) -> Result<()> {
    let t0 = Instant::now();
    let data = swd::load_training(&a.samples, &a.background, &a.categorical, a.nodata)?;
    let t_load = t0.elapsed();
    let settings = MaxentSettings {
        features: match &a.features {
            Some(s) => parse_features(s)?,
            None => FeatureClasses::default(),
        },
        auto_features: !a.no_auto_features,
        beta_multiplier: a.beta_multiplier,
        max_iterations: a.max_iterations,
        convergence_threshold: a.convergence_threshold,
        ..MaxentSettings::default()
    };
    let t1 = Instant::now();
    let (model, report) = train(&data.swd, &settings, &()).context("training failed")?;
    let t_train = t1.elapsed();

    std::fs::create_dir_all(&a.out)?;
    std::fs::write(
        a.out.join(format!("{}.lambdas", data.species)),
        model.to_lambdas(),
    )?;

    let t2 = Instant::now();
    let names: Vec<String> = data.swd.vars.iter().map(|v| v.name.clone()).collect();
    let compiled = model.compile(&names, true).map_err(anyhow::Error::msg)?;
    let fmt: OutputFormat = a.format.into();
    let mut csv = String::from("x,y,prediction\n");
    for (i, (x, y)) in data.bg_coords.iter().enumerate() {
        let row: Vec<f64> = data.swd.background.iter().map(|c| c[i]).collect();
        csv.push_str(&format!("{x},{y},{}\n", compiled.predict(&row, fmt)));
    }
    std::fs::write(
        a.out
            .join(format!("{}_backgroundPredictions.csv", data.species)),
        csv,
    )?;
    let t_predict = t2.elapsed();

    let summary = serde_json::json!({
        "species": data.species,
        "samples": data.swd.samples[0].len(),
        "samples_dropped_missing": data.samples_dropped,
        "background": data.swd.background[0].len(),
        "background_dropped_missing": data.background_dropped,
        "background_after_adding_samples": report.num_background,
        "added_samples": report.num_added_samples,
        "iterations": report.iterations,
        "loss": report.loss,
        "regularized_training_gain": report.regularized_gain,
        "unregularized_training_gain": report.unregularized_gain,
        "entropy": model.entropy,
        "percent_contribution": report.percent_contribution,
        "timing_ms": {
            "load": t_load.as_secs_f64() * 1e3,
            "train": t_train.as_secs_f64() * 1e3,
            "predict_background": t_predict.as_secs_f64() * 1e3,
        },
        "engine": format!("niche-maxent {}", env!("CARGO_PKG_VERSION")),
    });
    std::fs::write(
        a.out.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn predict_swd(a: PredictSwd) -> Result<()> {
    let text = std::fs::read_to_string(&a.model)
        .with_context(|| format!("reading {}", a.model.display()))?;
    let model = MaxentModel::from_lambdas(&text)?;
    let pts = swd::load_points(&a.points)?;
    let compiled = model
        .compile(&pts.vars, !a.no_clamp)
        .map_err(anyhow::Error::msg)?;
    let fmt: OutputFormat = a.format.into();
    let mut csv = String::from("x,y,prediction\n");
    for (row, (x, y)) in pts.rows.iter().zip(&pts.coords) {
        csv.push_str(&format!("{x},{y},{}\n", compiled.predict(row, fmt)));
    }
    std::fs::write(&a.out, csv)?;
    Ok(())
}
