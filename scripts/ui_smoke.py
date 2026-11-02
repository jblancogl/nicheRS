#!/usr/bin/env python3
"""UI smoke test: runs synthetic demo studies end to end in a headless browser.

Development-only (needs Playwright for Python and its Chromium):
    NICHE_DATA_DIR=$(mktemp -d) ./target/release/niche serve --port 8091 &   # local mode, fresh data
    uv run --with playwright python -I scripts/ui_smoke.py http://127.0.0.1:8091 [--chromium PATH] [--shots DIR]

Every dataset used here is SYNTHETIC (the demo's virtual species, and a generated survey written by
this script): nothing here is a real observation or prediction.

Covers:
- Expert wizard on the demo: grid resolution, own-data import (a SYNTHETIC Lambert-93 CSV, held
  out for independent validation), predictor group headings, algorithm (ensemble MaxEnt + RF),
  target-group background, distance thinning, spatial grid blocks, null models, AOA, a future
  projection; launch and live progress.
- Results with the scientific wording of ADR-0007 (relative suitability, not probability),
  synthetic labelling, evidence level; Validation tab (null models, independent validation),
  map layers and legends (AOA hatching, sampling effort, future change), Future tab.
- A second demo study with two species: Decision support (survey priority, project footprint,
  several species, comparison of the two studies), analysis warnings in the interface language.
- Labels of those screens in FR and ES (language chosen in the user menu's submenu), forbidden
  wording in every language, no side-panel overflow at desktop, tablet and phone widths.
- Welcome screen (draft offered, discard), help: contextual "?" opens the matching section, Esc
  closes and returns focus, every article in every language free of "probability of presence"
  wording outside explicit negations, bottom sheet on a phone viewport, no console errors.
"""
import argparse
import json
import math
import os
import random
import re
import sys
import tempfile
import traceback

from playwright.sync_api import sync_playwright

BENIGN = ("GL Driver Message", "script-src' was not explicitly set")

ARTICLES = ["getting-started", "map", "evidence", "validation", "area", "uncertainty", "data",
            "variables", "predictors", "algorithms", "future", "field-data", "decision", "exports", "account", "glossary", "responsible"]
# Phrases that must only appear inside an explicit negation ("not a ...", "n'est pas ...", "no es ...").
PRESENCE = {"en": "probability of presence", "fr": "probabilité de présence", "es": "probabilidad de presencia"}
NEGATION = re.compile(r"\b(not|never|pas|jamais|no|nunca|ni)\b", re.I)
EXPECT = {"en": "relative habitat suitability", "fr": "idonéité relative", "es": "idoneidad relativa"}
LANG_LABEL = {"en": "English", "fr": "Français", "es": "Español"}

# Interface labels per language for the screens checked in every language (from the messages files).
L = {
    "en": {"tabs": ["Summary", "Validation", "Variables", "Data & exports", "Future", "Decision support"],
           "null": "Null models (random locations)", "priority": "Survey priority", "footprint": "Project footprint",
           "several": "Several species", "compare": "Compare with…", "aoa": "Outside the area of applicability (hatching)",
           "effort": "Sampling effort (target-group records)", "change": "Change in suitable conditions",
           "future": "Future climate projections", "independent": "Independent validation"},
    "fr": {"tabs": ["Synthèse", "Validation", "Variables", "Données et exports", "Futur", "Aide à la décision"],
           "null": "Modèles nuls", "priority": "Priorité de prospection", "footprint": "Emprise de projet",
           "several": "Plusieurs espèces", "compare": "Comparer avec…", "aoa": "Hors du domaine d'applicabilité (hachures)",
           "effort": "Effort d'échantillonnage (données du groupe cible)", "change": "Évolution des conditions favorables",
           "future": "Projections sous climat futur", "independent": "Validation indépendante"},
    "es": {"tabs": ["Resumen", "Validación", "Variables", "Datos y exportación", "Futuro", "Apoyo a la decisión"],
           "null": "Modelos nulos", "priority": "Prioridad de prospección", "footprint": "Huella de proyecto",
           "several": "Varias especies", "compare": "Comparar con…", "aoa": "Fuera del área de aplicabilidad (rayado)",
           "effort": "Esfuerzo de muestreo (registros del grupo objetivo)", "change": "Cambio en las condiciones favorables",
           "future": "Proyecciones con clima futuro", "independent": "Validación independiente"},
}
# English texts that must not remain on FR/ES screens (server texts worded by the client).
ENGLISH_LEFTOVERS = ["Synthetic temperature", "Synthetic precipitation", "Synthetic demonstration layer",
                     "Stacked species maps overpredict", "No sampling-effort surface for this study",
                     "smaller than one model cell", "of the footprint lies outside", "is below the minimum"]
