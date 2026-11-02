#!/usr/bin/env python3
"""Hosted-mode UI smoke test (ADR-0105): invitation → account → synthetic demo run → quota →
delete → sign out → sign in. Development only (needs Playwright), against a throw-away instance:

    D=$(mktemp -d)
    NICHE_DATA_DIR=$D ./target/release/niche serve --port 8092 --auth accounts &
    NICHE_DATA_DIR=$D ./target/release/niche admin org create "Smoke test" --quota 2
    LINK=$(NICHE_DATA_DIR=$D ./target/release/niche admin --public-url http://127.0.0.1:8092 invite --org "Smoke test")
    uv run --with playwright python -I scripts/ui_smoke_hosted.py http://127.0.0.1:8092 "$LINK" [--chromium PATH] [--shots DIR]

The organisation must be new (quota 2, nothing used): the script checks the quota counter.
"""
import argparse
import re
import sys

from playwright.sync_api import sync_playwright

BENIGN = ("GL Driver Message", "script-src' was not explicitly set")
EMAIL = "smoke@example.org"
PASSWORD = "a long smoke-test password"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("base")
    ap.add_argument("invite", help="invitation link printed by `niche admin invite`")
    ap.add_argument("--chromium", default=None)
    ap.add_argument("--shots", default=None)
    a = ap.parse_args()
    failures: list[str] = []

    def check(cond: bool, msg: str) -> None:
        if not cond:
            failures.append(msg)

    def shot(pg, name: str) -> None:
        if a.shots:
            pg.screenshot(path=f"{a.shots}/hosted-{name}.png")

    with sync_playwright() as p:
        b = p.chromium.launch(headless=True, executable_path=a.chromium) if a.chromium else p.chromium.launch(headless=True)
        # English UI whatever the system locale: the checks below match English strings.
        pg = b.new_page(viewport={"width": 1440, "height": 900}, locale="en-US")
        errors: list[str] = []
        # 401s are expected while signed out (session check before the redirect to /login).
        pg.on(
            "console",
            lambda m: errors.append(m.text)
            if m.type == "error" and not any(x in m.text for x in BENIGN) and "401" not in m.text
            else None,
        )

        # 1. Signed out: the workspace is not shown, the sign-in page is.
        pg.goto(a.base + "/jobs")
        pg.wait_for_url("**/login?next=%2Fjobs", timeout=10_000)
        try:
            pg.get_by_role("heading", name="Sign in").wait_for(timeout=10_000)
        except Exception:
            failures.append("no sign-in page when signed out")
        check(pg.get_by_text("Studies").count() == 0, "workspace visible while signed out")
        shot(pg, "login")

        # 2. Invitation: the fragment is read and removed from the address bar.
        pg.goto(a.invite)
        pg.get_by_role("heading", name="Create your account").wait_for()
        pg.get_by_text("Smoke test").first.wait_for()
        check("#" not in pg.url, "invitation token left in the address bar")
        pg.get_by_label("Your name").fill("Smoke Tester")
        pg.get_by_label("E-mail").fill(EMAIL)
        pg.get_by_label("New password").fill(PASSWORD)
        pg.get_by_label("Repeat the password").fill(PASSWORD)
        shot(pg, "invite")
        pg.get_by_role("button", name="Create account").click()
        pg.wait_for_url(a.base + "/", timeout=10_000)
        pg.wait_for_load_state("networkidle")
        account = pg.get_by_role("button", name="Account:")
        check(account.count() == 1, "account menu missing after sign-up")
        check("0/2" in account.inner_text(), f"quota counter should read 0/2: {account.inner_text()!r}")

        # 3. A used invitation cannot be reused.
        pg2 = b.new_page(locale="en-US")
        pg2.goto(a.invite)
        pg2.get_by_text("This link cannot be used").wait_for(timeout=10_000)
        pg2.close()

        # 4. Synthetic demo run through the wizard, as in ui_smoke.py.
        pg.get_by_role("button", name="Try the synthetic demo").click()
        pg.locator("#side-panel").wait_for(timeout=10_000)  # the wizard (map layout) is open
        # A new study first asks for a setup path (Quick: 3 steps, Expert: every step).
        choice = pg.get_by_role("button", name="Quick", exact=False).filter(has_text="Quick").first
        if choice.count():
            choice.click()
            pg.wait_for_timeout(800)
        else:
            failures.append("no setup path choice (Quick / Expert)")
        for _ in range(8):
            if pg.get_by_role("button", name="Launch study").count():
                break
            footer = [x for x in pg.locator("#side-panel button").all() if x.is_visible()]
            if not footer:
                break
            footer[-1].click()
            pg.wait_for_timeout(1500)
        launch = pg.get_by_role("button", name="Launch study")
        if not launch.count():
            failures.append("could not reach the launch step")
        else:
            launch.click()
            for _ in range(120):
                if "/results" in pg.url:
                    break
                pg.wait_for_timeout(1000)
            pg.wait_for_load_state("networkidle")
            text = pg.locator("body").inner_text().lower()
            check("relative habitat suitability" in text, "results page lacks the suitability wording")
            check("synthetic" in text, "results page lacks the synthetic label")
            shot(pg, "results")

        # 5. Quota counted (1 species modelled) and shown.
        pg.reload()
        pg.wait_for_load_state("networkidle")
        account = pg.get_by_role("button", name="Account:")
        check("1/2" in account.inner_text(), f"quota counter should read 1/2: {account.inner_text()!r}")
        account.click()
        pg.get_by_text("Modelled species").wait_for()
        shot(pg, "account-menu")
        pg.keyboard.press("Escape")

        # 6. Delete the run (confirmation names it); the quota is not refunded.
        pg.goto(a.base + "/jobs")
        pg.wait_for_load_state("networkidle")
        pg.get_by_role("button", name=re.compile("^More actions for")).first.click()
        pg.get_by_role("menuitem", name="Delete").click()
        pg.get_by_role("alertdialog").wait_for()
        shot(pg, "delete-dialog")
        pg.get_by_role("button", name="Delete study").click()
        pg.get_by_text("No studies yet").wait_for(timeout=10_000)
        check("1/2" in pg.get_by_role("button", name="Account:").inner_text(), "deleting refunded the quota")

        # 7. Sign out, then sign in again.
        pg.get_by_role("button", name="Account:").click()
        pg.get_by_role("menuitem", name="Sign out").click()
        pg.wait_for_url("**/login**", timeout=10_000)
        pg.goto(a.base + "/")
        pg.wait_for_url("**/login**", timeout=10_000)
        pg.get_by_label("E-mail").fill(EMAIL)
        pg.get_by_label("Password").fill("wrong password value")
        pg.get_by_role("button", name="Sign in").click()
        pg.get_by_text("e-mail or password incorrect").wait_for(timeout=10_000)
        pg.get_by_label("Password").fill(PASSWORD)
        pg.get_by_role("button", name="Sign in").click()
        pg.wait_for_url(a.base + "/", timeout=10_000)
        # Wait for the shell to render: count() alone races the first paint after sign-in.
        try:
            pg.get_by_role("button", name="Account:").wait_for(timeout=10_000)
        except Exception:
            pass
        check(pg.get_by_role("button", name="Account:").count() == 1, "not signed in again")

        # The expected 401 on the failed sign-in is not a UI error.
        errors = [e for e in errors if "status of 401" not in e]
        if errors:
            failures.append(f"console errors: {errors[:5]}")
        b.close()
    for f in failures:
        print("FAIL:", f)
    print("Hosted UI smoke:", "OK" if not failures else f"{len(failures)} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
