//! Natural Solutions administration (ADR-0106, ADR-0112, ADR-0113): organisations, accounts,
//! usage, activity, system state, feedback and the audit trail.

use super::*;

/// Record a change in the audit trail (ADR-0113) and log it (ids only). A failed write is
/// logged, not returned: the change itself is already done.
pub(crate) async fn audit(s: &AppState, ev: AuditEvent) {
    tracing::info!(
        action = ev.action.as_str(),
        actor_id = ?ev.actor_id,
        org_id = ?ev.org_id,
        user_id = ?ev.user_id,
        job_id = ?ev.job_id,
        "audit"
    );
    let e = s.engine.clone();
    if let Err(err) = blocking(move || e.store().audit(&ev)).await {
        tracing::error!(error = %err.0, "audit trail not written");
    }
}

pub(crate) fn first_access_this_hour(s: &AppState, user: &str, job: &str) -> bool {
    let Ok(mut m) = s.admin_access.lock() else {
        return true;
    };
    m.retain(|_, t| t.elapsed() < Duration::from_secs(3600));
    let key = (user.to_string(), job.to_string());
    if m.contains_key(&key) {
        return false;
    }
    // Not refreshed on later requests, so a long session is logged again every hour.
    m.insert(key, Instant::now());
    true
}

/// The signed-in administrator, or 403 (404 in local mode, which has no accounts).
pub(crate) fn admin(s: &AppState, ctx: &Ctx) -> ApiResult<Principal> {
    accounts_only(s)?;
    match &ctx.principal {
        Some(p) if p.is_admin() => Ok(p.clone()),
        _ => Err(ApiError(AppError::Forbidden(
            "administration is reserved to Natural Solutions administrators".into(),
        ))),
    }
}

pub(crate) fn json<T: serde::Serialize>(v: &T) -> Json<serde_json::Value> {
    Json(serde_json::to_value(v).unwrap_or_default())
}

pub(crate) async fn admin_orgs(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(&blocking(move || e.store().org_overview()).await?))
}

/// One organisation's overview (its page in the console).
pub(crate) async fn admin_org(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(org): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(
        &blocking(move || e.store().org_overview_of(&org)).await?,
    ))
}

pub(crate) async fn admin_create_org(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<CreateOrgRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let org = blocking(move || e.store().org_create(&r.name, r.monthly_quota)).await?;
    let ev = AuditEvent::web(&p, A::OrgCreated)
        .org(&org.id)
        .detail(serde_json::json!({"quota": org.monthly_quota}));
    audit(&s, ev).await;
    Ok((StatusCode::CREATED, json(&org)))
}

pub(crate) async fn admin_update_org(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(org): Path<String>,
    Json(r): Json<UpdateOrgRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let p = admin(&s, &ctx)?;
    if r.disabled == Some(true) && (org == p.org_id || org.eq_ignore_ascii_case(&p.org_name)) {
        return Err(ApiError(AppError::Conflict(
            "you cannot disable your own organisation (you would lock yourself out)".into(),
        )));
    }
    let e = s.engine.clone();
    let (before, updated) = blocking(move || {
        let st = e.store();
        let before = st.org(&org)?;
        let mut o = before.clone();
        if r.disabled == Some(true) && o.is_operator {
            return Err(AppError::Conflict(
                "the operator organisation cannot be disabled from the web".into(),
            ));
        }
        if let Some(q) = r.monthly_quota {
            o = st.org_set_quota(&o.id, q)?;
        }
        if let Some(d) = r.disabled {
            o = st.org_set_disabled(&o.id, d)?;
        }
        Ok((before, o))
    })
    .await?;
    if before.monthly_quota != updated.monthly_quota {
        let ev = AuditEvent::web(&p, A::OrgQuotaChanged)
            .org(&updated.id)
            .detail(serde_json::json!({"from": before.monthly_quota, "to": updated.monthly_quota}));
        audit(&s, ev).await;
    }
    match (before.disabled_at.is_some(), updated.disabled_at.is_some()) {
        (false, true) => audit(&s, AuditEvent::web(&p, A::OrgDisabled).org(&updated.id)).await,
        (true, false) => audit(&s, AuditEvent::web(&p, A::OrgEnabled).org(&updated.id)).await,
        _ => {}
    }
    Ok(json(&updated))
}

pub(crate) async fn admin_invites(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(org): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(&blocking(move || e.store().open_invites(&org)).await?))
}

pub(crate) async fn admin_create_invite(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(org): Path<String>,
) -> ApiResult<Response> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let (token, org_id) =
        blocking(move || Ok((e.store().invite(&org)?, e.store().org(&org)?.id))).await?;
    audit(&s, AuditEvent::web(&p, A::InviteCreated).org(org_id)).await;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        json(&LinkCreated {
            path: format!("/invite#{token}"),
            days_valid: niche_app::accounts::INVITE_DAYS,
        }),
    )
        .into_response())
}

