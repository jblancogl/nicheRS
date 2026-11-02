#!/usr/bin/env python3
"""Golden comparison: NicheRS MaxEnt kernel vs maxent.jar 3.4.4 on identical SWD inputs.

Stdlib only. Runs every case with both engines, compares models and predictions against
tolerances fixed BEFORE looking at results (see TOLERANCES and docs/benchmarks/methodology.md),
measures wall time and peak RSS with /usr/bin/time, and writes:
  bench-out/compare/results.json
  docs/benchmarks/phase3-results.md

Usage: python3 -I scripts/compare_maxent.py [--reps N] [case ...]
"""
import argparse
import json
import math
import os
import platform
import re
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
JAR = ROOT / "reference/vendor/jar/maxent.jar"
NICHE = ROOT / "target/release/niche"
FIX = ROOT / "tests/fixtures/synthetic"
OUT = ROOT / "bench-out/compare"

# Fixed before interpreting any result. Java writes predictions with ~6 significant digits
# and maxentResults.csv with 4 decimals, which bounds what can be compared.
TOLERANCES = {
    "feature_set": "identical names and order",
    "lambda_abs": 1e-6,
    "cloglog_abs": 1e-5,
    "raw_rel": 1e-4,
    "gain_abs": 1e-4,
    "iterations": "equal",
}

WARM_COPIES = 5

# name: (dataset, java args, niche args)
CASES = {
    "s01_default":        ("s01_small", [], []),
    "s02_default":        ("s02_medium", [], []),
    "s02_beta0.5":        ("s02_medium", ["betamultiplier=0.5"], ["--beta-multiplier", "0.5"]),
    "s02_beta2":          ("s02_medium", ["betamultiplier=2"], ["--beta-multiplier", "2"]),
    "s03_default":        ("s03_large", [], []),
    "s03_lq_noauto":      ("s03_large", ["product=false", "hinge=false", "autofeature=false"],
                           ["--features", "l,q", "--no-auto-features"]),
    "s03_lqpth_noauto":   ("s03_large", ["threshold=true", "autofeature=false"],
                           ["--features", "l,q,p,t,h", "--no-auto-features"]),
    "s03_hinge_only":     ("s03_large", ["linear=false", "quadratic=false", "product=false", "autofeature=false"],
                           ["--features", "h", "--no-auto-features"]),
    "s04_categorical":    ("s04_categorical", ["togglelayertype=landcover"], ["--categorical", "landcover"]),
    "s05_missing":        ("s05_missing", [], []),
    "s06_bigbg":          ("s06_bigbg", [], []),
    # Real species (GBIF CC0/CC-BY occurrences, CHELSA v2.1 predictors), exported with
    # `niche export-swd tests/real-specs/<name>.json --out bench-out/real/<name>` (gate G3).
    "real_bretagne_lacerta":       ("real/bretagne_lacerta", [], []),
    "real_occitanie_quercus_ilex": ("real/occitanie_quercus_ilex", [], []),
    "real_auvergne_salamandra":    ("real/auvergne_salamandra", [], []),
    "real_normandie_erinaceus":    ("real/normandie_erinaceus", [], []),
    "real_nordest_rana_dalmatina": ("real/nordest_rana_dalmatina", [], []),
}


def timed(cmd):
    """Run cmd under /usr/bin/time -v; return (seconds, peak_rss_mb, stdout)."""
    p = subprocess.run(["/usr/bin/time", "-v"] + cmd, capture_output=True, text=True)
    if p.returncode != 0:
        raise RuntimeError(f"command failed ({p.returncode}): {' '.join(cmd)}\n{p.stderr[-2000:]}")
    wall = re.search(r"Elapsed \(wall clock\) time .*: (?:(\d+):)?(\d+):([\d.]+)", p.stderr)
    h = int(wall.group(1) or 0)
    secs = h * 3600 + int(wall.group(2)) * 60 + float(wall.group(3))
    rss_kb = int(re.search(r"Maximum resident set size \(kbytes\): (\d+)", p.stderr).group(1))
    return secs, rss_kb / 1024.0, p.stdout


def read_lambdas(path):
    feats, meta = [], {}
    for line in Path(path).read_text().splitlines():
        parts = [s.strip() for s in line.split(",")]
        if parts[0] in ("linearPredictorNormalizer", "densityNormalizer", "numBackgroundPoints", "entropy"):
            meta[parts[0]] = float(parts[1])
        else:
            feats.append((parts[0], float(parts[1]), [float(x) for x in parts[2:]]))
    return feats, meta


def read_csv(path):
    lines = Path(path).read_text().splitlines()
    header = lines[0].split(",")
    return header, [l.split(",") for l in lines[1:]]


def java_opt_times(log):
    """Optimizer wall times printed by Sequential.run ("N: time = 1,234 loss = ..."), seconds.
    The decimal separator follows the JVM locale."""
    return [float(m.group(1).replace(",", ".")) for m in
            re.finditer(r"^\d+: time = ([\d.,]+) loss", Path(log).read_text(), re.M)]