# Wording never allowed about suitability outputs (AGENTS.md, ADR-0007), outside negations.
FORBIDDEN = {"en": ["occupied area", "probability of occurrence:", "confidence:"],
             "fr": ["surface occupée", "aire occupée"],
             "es": ["área ocupada", "superficie ocupada"]}


def wording_failures(lang: str, text: str, where: str) -> list[str]:
    out = []
    low = text.lower()
    for m in re.finditer(re.escape(PRESENCE[lang]), low):
        window = low[max(0, m.start() - 45):m.start()]
        if not NEGATION.search(window):
            out.append(f"{where} [{lang}]: '{PRESENCE[lang]}' outside a negation: …{text[max(0, m.start() - 45):m.end()]}")
    for bad in ("confidence:", "% confidence", "% de confiance", "% de confianza"):
        if bad in low and not NEGATION.search(low[max(0, low.index(bad) - 60):low.index(bad)]):
            out.append(f"{where} [{lang}]: forbidden '{bad}'")
    return out


def results_wording(lang: str, text: str, where: str) -> list[str]:
    out = wording_failures(lang, text, where)
    low = text.lower()
    for bad in FORBIDDEN[lang]:
        i = low.find(bad)
        if i >= 0 and not NEGATION.search(low[max(0, i - 60):i]):
            out.append(f"{where} [{lang}]: forbidden wording '{bad}'")
    return out


# --- SYNTHETIC survey in Lambert-93 (EPSG:2154), written by this script -----------------------

def lambert93(lng: float, lat: float) -> tuple[float, float]:
    """WGS84 → Lambert-93 (LCC 2SP on GRS80; differences with ETRS89 negligible here)."""
    a, f = 6378137.0, 1 / 298.257222101
    e = math.sqrt(2 * f - f * f)
    rad = math.radians

    def m(p):
        return math.cos(p) / math.sqrt(1 - (e * math.sin(p)) ** 2)

    def t(p):
        return math.tan(math.pi / 4 - p / 2) / ((1 - e * math.sin(p)) / (1 + e * math.sin(p))) ** (e / 2)

    p1, p2, p0, l0 = rad(49), rad(44), rad(46.5), rad(3)
    n = (math.log(m(p1)) - math.log(m(p2))) / (math.log(t(p1)) - math.log(t(p2)))
    F = m(p1) / (n * t(p1) ** n)
    r0 = a * F * t(p0) ** n
    r = a * F * t(rad(lat)) ** n
    th = n * (rad(lng) - l0)
    return 700000 + r * math.sin(th), 6600000 + r0 - r * math.cos(th)


def synthetic_survey(path: str) -> None:
    """60 SYNTHETIC presence/absence sites over the demo region (3–6 °E, 43.8–45.8 °N), French
    decimal commas, ';' separator. Presence follows a smooth invented gradient: not field data."""
    rng = random.Random(20261020)
    rows = ["nom_cite;x_l93;y_l93;date_obs;statut;remarque"]
    for i in range(60):
        lng, lat = 3.15 + rng.random() * 2.7, 43.95 + rng.random() * 1.7
        x, y = lambert93(lng, lat)
        found = rng.random() < 0.15 + 0.7 * (lat - 43.8) / 2.0
        xs, ys = f"{x:.2f}".replace(".", ","), f"{y:.2f}".replace(".", ",")
        status = "Présent" if found else "Non observé"
        rows.append(f"Virtualis demonstrandum;{xs};{ys};{2021 + i % 3}-05-17;{status};SYNTHETIC test row, not a field observation")
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(rows) + "\n")


# --- helpers ----------------------------------------------------------------------------------

def shot(pg, shots, name):
    if shots:
        pg.screenshot(path=os.path.join(shots, f"{name}.png"))


def set_lang(pg, lang: str) -> None:
    """The interface language is chosen in the user menu, in its Language submenu."""
    pg.locator("[data-user-menu]").click()
    pg.locator("[data-language-menu]").click()
    pg.get_by_role("menuitemradio", name=LANG_LABEL[lang]).click()
    for _ in range(3):  # close the submenu and the menu, whichever stay open
        if not pg.get_by_role("menu").count():
            break
        pg.keyboard.press("Escape")
        pg.wait_for_timeout(200)
    pg.wait_for_timeout(300)


