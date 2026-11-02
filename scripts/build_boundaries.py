#!/usr/bin/env python3
"""Administrative boundaries for NicheRS study areas: download, normalise, simplify.

Outside shipped code (uses GDAL through pyogrio, and shapely). Writes one NDJSON line per
administrative unit, which `niche boundaries build` encodes into the compact pack the server
serves (`<cache>/boundaries/*.nrsb`). Downloads go to a work directory, never into the repo.

Sources (both allow commercial reuse with attribution):
  FR  IGN ADMIN EXPRESS COG CARTO 4.0 (GPKG, WGS84, whole France incl. overseas), Licence Ouverte 2.0
  ES  IGN España, OGC API Features `administrativeunit` (INSPIRE AU), CC BY 4.0 equivalent

Levels: country, region (FR région / ES comunidad autónoma), department (FR département /
ES provincia), intercommunality (FR EPCI), municipality (FR commune / ES municipio); with
`--set protected`, a second pack of protected areas (FR: INPN parks, reserves, APB, Ramsar,
Natura 2000; ES: MITECO ENP and Red Natura 2000), Licence Ouverte 2.0 / CC BY 4.0. Each
level is simplified for its scale; study areas are rasterised on a 30-arc-second (~1 km)
grid, so the commune tolerance (~60 m) is far below a grid cell.

Usage:
  uv run --no-project --with pyogrio --with shapely --with numpy python -I scripts/build_boundaries.py \\
      --work /tmp/nichers-boundaries --out /tmp/nichers-boundaries/fr-es.ndjson
"""
import argparse
import hashlib
import json
import subprocess
import sys
import urllib.request
from pathlib import Path

FR_EDITION = "ADMIN-EXPRESS-COG-CARTO_4-0__GPKG_WGS84G_FRA_2026-01-01"
FR_URL = f"https://data.geopf.fr/telechargement/download/ADMIN-EXPRESS-COG-CARTO/{FR_EDITION}/{FR_EDITION}.7z"
ES_API = "https://api-features.ign.es/collections/administrativeunit/items"
# The IGN download service refuses Python's default user agent.
USER_AGENT = "NicheRS-boundaries/1"


def urlopen(url: str, timeout: int):
    return urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": USER_AGENT}), timeout=timeout)

# Douglas–Peucker tolerance in degrees per level (1e-3° ≈ 110 m in latitude).
TOLERANCE = {"country": 0.004, "region": 0.002, "department": 0.0012, "intercommunality": 0.0008, "municipality": 0.0005,
             "protected": 0.0005}

# Countries are also searched by their names in the UI languages and co-official languages.
ALIASES = {
    "FR": ["Francia", "Frantzia", "Frankreich"],
    "ES": ["Spain", "Espagne", "Espanya", "Espainia", "Espanha", "Spanien"],
    "FR-METRO": ["Metropolitan France", "Francia metropolitana", "Hexagone"],
    "ES-PENIN": ["Peninsular Spain and Balearic Islands", "Espagne péninsulaire et Baléares", "Península y Baleares"],
}
# The modelling grid covers the bounding box of the study area: a country with overseas
# territories or distant islands never fits at 1 km. These units are the usual mainland study
# areas (FR: régions 01–06 are overseas; ES: Canarias, Ceuta, Melilla, plazas de soberanía).
FR_OVERSEAS = {"01", "02", "03", "04", "06"}
ES_NOT_PENINSULAR = {"3405", "3418", "3419", "3420"}

SOURCES = {
    "FR": {
        "name": "IGN — ADMIN EXPRESS COG CARTO 4.0, édition 2026-01-01",
        "licence": "Licence Ouverte / Open Licence 2.0 (Etalab)",
        "url": FR_URL,
        "attribution": "© IGN — ADMIN EXPRESS COG CARTO 2026",
    },
    "ES": {
        "name": "IGN España — Unidades administrativas (INSPIRE AU), API OGC Features",
        "licence": "CC BY 4.0 (equivalente, IGN/CNIG)",
        "url": ES_API,
        "attribution": "© Instituto Geográfico Nacional de España",
    },
}


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def fetch(url: str, dest: Path) -> Path:
    if not dest.exists():
        print(f"downloading {url}", file=sys.stderr)
        tmp = dest.with_suffix(dest.suffix + ".part")
        with urlopen(url, timeout=600) as r, tmp.open("wb") as f:
            while chunk := r.read(1 << 20):
                f.write(chunk)
        tmp.rename(dest)
    return dest