pub(crate) async fn admin_revoke_invite(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let org_id = blocking(move || {
        let org_id = e.store().invite_org(&id)?;
        e.store().revoke_link(&id)?;
        Ok(org_id)
    })
    .await?;
    let mut ev = AuditEvent::web(&p, A::InviteRevoked);
    ev.org_id = org_id;
    audit(&s, ev).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(crate) struct UsersQ {
    /// Organisation id or name.
    org: Option<String>,
    q: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

/// Accounts, filtered and paged on the server (ADR-0113).
pub(crate) async fn admin_users(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<UsersQ>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    let page = blocking(move || {
        let st = e.store();
        let org = q.org.map(|o| st.org(&o)).transpose()?.map(|o| o.id);
        st.user_search(&UserFilter {
            org,
            q: q.q,
            offset: q.offset,
            limit: q.limit.map(|l| l.min(MAX_USER_PAGE)),
        })
    })
    .await?;
    Ok(json(&page))
}

/// Accounts per page at most (the console asks for one organisation's members in one go).
const MAX_USER_PAGE: usize = 500;

/// One account (its page in the console).
pub(crate) async fn admin_user(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(&blocking(move || e.store().user_info(&id)).await?))
}

pub(crate) async fn admin_disable_user(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<UserDisableRequest>,
) -> ApiResult<StatusCode> {
    let p = admin(&s, &ctx)?;
    if r.disabled && r.email.trim().eq_ignore_ascii_case(&p.email) {
        return Err(ApiError(AppError::Conflict(
            "you cannot disable your own account".into(),
        )));
    }
    let e = s.engine.clone();
    let (email, disabled) = (r.email.clone(), r.disabled);
    let account = blocking(move || {
        e.store().user_set_disabled(&email, disabled)?;
        e.store().account_ref(&email)
    })
    .await?;
    let action = if r.disabled {
        A::AccountDisabled
    } else {
        A::AccountEnabled
    };
    audit(&s, AuditEvent::web(&p, action).account(&account)).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Erase an account (GDPR). The e-mail must be typed twice, so a misclick cannot erase it.
pub(crate) async fn admin_delete_user(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<UserDeleteRequest>,
) -> ApiResult<StatusCode> {
    let p = admin(&s, &ctx)?;
    if !r.email.trim().eq_ignore_ascii_case(r.confirm_email.trim()) {
        return Err(ApiError(AppError::Invalid(
            "type the account's e-mail again to confirm the deletion".into(),
        )));
    }
    if r.email.trim().eq_ignore_ascii_case(&p.email) {
        return Err(ApiError(AppError::Conflict(
            "you cannot delete your own account".into(),
        )));
    }
    let e = s.engine.clone();
    let email = r.email.clone();
    let account = blocking(move || {
        let account = e.store().account_ref(&email)?;
        e.store().user_delete(&email)?;
        Ok(account)
    })
    .await?;
    audit(&s, AuditEvent::web(&p, A::AccountDeleted).account(&account)).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_reset_user(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<UserRequest>,
) -> ApiResult<Response> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let email = r.email.clone();
    let (token, account) = blocking(move || {
        let token = e.store().reset_link(&email)?;
        Ok((token, e.store().account_ref(&email)?))
    })
    .await?;
    audit(
        &s,
        AuditEvent::web(&p, A::ResetLinkCreated).account(&account),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "no-store")],
        json(&LinkCreated {
            path: format!("/reset#{token}"),
            days_valid: niche_app::accounts::RESET_DAYS,
        }),
    )
        .into_response())
}

#[derive(Deserialize)]
pub(crate) struct MonthQ {
    month: Option<String>,
}

pub(crate) fn valid_month(m: Option<&str>) -> ApiResult<()> {
    let ok = m.is_none_or(|m| {
        m.len() == 7
            && m.as_bytes()[4] == b'-'
            && m[..4].chars().all(|c| c.is_ascii_digit())
            && m[5..].parse::<u8>().is_ok_and(|n| (1..=12).contains(&n))
    });
    if ok {
        Ok(())
    } else {
        Err(ApiError(AppError::Invalid("month must be YYYY-MM".into())))
    }
}

pub(crate) async fn admin_usage(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<MonthQ>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    valid_month(q.month.as_deref())?;
    let e = s.engine.clone();
    Ok(json(
        &blocking(move || e.store().usage_summary(q.month.as_deref())).await?,
    ))
}

pub(crate) fn csv_field(f: &str) -> String {
    if f.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", f.replace('"', "\"\""))
    } else {
        f.to_string()
    }
}

/// One line per modelled species (for invoicing), same columns as `niche admin usage`.
pub(crate) async fn admin_usage_csv(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<MonthQ>,
) -> ApiResult<Response> {
    admin(&s, &ctx)?;
    valid_month(q.month.as_deref())?;
    let e = s.engine.clone();
    let month = q.month.clone();
    let rows = blocking(move || e.store().usage(None, month.as_deref())).await?;
    let mut out = String::from("organisation,month,job_id,species_key,counted_at\n");
    for r in rows {
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_field(&r.org_name),
            r.month,
            r.job_id,
            r.species_key,
            r.at
        ));
    }
    let name = format!(
        "nichers-usage-{}.csv",
        q.month.as_deref().unwrap_or("all-months")
    );
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        out,
    )
        .into_response())
}