def start_demo(pg, base: str, second: bool) -> None:
    """New study, Expert path, synthetic demo (optionally with the second virtual species)."""
    pg.goto(base + "/")
    pg.wait_for_load_state("networkidle")
    demo = pg.get_by_role("button", name="Try the synthetic demo")
    if demo.count():  # empty workspace: the demo is offered first, then the setup path
        demo.first.click()
    else:
        pg.get_by_role("button", name="New study").first.click()
    expert = pg.get_by_role("button", name="Expert").filter(has_text="Expert").first
    expert.wait_for(timeout=10_000)
    expert.click()
    pg.wait_for_timeout(500)
    if demo.count():
        demo.first.click()
    pg.locator("#side-panel").wait_for(timeout=10_000)
    pg.wait_for_timeout(800)
    if second:
        # The demo lands on the data step; the second species is added on the species step.
        pg.locator("#side-panel").get_by_role("button", name=re.compile("^Species")).click()
        pg.get_by_role("button", name="Add a second synthetic species").click()
        pg.wait_for_timeout(300)
        pg.locator("#side-panel").get_by_role("button", name=re.compile("^Data review")).click()
        pg.wait_for_timeout(800)


def continue_(pg) -> None:
    btns = [x for x in pg.locator("#side-panel button").all() if x.is_visible()]
    btns[-1].click()
    pg.wait_for_timeout(1500)


def choose_select(pg, trigger: str, option: str) -> None:
    pg.locator(trigger).click()
    pg.get_by_role("option", name=option, exact=True).click()


def launch_and_wait(pg, failures, what: str) -> str | None:
    launch = pg.get_by_role("button", name="Launch study")
    if not launch.count():
        failures.append(f"{what}: could not reach the launch step")
        return None
    launch.click()
    seen_progress = False
    for _ in range(900):
        if "/results" in pg.url:
            break
        if not seen_progress and pg.locator("[role=progressbar]").count():
            seen_progress = True
        pg.wait_for_timeout(1000)
    if "/results" not in pg.url:
        failures.append(f"{what}: results page not reached after 15 min")
        return None
    if not seen_progress:
        failures.append(f"{what}: no live progress bar seen while running")
    pg.wait_for_load_state("networkidle")
    pg.wait_for_timeout(1500)
    return pg.url.split("?")[0]


def panel_overflow(pg, where: str) -> list[str]:
    """Elements of the side panel wider than the panel (horizontal scroll inside an element that
    scrolls on purpose — tables in an overflow-x container — is allowed)."""
    bad = pg.evaluate(
        """() => {
      const p = document.querySelector('#side-panel'); if (!p) return [];
      const r = p.getBoundingClientRect(); const out = [];
      for (const el of p.querySelectorAll('*')) {
        const b = el.getBoundingClientRect();
        if (b.width === 0 || b.right <= r.right + 1) continue;
        let a = el.parentElement, clipped = false;
        while (a && a !== p) { const s = getComputedStyle(a);
          if (['auto','scroll','hidden','clip'].includes(s.overflowX) && a.getBoundingClientRect().right <= r.right + 1) { clipped = true; break; }
          a = a.parentElement; }
        if (!clipped) out.push(el.tagName + '.' + (el.className && el.className.baseVal === undefined ? String(el.className).slice(0, 40) : '') + ' +' + Math.round(b.right - r.right) + 'px');
      }
      return out.slice(0, 3);
    }"""
    )
    # The null-model table must fit the panel (its verdict column used to scroll out of sight).
    # Wider data tables (footprint, candidates, per-GCM) scroll horizontally on purpose.
    wide = pg.evaluate(
        """() => [...document.querySelectorAll('#side-panel table')]
          .filter(t => /random locations|aléatoires|aleatorias/.test(t.querySelector('caption')?.textContent || ''))
          .filter(t => t.getClientRects().length && t.scrollWidth > t.parentElement.clientWidth + 1)
          .map(t => (t.querySelector('caption, th') || t).textContent.trim().slice(0, 40) + ' ' + t.scrollWidth + '>' + t.parentElement.clientWidth)"""
    )
    # Every results tab is visible (none scrolled out of the tab list).
    hidden_tabs = pg.evaluate(
        """() => [...document.querySelectorAll('#side-panel [role=tablist] [role=tab]')].filter(t => {
          const l = t.parentElement.getBoundingClientRect(), b = t.getBoundingClientRect();
          return b.width > 0 && (b.right > l.right + 1 || b.left < l.left - 1); }).map(t => t.textContent)"""
    )
    page_scroll = pg.evaluate("document.documentElement.scrollWidth > window.innerWidth + 1")
    out = [f"{where}: side panel overflows: {bad}"] if bad else []
    if wide:
        out.append(f"{where}: tables wider than the side panel: {wide[:3]}")
    if hidden_tabs:
        out.append(f"{where}: results tabs out of sight: {hidden_tabs}")
    if page_scroll:
        out.append(f"{where}: horizontal page scroll")
    return out