def simplify(geom, level):
    from shapely import make_valid
    from shapely.geometry import MultiPolygon, Polygon

    g = make_valid(geom).simplify(TOLERANCE[level], preserve_topology=True)
    polys = [g] if isinstance(g, Polygon) else [p for p in getattr(g, "geoms", []) if isinstance(p, Polygon)]
    # Drop slivers far below a grid cell (islets < ~0.02 km² at this latitude).
    polys = [p for p in polys if p.area > 2e-6] or polys
    return MultiPolygon(polys) if polys else None


def emit(out, unit):
    from shapely.geometry import mapping

    g = simplify(unit.pop("geometry"), unit["level"])
    if g is None or g.is_empty:
        return 0
    unit["geometry"] = mapping(g)
    out.write(json.dumps(unit, ensure_ascii=False, separators=(",", ":")) + "\n")
    return 1


def france(work: Path, out) -> dict:
    import pyogrio
    from shapely import from_wkb, union_all

    archive = fetch(FR_URL, work / f"{FR_EDITION}.7z")
    gpkg = next(work.glob(f"**/{FR_EDITION.split('__')[0]}*/**/*.gpkg"), None) or next(work.glob("**/ADE-COG-CARTO*/**/*.gpkg"), None)
    if gpkg is None:
        subprocess.run(["7z", "x", "-y", f"-o{work}", str(archive)], check=True, stdout=subprocess.DEVNULL)
        gpkg = next(work.glob("**/*.gpkg"))
    layers = {name.lower(): name for name, _ in pyogrio.list_layers(gpkg)}

    n = 0

    def rows(layer):
        meta, _fids, geoms, fields = pyogrio.raw.read(gpkg, layer=layers[layer])
        names = list(meta["fields"])
        for i, wkb in enumerate(geoms):
            yield {k: fields[j][i] for j, k in enumerate(names)}, from_wkb(wkb)

    regions = list(rows("region"))
    for p, g in regions:
        n += emit(out, {"id": f"FR-R{p['code_insee']}", "country": "FR", "level": "region", "name": p["nom_officiel"], "parent": "FR", "geometry": g})
    for p, g in rows("departement"):
        n += emit(out, {"id": f"FR-D{p['code_insee']}", "country": "FR", "level": "department", "name": p["nom_officiel"], "parent": f"FR-R{p['code_insee_de_la_region']}", "geometry": g})
    for p, g in rows("epci"):
        if p.get("code_siren"):
            # An EPCI may straddle departments: attach it to the first one listed (for display).
            deps = [d for d in str(p.get("codes_insee_des_departements_membres") or "").replace("/", " ").replace(",", " ").split() if d]
            parent = f"FR-D{deps[0]}" if deps else "FR"
            n += emit(out, {"id": f"FR-E{p['code_siren']}", "country": "FR", "level": "intercommunality", "name": p["nom_officiel"], "parent": parent, "geometry": g})
    for p, g in rows("commune"):
        pop = p.get("population")
        n += emit(out, {"id": f"FR-C{p['code_insee']}", "country": "FR", "level": "municipality", "name": p["nom_officiel"], "parent": f"FR-D{p['code_insee_du_departement']}", "population": int(pop) if pop else None, "geometry": g})
    n += emit(out, {"id": "FR", "country": "FR", "level": "country", "name": "France", "parent": None, "aliases": ALIASES["FR"], "geometry": union_all([g for _, g in regions])})
    metro = [g for p, g in regions if str(p["code_insee"]) not in FR_OVERSEAS]
    n += emit(out, {"id": "FR-METRO", "country": "FR", "level": "country", "name": "France métropolitaine", "parent": None, "aliases": ALIASES["FR-METRO"], "geometry": union_all(metro)})
    return {**SOURCES["FR"], "edition": FR_EDITION, "sha256": sha256(archive), "units": n}