#[derive(Deserialize)]
pub(crate) struct ActivityQ {
    /// Organisation id or name: its studies only (its page in the console).
    org: Option<String>,
}

pub(crate) async fn admin_activity(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<ActivityQ>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    let mut rows = blocking(move || {
        let st = e.store();
        let org = q.org.map(|o| st.org(&o)).transpose()?.map(|o| o.id);
        st.activity(100, org.as_deref())
    })
    .await?;
    for r in rows
        .iter_mut()
        .filter(|r| r.status == niche_app::dto::JobStatus::Queued)
    {
        r.gbif_wait = s.engine.gbif_wait(&r.id);
    }
    Ok(json(&rows))
}

pub(crate) async fn admin_system(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    let mut st = blocking(move || e.system_status()).await?;
    st.build = s.opts.build.clone();
    st.volume_backup = s.opts.volume_backup.clone();
    Ok(json(&st))
}

pub(crate) async fn admin_metrics(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<MonthQ>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let month = q
        .month
        .filter(|m| m.len() == 7)
        .unwrap_or_else(niche_app::accounts::current_month);
    let e = s.engine.clone();
    Ok(json(&blocking(move || e.store().metrics(&month)).await?))
}

pub(crate) async fn admin_set_manager(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<UserManagerRequest>,
) -> ApiResult<StatusCode> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let (email, manager) = (r.email.clone(), r.manager);
    let account = blocking(move || {
        e.store().user_set_manager(&email, manager)?;
        e.store().account_ref(&email)
    })
    .await?;
    let action = if r.manager {
        A::ManagerGranted
    } else {
        A::ManagerRemoved
    };
    audit(&s, AuditEvent::web(&p, action).account(&account)).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Make a member of the operator organisation an administrator (ADR-0112). The administrator
/// re-enters their password, so a stolen session cannot create administrators; failures count
/// as failed sign-ins of that account. Removing the flag stays on the command line.
pub(crate) async fn admin_grant_admin(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Json(r): Json<GrantAdminRequest>,
) -> ApiResult<Response> {
    let p = admin(&s, &ctx)?;
    let key = p.email.to_lowercase();
    if throttled(&s, &key) {
        let body = serde_json::json!({"type": "about:blank", "title": "Too many attempts", "status": 429, "detail": "too many wrong passwords for this account; try again in 15 minutes"});
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body.to_string(),
        )
            .into_response());
    }
    let e = s.engine.clone();
    let (admin_id, password) = (p.user_id.clone(), r.password);
    let ok = blocking(move || e.store().check_password_of(&admin_id, &password)).await?;
    record_failure(&s, &key, !ok);
    if !ok {
        tracing::warn!(admin_id = %p.user_id, "administrator grant refused: wrong password");
        return Err(ApiError(AppError::Forbidden(
            "your password is not correct".into(),
        )));
    }
    let e = s.engine.clone();
    let email = r.email;
    let account = blocking(move || {
        e.store().user_set_admin(&email, true)?;
        e.store().account_ref(&email)
    })
    .await?;
    tracing::warn!(admin_id = %p.user_id, user_id = %account.user_id, "administrator role granted");
    audit(&s, AuditEvent::web(&p, A::AdminGranted).account(&account)).await;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
pub(crate) struct AuditQ {
    org: Option<String>,
    user: Option<String>,
    before: Option<i64>,
    limit: Option<usize>,
}

/// The audit trail, newest first, one page at a time (ADR-0113).
pub(crate) async fn admin_audit(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Query(q): Query<AuditQ>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    let f = AuditFilter {
        org: q.org,
        user: q.user,
        before: q.before,
        limit: q.limit,
    };
    Ok(json(&blocking(move || e.store().audit_list(&f)).await?))
}

pub(crate) async fn admin_feedback(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
) -> ApiResult<Json<serde_json::Value>> {
    admin(&s, &ctx)?;
    let e = s.engine.clone();
    Ok(json(
        &blocking(move || e.store().feedback_list(None)).await?,
    ))
}

pub(crate) async fn admin_feedback_status(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
    Json(r): Json<FeedbackStatusRequest>,
) -> ApiResult<StatusCode> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let id2 = id.clone();
    blocking(move || e.store().feedback_set_status(&id2, r.status)).await?;
    tracing::info!(admin_id = %p.user_id, feedback_id = %id, status = r.status.as_str(), "feedback updated");
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn admin_feedback_delete(
    State(s): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let p = admin(&s, &ctx)?;
    let e = s.engine.clone();
    let id2 = id.clone();
    blocking(move || e.store().feedback_delete(&id2)).await?;
    tracing::info!(admin_id = %p.user_id, feedback_id = %id, "feedback deleted");
    Ok(StatusCode::NO_CONTENT)
}