# --- studies ----------------------------------------------------------------------------------

def expert_study(pg, base, csv_path, failures, shots) -> str | None:
    """Every Science v1 option on one synthetic demo study."""
    start_demo(pg, base, second=False)
    side = pg.locator("#side-panel")
    # Data review: coarser grid (faster), the organisation's SYNTHETIC field data held out.
    side.get_by_role("button", name="≈ 5 km").click()
    pg.wait_for_timeout(800)
    side.get_by_role("button", name=re.compile("^The organisation's field data")).click()
    side.get_by_role("button", name="Import a CSV file").click()
    pg.locator("#own-file").set_input_files(csv_path)
    pg.locator("#own-name").wait_for(timeout=10_000)
    pg.locator("#own-name").fill("SYNTHETIC survey (Lambert-93, generated by ui_smoke.py)")
    crs = pg.locator("#own-crs")
    if crs.evaluate("e => e.tagName") == "SELECT":
        crs.select_option("EPSG:2154")
    side.get_by_role("button", name="Import", exact=True).click()
    report = side.get_by_text("Import report")
    try:
        report.wait_for(timeout=15_000)
    except Exception:
        failures.append("own data: no import report after importing the synthetic CSV")
    t = side.inner_text()
    if "60 of 60 rows imported" not in t:
        failures.append("own data: the 60 synthetic Lambert-93 rows were not all imported")
    side.get_by_label("Use for: Virtualis demonstrandum").select_option(label="Virtualis demonstrandum (SYNTHETIC)")
    pg.wait_for_timeout(300)
    side.get_by_label(re.compile("^Sources per species: Virtualis demonstrandum")).select_option("validation")
    pg.wait_for_timeout(1500)
    shot(pg, shots, "expert-data")
    continue_(pg)
    # Variables: predictor group headings.
    t = side.inner_text()
    for g in ("Climate", "Terrain"):
        if g not in t:
            failures.append(f"variables step lacks the group heading '{g}'")
    continue_(pg)
    # Model: Expert options.
    side.get_by_role("switch", name="Expert mode").click()
    pg.wait_for_timeout(400)
    side.get_by_text("Ensemble: MaxEnt + random forest").click()
    pg.locator("#rf-trees").fill("100")
    side.get_by_text("Fixed — one feature set and one multiplier").click()
    for fs in ("L", "LQH", "LQHP", "H"):
        box = side.locator("label", has=pg.locator(f"span.font-mono:text-is('{fs}')")).get_by_role("checkbox")
        if box.count() and box.first.get_attribute("aria-checked") == "true":
            box.first.click()
    pg.locator("#beta").fill("1")
    pg.locator("#bg").fill("3000")
    side.get_by_text("Target group (sampling-bias correction)").click()
    pg.locator("#thin-km").fill("5")
    side.get_by_text("Spatial grid blocks sized by spatial autocorrelation (k folds)").click()
    choose_select(pg, "#null-models", "20")
    side.get_by_text("Area of applicability (AOA) layer").first.click()
    side.get_by_role("button", name="Add a scenario").click()
    pg.wait_for_timeout(300)
    side.get_by_text("All five (recommended)").click()  # one climate model only: faster
    first_gcm = side.locator("fieldset", has_text="Climate models (GCMs)").get_by_role("checkbox").nth(1)
    first_gcm.click()
    pg.wait_for_timeout(300)
    shot(pg, shots, "expert-model")
    if side.locator("[aria-invalid=true]").count():
        failures.append("model step: an expert field is invalid")
    continue_(pg)
    t = side.inner_text()
    for must in ("Ensemble", "1 future scenario", "SYNTHETIC"):
        if must not in t:
            failures.append(f"review step lacks '{must}'")
    shot(pg, shots, "expert-review")
    return launch_and_wait(pg, failures, "expert study")


def two_species_study(pg, base, failures) -> str | None:
    start_demo(pg, base, second=True)
    pg.locator("#side-panel").get_by_role("button", name="≈ 5 km").click()
    pg.wait_for_timeout(800)
    for _ in range(4):
        if pg.get_by_role("button", name="Launch study").count():
            break
        continue_(pg)
    return launch_and_wait(pg, failures, "two-species study")


# --- results ----------------------------------------------------------------------------------