def spain(work: Path, out) -> dict:
    from shapely.geometry import shape
    from shapely import union_all

    cache = work / "es-administrativeunit.ndjson"
    if not cache.exists():
        tmp = cache.with_suffix(".part")
        with tmp.open("w") as f:
            offset = 0
            while True:
                url = f"{ES_API}?f=json&limit=500&offset={offset}"
                with urlopen(url, timeout=300) as r:
                    page = json.load(r)
                feats = page.get("features", [])
                for ft in feats:
                    f.write(json.dumps(ft) + "\n")
                print(f"ES {offset + len(feats)} units", file=sys.stderr)
                if len(feats) < 500:
                    break
                offset += 500
        tmp.rename(cache)
    level_of = {"Municipio": "municipality", "Provincia": "department", "Comunidad autónoma": "region", "Ciudad con estatuto de autonomía": "region", "País": "country"}
    units, seen_levels = [], {}
    for line in cache.open():
        ft = json.loads(line)
        p = ft["properties"]
        name_level = p.get("nationallevelname") or ""
        seen_levels[name_level] = seen_levels.get(name_level, 0) + 1
        level = level_of.get(name_level)
        if level is None or ft.get("geometry") is None:
            continue
        units.append((level, p, shape(ft["geometry"])))
    print(f"ES levels: {seen_levels}", file=sys.stderr)
    # nationalcode: 34 (country) · CCAA (2) · provincia (2) · … · municipio; parents from prefixes.
    prov_by_code = {p["nationalcode"]: p for lv, p, _ in units if lv == "department"}
    n = 0
    for level, p, g in units:
        code = p["nationalcode"]
        if level == "region":
            parent = "ES"
        elif level == "department":
            parent = f"ES-{code[:4]}"
        elif level == "municipality":
            parent = f"ES-{code[:6]}" if f"{code[:6]}" in prov_by_code or any(c.startswith(code[:6]) for c in prov_by_code) else "ES"
        else:
            parent = None
        extra = {"aliases": ALIASES["ES"]} if level == "country" else {}
        uid = "ES" if level == "country" else f"ES-{code[:4] if level == 'region' else code[:6] if level == 'department' else code}"
        n += emit(out, {"id": uid, "country": "ES", "level": level, "name": p["nameunit"], "parent": parent, **extra, "geometry": g})
    mainland = [g for lv, p, g in units if lv == "region" and p["nationalcode"][:4] not in ES_NOT_PENINSULAR]
    n += emit(out, {"id": "ES-PENIN", "country": "ES", "level": "country", "name": "España peninsular y Baleares", "parent": None, "aliases": ALIASES["ES-PENIN"], "geometry": union_all(mainland)})
    if not any(lv == "country" for lv, _, _ in units):
        n += emit(out, {"id": "ES", "country": "ES", "level": "country", "name": "España", "parent": None, "aliases": ALIASES["ES"], "geometry": union_all([g for lv, _, g in units if lv == "region"])})
    return {**SOURCES["ES"], "edition": "api-features " + __import__("datetime").date.today().isoformat(), "sha256": sha256(cache), "units": n}


# ---- protected areas (second pack: `--set protected`) -------------------------------------------