def warm_java(name, d, jargs, jcmd):
    """Train WARM_COPIES copies of the species in one JVM; median optimizer time of the last 3,
    i.e. after JIT warm-up (methodology: warm measurement)."""
    wdir = OUT / name / "java_warm"
    wdir.mkdir(parents=True, exist_ok=True)
    lines = (d / "samples.csv").read_text().splitlines()
    rows = [lines[0]]
    for k in range(WARM_COPIES):
        rows += [f"sp{k}," + l.split(",", 1)[1] for l in lines[1:]]
    (wdir / "samples.csv").write_text("\n".join(rows) + "\n")
    cmd = [c for c in jcmd if not c.startswith(("samplesfile=", "outputdirectory="))]
    cmd += [f"samplesfile={wdir/'samples.csv'}", f"outputdirectory={wdir}"]
    subprocess.run(cmd, check=True, capture_output=True)
    times = java_opt_times(wdir / "maxent.log")
    return statistics.median(times[-3:])


def run_case(name, reps):
    ds, jargs, nargs = CASES[name]
    d = (ROOT / "bench-out" / ds) if ds.startswith("real/") else FIX / ds
    sp = (d / "samples.csv").read_text().splitlines()[1].split(",")[0]
    jout, rout = OUT / name / "java", OUT / name / "rust"
    jout.mkdir(parents=True, exist_ok=True)
    rout.mkdir(parents=True, exist_ok=True)
    jcmd = ["java", "-Djava.awt.headless=true", "-Xmx4g", "-jar", str(JAR),
            f"samplesfile={d/'samples.csv'}", f"environmentallayers={d/'background.csv'}",
            f"outputdirectory={jout}", "autorun", "visible=false", "warnings=false",
            "askoverwrite=false", "writebackgroundpredictions=true", "pictures=false",
            "responsecurves=false", "jackknife=false", "outputformat=cloglog", "threads=1",
            "writeclampgrid=false", "writemess=false"] + jargs
    rcmd = [str(NICHE), "train-swd", f"--samples={d/'samples.csv'}", f"--background={d/'background.csv'}",
            f"--out={rout}", "--format=cloglog"] + nargs
    jt, rt = [], []
    for _ in range(reps):
        jt.append(timed(jcmd)[:2])
        rt.append(timed(rcmd)[:2])

    java_opt_cold = java_opt_times(jout / "maxent.log")[-1]
    java_opt_warm = warm_java(name, d, jargs, jcmd)

    jf, jm = read_lambdas(jout / f"{sp}.lambdas")
    rf, rm = read_lambdas(rout / f"{sp}.lambdas")
    same_set = [f[0] for f in jf] == [f[0] for f in rf]
    lam_diff = max((abs(a[1] - b[1]) for a, b in zip(jf, rf)), default=0.0) if same_set else math.inf
    minmax_diff = max((abs(x - y) for a, b in zip(jf, rf) for x, y in zip(a[2], b[2])), default=0.0) if same_set else math.inf

    _, jrows = read_csv(jout / f"{sp}_backgroundPredictions.csv")
    _, rrows = read_csv(rout / f"{sp}_backgroundPredictions.csv")
    jpred = {(float(r[0]), float(r[1])): (float(r[2]), float(r[4])) for r in jrows}
    clog_diff, raw_rel, missing = 0.0, 0.0, 0
    for r in rrows:
        key = (float(r[0]), float(r[1]))
        if key not in jpred:
            missing += 1
            continue
        clog_diff = max(clog_diff, abs(float(r[2]) - jpred[key][1]))
    # Raw from lambdas normalizers, relative difference.
    jres = dict(zip(*[l.split(",") for l in (jout / "maxentResults.csv").read_text().splitlines()[:2]]))
    rsum = json.loads((rout / "summary.json").read_text())
    gain_diff = abs(float(jres["Regularized training gain"]) - rsum["regularized_training_gain"])
    it_equal = int(jres["Iterations"]) == rsum["iterations"]
    norm_rel = abs(jm["densityNormalizer"] - rm["densityNormalizer"]) / abs(jm["densityNormalizer"])

    checks = {
        "feature_set": same_set,
        "lambda_abs": lam_diff <= TOLERANCES["lambda_abs"],
        "cloglog_abs": clog_diff <= TOLERANCES["cloglog_abs"] and missing == 0 and len(rrows) == len(jrows),
        "gain_abs": gain_diff <= TOLERANCES["gain_abs"],
        "iterations": it_equal,
        "normalizer_rel": norm_rel <= TOLERANCES["raw_rel"],
    }
    return {
        "case": name, "dataset": ds, "java_args": jargs, "niche_args": nargs,
        "n_features_java": len(jf), "n_features_rust": len(rf),
        "max_abs_lambda_diff": lam_diff, "max_abs_minmax_diff": minmax_diff,
        "max_abs_cloglog_diff": clog_diff, "density_normalizer_rel_diff": norm_rel,
        "gain_java": float(jres["Regularized training gain"]), "gain_rust": rsum["regularized_training_gain"],
        "iterations_java": int(jres["Iterations"]), "iterations_rust": rsum["iterations"],
        "bg_rows_java": len(jrows), "bg_rows_rust": len(rrows),
        "java_wall_s": [t[0] for t in jt], "java_rss_mb": [t[1] for t in jt],
        "rust_wall_s": [t[0] for t in rt], "rust_rss_mb": [t[1] for t in rt],
        "rust_train_ms": rsum["timing_ms"]["train"],
        "java_opt_ms_cold": java_opt_cold * 1e3, "java_opt_ms_warm": java_opt_warm * 1e3,
        "checks": checks, "pass": all(checks.values()),
    }