def check_expert_results(pg, url, failures, shots) -> None:
    pg.goto(url + "?view=full&tab=summary")
    pg.wait_for_load_state("networkidle")
    pg.wait_for_timeout(1500)
    text = pg.locator("body").inner_text()
    for must in ["Relative habitat suitability", "not a probability", "SYNTHETIC", "EVIDENCE LEVEL"]:
        if must.lower() not in text.lower():
            failures.append(f"results page lacks '{must}'")
    failures.extend(results_wording("en", text, "results/summary"))
    shot(pg, shots, "ui-smoke-results")
    side = pg.locator("#side-panel")
    # Validation: null models and independent validation.
    side.get_by_role("tab", name="Validation").click()
    pg.wait_for_timeout(800)
    t = side.inner_text()
    for must in (L["en"]["null"], "better than random-location models", L["en"]["independent"]):
        if must not in t:
            failures.append(f"validation tab lacks '{must}'")
    failures.extend(results_wording("en", t, "results/validation"))
    failures.extend(panel_overflow(pg, "validation tab (1440 px)"))
    shot(pg, shots, "results-validation")
    # Map layers and legends: AOA hatching, sampling effort, future change.
    overlay = pg.get_by_role("button", name="Layers & legend")
    if overlay.count() and overlay.get_attribute("aria-expanded") == "false":
        overlay.click()
    if not pg.get_by_text(L["en"]["aoa"]).count():
        failures.append("map overlay lacks the AOA hatching option")
    else:
        pg.get_by_text(L["en"]["aoa"]).click()
        pg.wait_for_timeout(800)
        shot(pg, shots, "map-aoa")
    eff = pg.get_by_text(L["en"]["effort"])
    if not eff.count():
        failures.append("map overlay lacks the sampling-effort layer (target-group background)")
    else:
        eff.first.click()
        pg.wait_for_timeout(1200)
        legend = pg.locator("body").inner_text()
        if "records" not in legend.lower():
            failures.append("sampling-effort legend lacks its unit (records)")
        shot(pg, shots, "map-effort")
    period = pg.get_by_role("radiogroup", name="Period and scenario").get_by_role("radio")
    if period.count() < 2:
        failures.append("map overlay lacks the future period choice")
    else:
        period.nth(1).click()
        pg.wait_for_timeout(1500)
        if not pg.get_by_text(L["en"]["change"]).count():
            failures.append("future change layer not offered on the map")
        shot(pg, shots, "map-future")
    # Future tab.
    side.get_by_role("tab", name="Future").click()
    pg.wait_for_timeout(800)
    t = side.inner_text()
    for must in ("SYNTHETIC future scenarios", "Suitable now", "Newly suitable"):
        if must not in t:
            failures.append(f"future tab lacks '{must}'")
    failures.extend(results_wording("en", t, "results/future"))
    shot(pg, shots, "results-future")
    # Variables tab: layer names.
    side.get_by_role("tab", name="Variables").click()
    pg.wait_for_timeout(500)
    if "Synthetic temperature" not in side.inner_text():
        failures.append("variables tab lacks the predictor names")
    # Decision support: survey priority with the target-group effort surface.
    side.get_by_role("tab", name="Decision support").click()
    pg.wait_for_timeout(500)
    side.get_by_role("button", name="Compute survey priority").click()
    try:
        side.get_by_text("Candidate areas worth surveying").first.wait_for(timeout=60_000)
    except Exception:
        failures.append("survey priority did not finish")
    t = side.inner_text()
    if "Sampling measured by the target group's records per cell" not in t:
        failures.append("survey priority does not use the target-group effort surface")
    failures.extend(results_wording("en", t, "decision/priority"))
    shot(pg, shots, "decision-priority")


FOOTPRINT = {"type": "Polygon", "coordinates": [[[4.40, 44.80], [4.43, 44.80], [4.43, 44.82], [4.40, 44.82], [4.40, 44.80]]]}


