# ADR-0111 — User feedback, forwarded to the operator's issue tracker

- Status: Accepted (2026-10-19). Product decisions (session of 2026-10-19):
  during the alpha, giving feedback must be very easy (text, images or voice); administrators see
  all of it, an organisation sees its own; each item becomes a GitLab issue so it can be processed
  automatically; **neither users nor administrators see that GitLab exists** — it is an operator
  setting of the deployment.
- Implementation: `crates/niche-app/src/feedback.rs`, `store.rs` (schema v8), `accounts.rs`
  (`user_delete`), `crates/niche-data/src/gitlab.rs`, `crates/niche-server/src/lib.rs`
  (`/feedback*`, `/admin/feedback*`, `/org/feedback`, `forward_loop`), `crates/niche-cli/src/app_cmd.rs`
  (`NICHE_FEEDBACK_GITLAB_*`), `web/src/features/feedback/`, `AdminPage.tsx` (tab), `OrgPage.tsx`.

## Decision

1. **One button on every signed-in page** ("Feedback" / « Votre avis » / «Opinión», top bar and the
   account, organisation and administration headers). It opens a side panel: an optional kind
   (problem, idea, something else), a message, and one-click *Capture screen* (browser screen
   capture, one frame, WebP), *Add image* (also paste with Ctrl+V or drop) and *Record voice*
   (MediaRecorder, up to 5 minutes). A message, an attachment, or both are enough. The draft
   survives closing the panel. The page, language, screen size and browser are attached and the
   server adds its own build; the panel says so.
2. **Storage.** `feedback` and `feedback_files` tables in the SQLite database (attachments as
   BLOBs), so database copies, migrations and erasure cover them. Limits: 10 000 characters,
   6 attachments, 12 MB decoded per item; images PNG/JPEG/WebP/GIF, audio WebM/Ogg/MP4/MPEG/WAV
   (no SVG or HTML: attachments are served from the application's origin, with
   `Content-Security-Policy: sandbox`). The upload is base64 inside JSON, which keeps the CSRF
   rule "state-changing requests are JSON" (ADR-0105) without exceptions.
3. **Who reads it.** NS administrators: every organisation's, in Administration → Feedback; they
   mark items as dealt with (or reopen) and may delete one (spam, personal data sent by mistake).
   Organisation managers: their organisation's, read-only, in My organisation. Attachments: the
   author, their managers and administrators. Hosted mode only (local mode has nobody to read it).
4. **Forwarding (operator only).** With `NICHE_FEEDBACK_GITLAB_URL`, `_PROJECT` and `_TOKEN` set,
   each item is sent in the background right after it is stored: attachments to the project's
   uploads, then an issue titled `[Problem|Idea|Feedback] <first line>`, labelled
   `NICHE_FEEDBACK_GITLAB_LABELS` (default `feedback`) plus `feedback::<kind>`, ending with the
   marker `<!-- niche-feedback:<id> -->` for automation. Failures are recorded on the item and
   retried every 15 minutes; a claim column prevents two workers from opening the same issue.
   The issue URL is only written to the server log. No API field, page or message mentions the
   tracker.
5. **The user's text cannot act on GitLab.** It is placed in a fenced block longer than any `~`
   run it contains, and context values in inline code, so quick actions (`/close`) and mentions
   (`@all`) are inert. The issue names the organisation, not the author: the author is visible
   in the administration through the reference.
6. **Erasure.** Erasing an account (ADR-0106) keeps its feedback for the organisation and removes
   the link to the author. Issues already opened are outside NicheRS; they carry no e-mail or name.

## Consequences

- One more table pair in the database; screenshots and recordings make it grow (bounded by the
  per-item limit). Old feedback is not purged automatically; administrators delete it.
- The GitLab token needs scope `api` on the project (role Reporter is enough for issues and
  uploads). It is a deployment secret like the GBIF password.
- Voice recordings are not transcribed; whoever processes the issue listens to them.