def fmt_e(x):
    return "∞" if x == math.inf else f"{x:.1e}"


def write_report(results, reps):
    cpu = ""
    try:
        cpu = next(l.split(":", 1)[1].strip() for l in open("/proc/cpuinfo") if l.startswith("model name"))
    except (OSError, StopIteration):
        pass
    java_v = subprocess.run(["java", "-version"], capture_output=True, text=True).stderr.splitlines()[0]
    rustc_v = subprocess.run([os.path.expanduser("~/.cargo/bin/rustc"), "--version"], capture_output=True, text=True).stdout.strip()
    lines = [
        "# Phase 3 — MaxEnt reference comparison (generated)",
        "",
        "Generated by `scripts/compare_maxent.py`; do not edit by hand. Datasets are synthetic",
        "virtual species (`scripts/gen_synthetic.py`), not ecological results.",
        "",
        f"- Machine: {cpu}, {os.cpu_count()} logical CPUs, {platform.system()} {platform.release()}",
        f"- Reference: maxent.jar 3.4.4 (sha256 in docs/benchmarks/methodology.md), {java_v}",
        f"- NicheRS: niche-maxent 0.1.0 (release build), {rustc_v}",
        f"- Repetitions per engine and case: {reps} (wall time is the full process: JVM start-up included for Java)",
        f"- Tolerances (fixed before running): `{json.dumps(TOLERANCES)}`",
        "",
        "## Numerical agreement",
        "",
        "| case | features J/R | max abs Δλ | max abs Δcloglog | Δ gain | iters J/R | result |",
        "|---|---|---|---|---|---|---|",
    ]
    for r in results:
        lines.append(f"| {r['case']} | {r['n_features_java']}/{r['n_features_rust']} | {fmt_e(r['max_abs_lambda_diff'])} | "
                     f"{fmt_e(r['max_abs_cloglog_diff'])} | {r['gain_java'] - r['gain_rust']:.1e} | "
                     f"{r['iterations_java']}/{r['iterations_rust']} | {'PASS' if r['pass'] else 'FAIL: ' + ', '.join(k for k, v in r['checks'].items() if not v)} |")
    lines += [
        "",
        "## Performance (process level)",
        "",
        "Median of the repetitions. Java includes JVM start-up, HTML report and file output that",
        "NicheRS does not produce, so wall-time ratios overstate the kernel difference; the",
        "`rust train` column is the in-process training time only (feature construction + optimizer).",
        "Java optimizer times are those printed by `Sequential.run` (optimizer only, excludes feature",
        f"construction). Warm = median of the last 3 of {WARM_COPIES} identical species trained in one JVM.",
        "",
        "| case | java wall s | rust wall s | java optimizer ms (cold) | java optimizer ms (warm JIT) | rust train ms | java peak RSS MB | rust peak RSS MB |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for r in results:
        lines.append(f"| {r['case']} | {statistics.median(r['java_wall_s']):.2f} | {statistics.median(r['rust_wall_s']):.3f} | "
                     f"{r['java_opt_ms_cold']:.0f} | {r['java_opt_ms_warm']:.0f} | {r['rust_train_ms']:.0f} | {statistics.median(r['java_rss_mb']):.0f} | {statistics.median(r['rust_rss_mb']):.0f} |")
    n_pass = sum(r["pass"] for r in results)
    lines += ["", f"**{n_pass}/{len(results)} cases pass all tolerances.**", ""]
    out = ROOT / "docs/benchmarks/phase3-results.md"
    out.write_text("\n".join(lines))
    print("\n".join(lines))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    names = a.cases or [c for c in CASES if not CASES[c][0].startswith("real/") or (ROOT / "bench-out" / CASES[c][0]).exists()]
    results = []
    for n in names:
        print(f"== {n}", file=sys.stderr)
        results.append(run_case(n, a.reps))
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "results.json").write_text(json.dumps(results, indent=2))
    write_report(results, a.reps)
    return 0 if all(r["pass"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