def check_decision(pg, url, other_url, failures, shots, tmp) -> None:
    """Footprint, several species and comparison on the two-species study."""
    pg.goto(url + "?view=full&tab=decision&ds=footprint")
    pg.wait_for_load_state("networkidle")
    pg.wait_for_timeout(1200)
    side = pg.locator("#side-panel")
    path = os.path.join(tmp, "synthetic-footprint.geojson")
    with open(path, "w") as fh:
        json.dump({"type": "Feature", "properties": {"name": "SYNTHETIC footprint"}, "geometry": FOOTPRINT}, fh)
    side.get_by_label("Footprint name").fill("SYNTHETIC footprint")
    side.locator("input[type=file]").set_input_files(path)
    pg.wait_for_timeout(500)
    side.get_by_role("button", name="Overlay footprint").click()
    try:
        side.get_by_text("Share of footprint").wait_for(timeout=60_000)
    except Exception:
        failures.append("footprint overlay did not finish")
    t = side.inner_text()
    if "smaller than one model cell" not in t:
        failures.append("footprint smaller than a cell: warning not shown")
    if "Does not replace a field inventory" not in t:
        failures.append("footprint disclaimer missing")
    failures.extend(results_wording("en", t, "decision/footprint"))
    shot(pg, shots, "decision-footprint")
    # Several species.
    side.get_by_role("button", name="Several species", exact=True).click()
    pg.wait_for_timeout(400)
    side.get_by_role("button", name="Compute", exact=True).click()
    try:
        side.get_by_text("Cells per number of species with suitable conditions").wait_for(timeout=60_000)
    except Exception:
        failures.append("multi-species stack did not finish")
    t = side.inner_text()
    if "Stacked species maps overpredict" not in t:
        failures.append("stacking caveat (analysis warning) not shown")
    failures.extend(results_wording("en", t, "decision/richness"))
    shot(pg, shots, "decision-richness")
    # Comparison with the expert study (same species, area and resolution).
    side.get_by_role("button", name="Compare with…", exact=True).click()
    pg.wait_for_timeout(400)
    if other_url:
        pg.locator("#compare-with").click()
        pg.get_by_role("option").first.click()
        side.get_by_role("button", name="Compare", exact=True).click()
        try:
            side.get_by_text("A (this study)").or_(side.get_by_role("alert")).first.wait_for(timeout=60_000)
        except Exception:
            pass
        if not side.get_by_text("A (this study)").count():
            alert = side.get_by_role("alert")
            failures.append(f"comparison of two studies did not finish: {alert.first.inner_text() if alert.count() else 'no result'}")
        failures.extend(results_wording("en", side.inner_text(), "decision/compare"))
        shot(pg, shots, "decision-compare")


def check_languages(pg, expert_url, multi_url, failures, shots) -> None:
    """Labels of the new screens in FR and ES, no English server texts left, wording rules."""
    for lang in ("fr", "es"):
        set_lang(pg, lang)
        lab = L[lang]
        pg.goto(expert_url + "?view=full&tab=validation")
        pg.wait_for_load_state("networkidle")
        pg.wait_for_timeout(1500)
        side = pg.locator("#side-panel")
        t = side.inner_text()
        tabs = [x.inner_text().strip() for x in side.get_by_role("tab").all()]
        for want in lab["tabs"]:
            if want not in tabs:
                failures.append(f"[{lang}] results tab '{want}' missing (got {tabs})")
        for k in ("null", "independent"):
            if lab[k].lower() not in t.lower():
                failures.append(f"[{lang}] validation tab lacks '{lab[k]}'")
        failures.extend(results_wording(lang, t, "results/validation"))
        if not pg.get_by_text(lab["aoa"]).count():
            failures.append(f"[{lang}] AOA hatching option not translated")
        if not pg.get_by_text(lab["effort"]).count():
            failures.append(f"[{lang}] effort layer not translated")
        shot(pg, shots, f"results-validation-{lang}")
        side.get_by_role("tab", name=lab["tabs"][2]).click()
        pg.wait_for_timeout(500)
        t = side.inner_text()
        for e in ENGLISH_LEFTOVERS:
            if e in t:
                failures.append(f"[{lang}] variables tab shows English '{e}'")
        side.get_by_role("tab", name=lab["tabs"][5]).click()
        pg.wait_for_timeout(800)
        t = side.inner_text()
        for k in ("priority", "footprint", "compare"):
            if lab[k] not in t:
                failures.append(f"[{lang}] decision tab lacks '{lab[k]}'")
        for e in ENGLISH_LEFTOVERS:
            if e in t:
                failures.append(f"[{lang}] decision tab shows English '{e}'")
        failures.extend(results_wording(lang, t, "decision"))
        shot(pg, shots, f"decision-{lang}")
        pg.goto(multi_url + "?view=full&tab=decision&ds=richness")
        pg.wait_for_load_state("networkidle")
        pg.wait_for_timeout(1500)
        t = pg.locator("#side-panel").inner_text()
        if lab["several"] not in t:
            failures.append(f"[{lang}] decision tab lacks '{lab['several']}'")
        for e in ENGLISH_LEFTOVERS:
            if e in t:
                failures.append(f"[{lang}] multi-species section shows English '{e}'")
        pg.goto(multi_url + "?view=full&tab=decision&ds=footprint")
        pg.wait_for_load_state("networkidle")
        pg.wait_for_timeout(1500)
        t = pg.locator("#side-panel").inner_text()
        for e in ENGLISH_LEFTOVERS:
            if e in t:
                failures.append(f"[{lang}] footprint section shows English '{e}'")
        shot(pg, shots, f"decision-footprint-{lang}")
    set_lang(pg, "en")