PATRINAT = "https://assets.patrinat.fr/files/donnees"
MITECO_BDN = "https://www.miteco.gob.es/content/dam/miteco/es/biodiversidad/servicios/banco-datos-naturaleza"
PROTECTED_SOURCES = [
    {
        "country": "FR",
        "name": "MNHN/PatriNat — INPN, base des espaces protégés (métropole)",
        "licence": "Licence Ouverte / Open Licence 2.0 (Etalab)",
        "url": f"{PATRINAT}/ep/EP.zip",
        "attribution": "© MNHN/PatriNat — INPN, espaces protégés",
    },
    {
        "country": "FR",
        "name": "MNHN/PatriNat — INPN, sites Natura 2000",
        "licence": "Licence Ouverte / Open Licence 2.0 (Etalab)",
        "url": f"{PATRINAT}/natura2000/NATURA2000.zip",
        "attribution": "© MNHN/PatriNat — INPN, Natura 2000",
    },
    {
        "country": "ES",
        "name": "MITECO — Banco de Datos de la Naturaleza, Espacios Naturales Protegidos",
        "licence": "CC BY 4.0",
        "url": f"{MITECO_BDN}/enp/Enp2025_shp.zip",
        "attribution": "© Ministerio para la Transición Ecológica y el Reto Demográfico, ENP",
    },
    {
        "country": "ES",
        "name": "MITECO — Banco de Datos de la Naturaleza, Red Natura 2000",
        "licence": "CC BY 4.0",
        "url": f"{MITECO_BDN}/3-rn2000/n2000_2025_shp.zip",
        "attribution": "© Ministerio para la Transición Ecológica y el Reto Demográfico, Red Natura 2000",
    },
]
# Designations that delimit a protected territory one may model; land purchases, listed
# landscapes, single trees and caves are left out.
FR_DESIGNATIONS = {
    "Parc naturel régional", "Parc national, zone cœur", "Parc national, aire d'adhésion",
    "Réserve naturelle nationale", "Réserve naturelle régionale", "Réserve naturelle de Corse",
    "Réserve biologique intégrale", "Réserve biologique dirigée", "Arrêté de protection de biotope",
    "Zone humide protégée par la convention de Ramsar", "Réserve nationale de chasse et de faune sauvage",
}
ES_ENP_SKIP = {"Árbol Singular", "Cuevas", "Zona de Importancia Comunitaria ZIC (ZEPA/ZEC)",
               "Espacio Protegido Red Natura 2000", "Zona de la Red Ecológica Europea Natura 2000"}
ES_N2000_TYPE = {"A": "ZEPA", "B": "ZEC", "C": "ZEPA y ZEC"}
# Below one 30" cell of land a protected area cannot be a study area on its own.
MIN_LAND_KM2 = 1.0


def unzip(archive: Path, dest: Path) -> Path:
    if not dest.exists():
        subprocess.run(["unzip", "-o", "-q", str(archive), "-d", str(dest)], check=True)
    return dest


def to_wgs84(geoms, crs: str):
    import numpy as np
    from pyproj import Transformer
    from shapely import transform

    t = Transformer.from_crs(crs, "EPSG:4326", always_xy=True)
    return [transform(g, lambda xy: np.column_stack(t.transform(xy[:, 0], xy[:, 1]))) for g in geoms]


def read_layer(path: Path):
    import pyogrio
    from shapely import from_wkb

    meta, _fids, geoms, fields = pyogrio.raw.read(path)
    names = list(meta["fields"])
    rows = [{k: fields[j][i] for j, k in enumerate(names)} for i in range(len(geoms))]
    return rows, to_wgs84([from_wkb(g) for g in geoms], meta["crs"])


class Land:
    """Land area of a geometry (km², approximate) from the departments/provinces of the admin pack."""

    def __init__(self, admin_ndjson: Path):
        from shapely import STRtree
        from shapely.geometry import shape

        self.geoms = [shape(json.loads(l)["geometry"]) for l in admin_ndjson.open() if '"level":"department"' in l]
        self.tree = STRtree(self.geoms)

    def km2(self, g) -> float:
        import math

        total = 0.0
        for i in self.tree.query(g, predicate="intersects"):
            part = g.intersection(self.geoms[i])
            if not part.is_empty:
                total += part.area * 111.32**2 * math.cos(math.radians(part.centroid.y))
        return total


