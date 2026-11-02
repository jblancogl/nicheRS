#!/usr/bin/env python3
"""Generate deterministic synthetic SDM datasets in MaxEnt SWD format.

Stdlib only, so the generated files depend on nothing but this script and the seed.
Every dataset is SYNTHETIC: a virtual species with a known response, used to compare
NicheRS against maxent.jar. These are not ecological results.

Layout per dataset (tests/fixtures/synthetic/<name>/):
  samples.csv     species,x,y,<vars>   presence records
  background.csv  species,x,y,<vars>   background points (fixed, so no random draw)
  truth.csv       x,y,suitability      true relative suitability on the background
  meta.json       generation parameters
"""
import json
import math
import random
import sys
from pathlib import Path

NODATA = -9999


def env_at(x, y, n):
    """Environmental variables at grid position (x, y) on an n x n grid."""
    u, v = x / (n - 1), y / (n - 1)
    temp = 5.0 + 20.0 * u + 1.5 * math.sin(7.0 * v)          # west-east gradient
    prec = 400.0 + 1200.0 * v + 80.0 * math.cos(5.0 * u)      # south-north gradient
    rough = math.sin(9.0 * u) * math.cos(6.0 * v) + 0.3 * math.sin(23.0 * u * v)
    cls = 1 + (int(u * 4) + 2 * int(v * 2)) % 4               # 4-class "land cover"
    return {"temp": temp, "prec": prec, "rough": rough, "landcover": cls}


def suitability(e, with_cat):
    s = math.exp(-((e["temp"] - 16.0) ** 2) / (2 * 3.0 ** 2))
    s *= 1.0 / (1.0 + math.exp(-(e["prec"] - 900.0) / 120.0))
    s *= 0.6 + 0.4 * max(0.0, e["rough"])  # hinge-like response
    if with_cat:
        s *= {1: 1.0, 2: 0.15, 3: 0.6, 4: 0.05}[e["landcover"]]
    return s


def generate(name, *, n_grid, n_presence, n_background, seed, variables, missing_frac=0.0):
    rng = random.Random(seed)
    with_cat = "landcover" in variables
    cells = [(x, y) for y in range(n_grid) for x in range(n_grid)]
    envs = {c: env_at(c[0], c[1], n_grid) for c in cells}
    suit = {c: suitability(envs[c], with_cat) for c in cells}

    background = cells if n_background >= len(cells) else rng.sample(cells, n_background)
    background.sort(key=lambda c: (c[1], c[0]))
    weights = [suit[c] for c in cells]
    presence = set()
    while len(presence) < n_presence:  # one record per cell, sampled proportional to suitability
        presence.add(rng.choices(cells, weights=weights, k=1)[0])
    presence = sorted(presence, key=lambda c: (c[1], c[0]))

    def fmt(c, missing_ok):
        vals = []
        for v in variables:
            val = envs[c][v]
            if missing_ok and missing_frac > 0 and rng.random() < missing_frac:
                vals.append(str(NODATA))
            elif v == "landcover":
                vals.append(str(val))
            else:
                vals.append(repr(round(val, 6)))
        return vals

    out = Path("tests/fixtures/synthetic") / name
    out.mkdir(parents=True, exist_ok=True)
    header = "species,x,y," + ",".join(variables) + "\n"
    with open(out / "samples.csv", "w") as f:
        f.write(header)
        for c in presence:
            f.write(",".join(["virtualis", str(c[0]), str(c[1])] + fmt(c, False)) + "\n")
    with open(out / "background.csv", "w") as f:
        f.write(header)
        for c in background:
            f.write(",".join(["background", str(c[0]), str(c[1])] + fmt(c, True)) + "\n")
    with open(out / "truth.csv", "w") as f:
        f.write("x,y,suitability\n")
        for c in background:
            f.write(f"{c[0]},{c[1]},{suit[c]:.6g}\n")
    meta = dict(name=name, n_grid=n_grid, n_presence=n_presence, n_background=len(background),
                seed=seed, variables=variables, missing_frac=missing_frac, nodata=NODATA,
                synthetic=True, generator="scripts/gen_synthetic.py")
    (out / "meta.json").write_text(json.dumps(meta, indent=2) + "\n")
    print(f"{name}: {len(presence)} presences, {len(background)} background")


CONT = ["temp", "prec", "rough"]
DATASETS = {
    "s01_small":      dict(n_grid=60,  n_presence=12,   n_background=3600,  seed=11, variables=CONT),
    "s02_medium":     dict(n_grid=100, n_presence=60,   n_background=10000, seed=12, variables=CONT),
    "s03_large":      dict(n_grid=150, n_presence=300,  n_background=10000, seed=13, variables=CONT),
    "s04_categorical":dict(n_grid=100, n_presence=150,  n_background=10000, seed=14, variables=CONT + ["landcover"]),
    "s05_missing":    dict(n_grid=100, n_presence=150,  n_background=10000, seed=15, variables=CONT, missing_frac=0.02),
    "s06_bigbg":      dict(n_grid=300, n_presence=500,  n_background=50000, seed=16, variables=CONT),
}

if __name__ == "__main__":
    names = sys.argv[1:] or list(DATASETS)
    for n in names:
        generate(n, **DATASETS[n])