def check_layouts(b, expert_url, failures, shots) -> None:
    """Tablet and phone: the panel (a bottom sheet on phones) never overflows horizontally."""
    for name, vp in (("tablet", {"width": 820, "height": 1180}), ("phone", {"width": 390, "height": 844})):
        ctx = b.new_context(viewport=vp, locale="en-US")
        pg = ctx.new_page()
        for tab in ("summary", "validation", "future", "decision"):
            pg.goto(expert_url + f"?view=full&tab={tab}")
            pg.wait_for_load_state("networkidle")
            pg.wait_for_timeout(1200)
            expand = pg.get_by_role("button", name="Expand panel")
            if expand.count():  # phone: the panel is a bottom sheet, expanded to read the tab
                expand.first.click()
                pg.wait_for_timeout(600)
            failures.extend(panel_overflow(pg, f"{tab} ({name} {vp['width']} px)"))
            shot(pg, shots, f"{name}-{tab}")
        ctx.close()


# --- help (unchanged checks, language in the submenu) -------------------------------------

def global_search_finds(pg, query: str) -> bool:
    """Ctrl K opens the top-bar search; typing part of a study name lists it."""
    pg.keyboard.press("Control+k")
    pg.keyboard.type(query)
    pg.wait_for_timeout(600)
    found = pg.locator("[data-global-search] [role=option]", has_text="Synthetic").count() > 0
    pg.keyboard.press("Escape")
    return found


