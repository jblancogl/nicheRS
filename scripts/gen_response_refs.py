#!/usr/bin/env python3
"""Reference marginal response curves from maxent.jar 3.4.4 for the synthetic golden cases.

Stdlib only. Runs the unmodified jar on every synthetic case of scripts/compare_maxent.py with
the same arguments plus `responsecurves=true writeplotdata=true`, and copies only the marginal
curve data (`plots/<species>_<variable>.dat`, not the one-variable `_only` models) into
tests/reference/maxent-3.4.4/<case>/plots/. Existing reference files are not touched.

Permutation importance needs no extra run: maxentResults.csv already reports it.

Usage: python3 -I scripts/gen_response_refs.py [case ...]
"""
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from compare_maxent import CASES, FIX, JAR, ROOT  # noqa: E402

REF = ROOT / "tests/reference/maxent-3.4.4"


def run(name):
    ds, jargs, _ = CASES[name]
    d = FIX / ds
    sp = (d / "samples.csv").read_text().splitlines()[1].split(",")[0]
    with tempfile.TemporaryDirectory() as tmp:
        cmd = ["java", "-Djava.awt.headless=true", "-Xmx4g", "-jar", str(JAR),
               f"samplesfile={d/'samples.csv'}", f"environmentallayers={d/'background.csv'}",
               f"outputdirectory={tmp}", "autorun", "visible=false", "warnings=false",
               "askoverwrite=false", "pictures=false", "responsecurves=true", "writeplotdata=true",
               "jackknife=false", "outputformat=cloglog", "threads=1",
               "writeclampgrid=false", "writemess=false"] + jargs
        subprocess.run(cmd, check=True, capture_output=True)
        out = REF / name / "plots"
        out.mkdir(parents=True, exist_ok=True)
        n = 0
        for f in sorted((Path(tmp) / "plots").glob(f"{sp}_*.dat")):
            if f.stem.endswith("_only"):
                continue
            shutil.copy(f, out / f.name)
            n += 1
        print(f"{name}: {n} curves")


def main():
    names = sys.argv[1:] or [n for n, (ds, _, _) in CASES.items()
                             if not ds.startswith("real/") and (REF / n).is_dir()]
    for n in names:
        run(n)


if __name__ == "__main__":
    main()