def protected_units(work: Path, land: Land, out) -> tuple[int, list]:
    from shapely import union_all

    counts, hashes = {}, []

    def put(uid, country, name, designation, source, geom):
        geom = geom.buffer(0) if not geom.is_valid else geom
        if land.km2(geom) < MIN_LAND_KM2:
            return
        key = (country, designation)
        counts[key] = counts.get(key, 0) + emit(out, {"id": uid, "country": country, "level": "protected", "name": str(name).strip(),
                                                      "parent": None, "designation": designation, "source": source, "geometry": geom})

    # FR: base des espaces protégés (metropolitan GeoPackage, Lambert-93).
    ep = fetch(PROTECTED_SOURCES[0]["url"], work / "fr-EP.zip")
    hashes.append(sha256(ep))
    rows, geoms = read_layer(unzip(ep, work / "fr-EP") / "EP" / "sig_metrop.gpkg")
    for r, g in zip(rows, geoms):
        if r["type_espace"] in FR_DESIGNATIONS:
            put(f"FR-P{r['id_mnhn']}", "FR", r["nom"], r["type_espace"], 0, g)

    # FR: Natura 2000 (codes in the shapefile, names and marine share in biotop.csv).
    n2k = fetch(PROTECTED_SOURCES[1]["url"], work / "fr-NATURA2000.zip")
    hashes.append(sha256(n2k))
    base = unzip(unzip(n2k, work / "fr-N2000") / "NATURA_BDD_122024.zip", work / "fr-N2000-bdd") / "NATURA_BDD_122024"
    import csv

    with (base / "biotop.csv.csv").open(encoding="utf-8") as f:
        sites = {r["sitecode"]: r for r in csv.DictReader(f, delimiter=";")}
    rows, geoms = read_layer(base / "SIG_NATURA" / "natura_sig.shp")
    for r, g in zip(rows, geoms):
        code = str(r["cd_sig"])[-9:]
        site = sites.get(code, {})
        if float(site.get("marine_area_pourc") or 0) >= 95:
            continue
        put(f"FR-N{code}", "FR", site.get("site_name") or code, f"Natura 2000 · {r['type_espac']}", 1, g)

    # ES: Espacios Naturales Protegidos (peninsula + Balearics, Canarias); parts share a code.
    enp = fetch(PROTECTED_SOURCES[2]["url"], work / "es-Enp2025_shp.zip")
    hashes.append(sha256(enp))
    parts = {}
    for shp in sorted(unzip(enp, work / "es-enp").glob("**/*.shp")):
        rows, geoms = read_layer(shp)
        for r, g in zip(rows, geoms):
            if r["ODESIGNATE"] not in ES_ENP_SKIP:
                parts.setdefault(r["SITE_CODE_"], (r, []))[1].append(g)
    for code, (r, gs) in parts.items():
        put(f"ES-P{code}", "ES", r["SITE_NAME"], r["ODESIGNATE"], 2, union_all(gs))

    # ES: Red Natura 2000.
    rn = fetch(PROTECTED_SOURCES[3]["url"], work / "es-n2000_2025_shp.zip")
    hashes.append(sha256(rn))
    for shp in sorted(unzip(rn, work / "es-n2000").glob("**/*.shp")):
        rows, geoms = read_layer(shp)
        for r, g in zip(rows, geoms):
            put(f"ES-N{r['site_code']}", "ES", r["SITE_NAME"], f"Natura 2000 · {ES_N2000_TYPE.get(r['TIPO'], r['TIPO'])}", 3, g)

    for (c, d), n in sorted(counts.items()):
        print(f"{c} {d}: {n}", file=sys.stderr)
    import datetime

    today = datetime.date.today().isoformat()
    sources = [{**src, "edition": f"downloaded {today}", "sha256": h} for src, h in zip(PROTECTED_SOURCES, hashes)]
    return sum(counts.values()), sources


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--work", required=True, type=Path, help="download and scratch directory (not the repo)")
    ap.add_argument("--out", required=True, type=Path, help="NDJSON output for `niche boundaries build`")
    ap.add_argument("--countries", default="FR,ES")
    ap.add_argument("--set", choices=["admin", "protected"], default="admin",
                    help="administrative units, or protected areas (needs --land)")
    ap.add_argument("--land", type=Path, help="admin NDJSON of the same countries (land filter for protected areas)")
    a = ap.parse_args()
    a.work.mkdir(parents=True, exist_ok=True)
    if a.set == "protected":
        if not a.land:
            ap.error("--set protected needs --land <admin ndjson>")
        with a.out.open("w") as out:
            n, sources = protected_units(a.work, Land(a.land), out)
        meta = a.out.with_suffix(".sources.json")
        meta.write_text(json.dumps({"tolerance_degrees": TOLERANCE["municipality"], "min_land_km2": MIN_LAND_KM2,
                                    "units": n, "sources": sources}, ensure_ascii=False, indent=2))
        print(json.dumps(sources, ensure_ascii=False, indent=2))
        return
    sources = []
    with a.out.open("w") as out:
        for c in a.countries.split(","):
            sources.append({"FR": france, "ES": spain}[c](a.work, out))
    meta = a.out.with_suffix(".sources.json")
    meta.write_text(json.dumps({"tolerance_degrees": TOLERANCE, "sources": sources}, ensure_ascii=False, indent=2))
    print(json.dumps(sources, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