def check_help(b, base: str, results_url: str | None, failures: list[str]) -> None:
    """Help panel and help pages. Uses a fresh context so localStorage starts empty."""
    ctx = b.new_context(viewport={"width": 1440, "height": 900}, locale="en-US")
    pg = ctx.new_page()
    pg.goto(base + "/")
    pg.wait_for_load_state("networkidle")
    # Every visit lands on the organisation's workspace (the study run above is listed there);
    # a draft being prepared is offered there, never entered silently.
    if not pg.locator("[data-workspace]").count():
        failures.append("a fresh browser does not land on the workspace")
    elif results_url and not pg.get_by_role("button", name=re.compile("^More actions for")).count():
        failures.append("the workspace does not list the study")
    elif results_url and not global_search_finds(pg, "synthetic"):
        failures.append("global search (Ctrl K) does not find the study")
    else:
        pg.get_by_role("button", name="New study").first.click()
        # A new study first asks for a setup path; the demo is offered on the species step.
        quick = pg.get_by_role("button", name="Quick").filter(has_text="Quick").first
        quick.wait_for(timeout=10_000)
        quick.click()
        pg.get_by_role("button", name="Try the synthetic demo").click()
        pg.goto(base + "/")
        pg.wait_for_load_state("networkidle")
        if not pg.get_by_role("button", name="Continue this study").count():
            failures.append("the workspace does not offer the draft being prepared")
        else:
            pg.get_by_role("button", name="Discard").click()
            if pg.get_by_role("button", name="Continue this study").count():
                failures.append("discarding the draft did not clear it")
    panel = pg.locator("[data-help-panel]")
    if results_url:
        pg.goto(results_url + "?view=full")
        pg.wait_for_load_state("networkidle")
        trig = pg.locator('[data-help-trigger="evidence/levels"]').first
        if not trig.count():
            failures.append("no contextual help trigger for evidence levels on results")
        else:
            trig.focus()
            pg.keyboard.press("Enter")
            panel.wait_for(state="visible", timeout=5000)
            pg.wait_for_timeout(400)
            t = panel.inner_text()
            for must in ["Supported", "≥ 50", "≥ 0.70", "Indicative", "≥ 0.40", "10 usable presence cells"]:
                if must not in t:
                    failures.append(f"evidence help lacks '{must}'")
            focused = pg.evaluate("document.activeElement && document.activeElement.id")
            if not str(focused).startswith("help-panel-evidence-levels"):
                failures.append(f"focus not moved to the evidence section (got {focused!r})")
            pg.keyboard.press("Escape")
            panel.wait_for(state="hidden", timeout=5000)
            back = pg.evaluate("document.activeElement && document.activeElement.getAttribute('data-help-trigger')")
            if back != "evidence/levels":
                failures.append(f"focus not returned to the help trigger after Esc (got {back!r})")
    for lang in ("fr", "es", "en"):
        # The interface language (help included) is chosen in the user menu's submenu.
        set_lang(pg, lang)
        pg.locator("[data-user-menu]").click()
        pg.locator("[data-help-button]").click()
        panel.wait_for(state="visible", timeout=5000)
        pg.wait_for_timeout(500)
        back_link = panel.locator("[data-help-back]")
        if back_link.count():
            back_link.first.click()  # article index
            pg.wait_for_timeout(200)
        listed = [a for a in ARTICLES if panel.locator(f'a[href="/help/{a}"]').count()]
        if len(listed) != len(ARTICLES):
            failures.append(f"help index in {lang} lists {len(listed)} of {len(ARTICLES)} articles: missing {sorted(set(ARTICLES) - set(listed))}")
        panel.locator('a[href="/help/map"]').first.click()
        pg.wait_for_timeout(300)
        t = panel.inner_text()
        if EXPECT[lang] not in t.lower():
            failures.append(f"help panel in {lang} lacks '{EXPECT[lang]}'")
        failures.extend(wording_failures(lang, t, "panel/map"))
        if "pending review" not in t and "attente de relecture" not in t and "pendiente de revisión" not in t:
            failures.append(f"help panel in {lang} lacks the review-status note")
        pg.keyboard.press("Escape")
    # Every article, every language, as full pages.
    for lang in ("en", "fr", "es"):
        pg.evaluate(f"localStorage.setItem('nichers.lang', '{lang}')")
        for a in ARTICLES:
            pg.goto(f"{base}/help/{a}")
            pg.locator("[data-help-article-title]").wait_for(timeout=5000)
            failures.extend(wording_failures(lang, pg.locator("main").inner_text(), f"/help/{a}"))
    ctx.close()
    # Phone: the panel is a bottom sheet.
    ctx = b.new_context(viewport={"width": 390, "height": 844}, locale="fr-FR")
    pg = ctx.new_page()
    pg.goto(base + "/")
    pg.wait_for_load_state("networkidle")
    pg.locator("[data-user-menu]").click()  # help lives in the user menu
    pg.locator("[data-help-button]").click()
    panel = pg.locator("[data-help-panel]")
    panel.wait_for(state="visible", timeout=5000)
    pg.wait_for_timeout(400)
    box = panel.bounding_box()
    if not box or box["width"] < 380 or box["y"] < 50:
        failures.append(f"help panel is not a bottom sheet on a phone viewport: {box}")
    if "Centre d'aide" not in panel.inner_text():
        failures.append("help language not detected from navigator.language (fr-FR)")
    ctx.close()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("base")
    ap.add_argument("--chromium", default=None, help="explicit chromium/headless-shell executable")
    ap.add_argument("--shots", default=None, help="directory for screenshots")
    a = ap.parse_args()
    if a.shots:
        os.makedirs(a.shots, exist_ok=True)
    failures: list[str] = []
    tmp = tempfile.mkdtemp(prefix="ui-smoke-")
    csv_path = os.path.join(tmp, "SYNTHETIC_survey_lambert93.csv")
    synthetic_survey(csv_path)

    def step(name, fn, *args):
        """Runs one part; an exception is a failure of that part, not the end of the run."""
        print(f"… {name}", flush=True)
        try:
            return fn(*args)
        except Exception as e:  # noqa: BLE001 - report and continue
            here = [f"line {t.lineno}" for t in traceback.extract_tb(e.__traceback__) if t.filename == __file__]
            failures.append(f"{name} ({', '.join(here[-2:])}): {type(e).__name__}: {str(e).splitlines()[0]}")
            if a.shots:
                try:
                    pg.screenshot(path=os.path.join(a.shots, "failure-" + re.sub(r"\W+", "-", name) + ".png"))
                except Exception:  # noqa: BLE001
                    pass
            return None

    with sync_playwright() as p:
        b = p.chromium.launch(headless=True, executable_path=a.chromium) if a.chromium else p.chromium.launch(headless=True)
        # English UI whatever the system locale: the checks below match English strings.
        pg = b.new_page(viewport={"width": 1440, "height": 900}, locale="en-US")
        errors: list[str] = []
        pg.on("console", lambda m: errors.append(m.text) if m.type == "error" and not any(x in m.text for x in BENIGN) else None)
        pg.on("pageerror", lambda e: errors.append(f"pageerror: {e}"))
        expert_url = step("expert study (every Science v1 option)", expert_study, pg, a.base, csv_path, failures, a.shots)
        if expert_url:
            step("expert study results", check_expert_results, pg, expert_url, failures, a.shots)
        multi_url = step("two-species study", two_species_study, pg, a.base, failures)
        if multi_url:
            step("decision support", check_decision, pg, multi_url, expert_url, failures, a.shots, tmp)
        if expert_url and multi_url:
            step("French and Spanish labels", check_languages, pg, expert_url, multi_url, failures, a.shots)
        if expert_url:
            step("tablet and phone layouts", check_layouts, b, expert_url, failures, a.shots)
        step("help", check_help, b, a.base, expert_url, failures)
        if errors:
            failures.append(f"console errors: {errors[:5]}")
        b.close()
    for f in failures:
        print("FAIL:", f)
    print("UI smoke:", "OK" if not failures else f"{len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
