//! Admin-Bereich unter `/admin`.
//!
//! Login mit Benutzername und Passwort (unabhängig von den Kiosk-PINs).
//! Alle Seiten außer dem Login verlangen eine gültige Admin-Sitzung; das
//! erledigt der Extraktor [`AdminSession`] – ein Handler, der ihn als Parameter
//! hat, wird ohne Anmeldung gar nicht erst aufgerufen.

use std::collections::{BTreeMap, HashMap};

use askama::Template;
use axum::{
    Form, Router,
    extract::{DefaultBodyLimit, FromRequestParts, Multipart, Path, Query, State},
    http::{HeaderValue, header, request::Parts},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{CookieJar, SameSite};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use sqlx::{QueryBuilder, Sqlite};

use super::{AppError, AppState, PageCtx, is_local_path, original_path, percent_encode, render};
use crate::{
    admins::{self, Admin},
    articles::{self, Article, ArticleError, ArticleInput},
    config::Config,
    db,
    i18n::{Language, Translator},
    ledger::{self, LedgerError, TxKind},
    money,
    sessions::{self, Owner},
    users::{self, User, UserError},
};

const ADMIN_COOKIE: &str = "admin_session";
/// Einträge pro Seite in der Buchungsübersicht
const PAGE_SIZE: i64 = 100;

fn admin_ttl() -> chrono::Duration {
    chrono::Duration::hours(8)
}

pub fn routes(config: &Config) -> Router<AppState> {
    // Bild-Uploads brauchen ein höheres Limit als normale Formulare
    let upload_limit = config.max_image_bytes + 64 * 1024;
    Router::new()
        .route("/", get(dashboard))
        .route("/login", get(login_form).post(login))
        .route("/logout", post(logout))
        .route("/articles", get(article_list))
        .route("/articles/new", get(article_new).post(article_create))
        .route("/articles/{id}", get(article_edit).post(article_update))
        .route(
            "/articles/{id}/image",
            post(article_image_upload).layer(DefaultBodyLimit::max(upload_limit)),
        )
        .route("/articles/{id}/image/delete", post(article_image_delete))
        .route("/articles/{id}/move/{dir}", post(article_move))
        .route("/users", get(user_list).post(user_create))
        .route("/users/{id}", get(user_edit).post(user_update))
        .route("/users/{id}/pin", post(user_pin))
        .route("/users/{id}/correction", post(user_correction))
        .route("/transactions", get(transaction_list))
        .route("/transactions.csv", get(transaction_csv))
        .route("/transactions/{id}/reverse", post(transaction_reverse))
        .route("/stats", get(stats))
        .route("/admins", get(admin_list).post(admin_create))
        .route("/admins/password", post(admin_password))
        .route("/admins/{id}/toggle", post(admin_toggle))
}

// ---------------------------------------------------------------------------
// Anmeldung
// ---------------------------------------------------------------------------

/// Der angemeldete Admin. Als Handler-Parameter erzwingt er die Anmeldung.
pub struct AdminSession {
    pub admin: Admin,
}

impl FromRequestParts<AppState> for AdminSession {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let login_redirect = || {
            let next = original_path(parts);
            Redirect::to(&format!("/admin/login?next={}", percent_encode(&next))).into_response()
        };
        let Some(token) = jar.get(ADMIN_COOKIE).map(|c| c.value().to_string()) else {
            return Err(login_redirect());
        };
        let owner = sessions::validate(&state.pool, &token, admin_ttl())
            .await
            .map_err(|e| AppError::from(e).into_response())?;
        let Some(Owner::Admin(id)) = owner else {
            return Err(login_redirect());
        };
        match admins::get(&state.pool, id).await {
            Ok(Some(admin)) if admin.active => Ok(AdminSession { admin }),
            Ok(_) => Err(login_redirect()),
            Err(e) => Err(AppError::from(e).into_response()),
        }
    }
}

impl PageCtx {
    fn admin(mut self, session: &AdminSession) -> Self {
        self.admin_name = Some(session.admin.username.clone());
        self
    }
}

#[derive(Template)]
#[template(path = "admin/login.html")]
struct LoginTemplate {
    ctx: PageCtx,
    next: String,
    username: String,
    error: Option<String>,
}

#[derive(Deserialize)]
struct NextQuery {
    next: Option<String>,
}

async fn login_form(ctx: PageCtx, Query(q): Query<NextQuery>) -> Result<Response, AppError> {
    let next = q
        .next
        .filter(|n| is_local_path(n))
        .unwrap_or_else(|| "/admin".into());
    Ok(render(&LoginTemplate {
        ctx,
        next,
        username: String::new(),
        error: None,
    })?
    .into_response())
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    next: Option<String>,
}

async fn login(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let next = form
        .next
        .filter(|n| is_local_path(n))
        .unwrap_or_else(|| "/admin".into());
    match admins::login(&state.pool, &form.username, &form.password).await? {
        Some(admin) => {
            tracing::info!("Admin „{}“ angemeldet", admin.username);
            let token = sessions::create(&state.pool, Owner::Admin(admin.id), admin_ttl()).await?;
            let jar = jar.add(ctx.cookie(ADMIN_COOKIE, token, SameSite::Strict));
            Ok((jar, Redirect::to(&next)).into_response())
        }
        None => {
            tracing::warn!("Fehlgeschlagener Admin-Login für „{}“", form.username);
            // Kleine Bremse gegen Durchprobieren
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let error = Some(ctx.t.t("admin-login-failed"));
            Ok(render(&LoginTemplate {
                ctx,
                next,
                username: form.username,
                error,
            })?
            .into_response())
        }
    }
}

async fn logout(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
) -> Result<Response, AppError> {
    if let Some(c) = jar.get(ADMIN_COOKIE) {
        sessions::delete(&state.pool, c.value()).await?;
    }
    let jar = jar.remove(ctx.cookie(ADMIN_COOKIE, String::new(), SameSite::Strict));
    Ok((jar, Redirect::to("/admin/login")).into_response())
}

// ---------------------------------------------------------------------------
// Übersicht
// ---------------------------------------------------------------------------

/// Kennzahlen über alle Guthaben.
#[derive(Debug, Default, sqlx::FromRow)]
struct BalanceSummary {
    user_count: i64,
    total_cents: i64,
    credit_cents: i64,
    debt_cents: i64,
}

async fn balance_summary(state: &AppState) -> Result<BalanceSummary, AppError> {
    Ok(sqlx::query_as(
        "SELECT COUNT(*) AS user_count,
                COALESCE(SUM(balance_cents), 0) AS total_cents,
                COALESCE(SUM(CASE WHEN balance_cents > 0 THEN balance_cents END), 0) AS credit_cents,
                COALESCE(SUM(CASE WHEN balance_cents < 0 THEN balance_cents END), 0) AS debt_cents
         FROM users",
    )
    .fetch_one(&state.pool)
    .await?)
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct DashboardTemplate {
    ctx: PageCtx,
    summary: BalanceSummary,
    month_revenue_cents: i64,
    month_deposits_cents: i64,
    recent: Vec<TxRow>,
    mismatches: Vec<ledger::BalanceMismatch>,
}

async fn dashboard(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let summary = balance_summary(&state).await?;
    let tz = state.config.timezone;
    let today = Utc::now().with_timezone(&tz).date_naive();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let since = local_day_start(tz, month_start);
    let (month_revenue_cents, month_deposits_cents): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(-SUM(CASE WHEN article_id IS NOT NULL AND kind IN ('purchase','reversal')
                                   THEN amount_cents END), 0),
                COALESCE(SUM(CASE WHEN kind = 'deposit' THEN amount_cents
                                  WHEN kind = 'reversal' AND article_id IS NULL AND admin_id IS NULL
                                  THEN amount_cents END), 0)
         FROM transactions WHERE created_at >= ?",
    )
    .bind(since)
    .fetch_one(&state.pool)
    .await?;
    let recent = query_transactions(&state, &TxFilter::default(), Some(15), 0).await?;
    let mismatches = ledger::verify_balances(&state.pool).await?;
    Ok(render(&DashboardTemplate {
        ctx,
        summary,
        month_revenue_cents,
        month_deposits_cents,
        recent,
        mismatches,
    })?
    .into_response())
}

// ---------------------------------------------------------------------------
// Artikel
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "admin/articles.html")]
struct ArticleListTemplate {
    ctx: PageCtx,
    articles: Vec<Article>,
}

async fn article_list(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let articles = articles::list_all(&state.pool).await?;
    Ok(render(&ArticleListTemplate { ctx, articles })?.into_response())
}

/// Eine Zeile „Name in Sprache X“ im Artikelformular.
struct TranslationField {
    lang: Language,
    value: String,
}

#[derive(Template)]
#[template(path = "admin/article_edit.html")]
struct ArticleEditTemplate {
    ctx: PageCtx,
    /// `None` = neuer Artikel
    id: Option<i64>,
    name: String,
    price: String,
    sort_order: i64,
    active: bool,
    barcodes: String,
    translations: Vec<TranslationField>,
    image_url: Option<String>,
    error: Option<String>,
    saved: bool,
    max_image_kb: usize,
}

/// Alle Sprachen außer der Standardsprache bekommen ein eigenes Namensfeld.
fn translation_fields(state: &AppState, existing: &[(String, String)]) -> Vec<TranslationField> {
    state
        .locales
        .languages()
        .iter()
        .filter(|l| l.code != state.locales.default_language())
        .map(|l| TranslationField {
            lang: l.clone(),
            value: existing
                .iter()
                .find(|(code, _)| *code == l.code)
                .map(|(_, n)| n.clone())
                .unwrap_or_default(),
        })
        .collect()
}

async fn article_new(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let next_sort: i64 =
        sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) + 10 FROM articles")
            .fetch_one(&state.pool)
            .await?;
    Ok(render(&ArticleEditTemplate {
        ctx,
        id: None,
        name: String::new(),
        price: String::new(),
        sort_order: next_sort,
        active: true,
        barcodes: String::new(),
        translations: translation_fields(&state, &[]),
        image_url: None,
        error: None,
        saved: false,
        max_image_kb: state.config.max_image_bytes / 1024,
    })?
    .into_response())
}

#[derive(Deserialize)]
struct SavedQuery {
    saved: Option<String>,
    error: Option<String>,
}

async fn article_edit(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Path(id): Path<i64>,
    Query(q): Query<SavedQuery>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let article = articles::get(&state.pool, id, "")
        .await?
        .ok_or(AppError::NotFound)?;
    let translations = articles::translations(&state.pool, id).await?;
    let barcodes = articles::barcodes(&state.pool, id).await?.join("\n");
    let error = match q.error.as_deref() {
        Some("image-type") => Some(ctx.t.t("error-image-type")),
        Some("image-size") => Some(ctx.t.tn(
            "error-image-size",
            "kb",
            (state.config.max_image_bytes / 1024) as i64,
        )),
        Some(_) => Some(ctx.t.t("error-internal")),
        None => None,
    };
    Ok(render(&ArticleEditTemplate {
        ctx,
        id: Some(id),
        name: article.name.clone(),
        price: money::format_plain(article.price_cents),
        sort_order: article.sort_order,
        active: article.active,
        barcodes,
        translations: translation_fields(&state, &translations),
        image_url: article.image_url(),
        error,
        saved: q.saved.is_some(),
        max_image_kb: state.config.max_image_bytes / 1024,
    })?
    .into_response())
}

/// Liest das Artikelformular. Die Felder `name_<sprache>` enthalten die Übersetzungen.
fn parse_article_form(
    state: &AppState,
    form: &HashMap<String, String>,
) -> (ArticleInput, Option<&'static str>) {
    let field = |k: &str| {
        form.get(k)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let mut error = None;
    let price_cents = match money::parse_cents(&field("price")) {
        Ok(c) if c >= 0 => c,
        _ => {
            error = Some("error-price-invalid");
            0
        }
    };
    let translations = state
        .locales
        .languages()
        .iter()
        .filter_map(|l| {
            form.get(&format!("name_{}", l.code))
                .map(|v| (l.code.clone(), v.trim().to_string()))
        })
        .collect();
    let input = ArticleInput {
        name: field("name"),
        price_cents,
        active: form.contains_key("active"),
        sort_order: field("sort_order").parse().unwrap_or(0),
        translations,
        barcodes: field("barcodes").lines().map(str::to_string).collect(),
    };
    (input, error)
}

fn article_error_text(t: &Translator, e: &ArticleError, max_image_kb: usize) -> String {
    match e {
        ArticleError::NameEmpty => t.t("error-name-empty"),
        ArticleError::InvalidPrice => t.t("error-price-invalid"),
        ArticleError::InvalidBarcode(code) => t.t1("error-barcode-invalid", "code", code),
        ArticleError::BarcodeTaken { code, article } => {
            let mut args = fluent_bundle::FluentArgs::new();
            args.set("code", code.clone());
            args.set("article", article.clone());
            t.ta("error-barcode-taken", Some(&args))
        }
        ArticleError::NotFound => t.t("error-not-found"),
        ArticleError::UnsupportedImage => t.t("error-image-type"),
        ArticleError::ImageTooLarge => t.tn("error-image-size", "kb", max_image_kb as i64),
        ArticleError::Db => t.t("error-internal"),
    }
}

async fn save_article(
    state: AppState,
    ctx: PageCtx,
    id: Option<i64>,
    form: HashMap<String, String>,
) -> Result<Response, AppError> {
    let (input, parse_error) = parse_article_form(&state, &form);
    let result = match parse_error {
        Some(key) => Err(ctx.t.t(key)),
        None => articles::save(&state.pool, id, &input)
            .await
            .map_err(|e| article_error_text(&ctx.t, &e, state.config.max_image_bytes / 1024)),
    };
    match result {
        Ok(id) => Ok(Redirect::to(&format!("/admin/articles/{id}?saved=1")).into_response()),
        Err(error) => {
            let image_url = match id {
                Some(id) => articles::get(&state.pool, id, "")
                    .await?
                    .and_then(|a| a.image_url()),
                None => None,
            };
            Ok(render(&ArticleEditTemplate {
                ctx,
                id,
                name: input.name,
                price: form.get("price").cloned().unwrap_or_default(),
                sort_order: input.sort_order,
                active: input.active,
                barcodes: form.get("barcodes").cloned().unwrap_or_default(),
                translations: translation_fields(&state, &input.translations),
                image_url,
                error: Some(error),
                saved: false,
                max_image_kb: state.config.max_image_bytes / 1024,
            })?
            .into_response())
        }
    }
}

async fn article_create(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    save_article(state, ctx, None, form).await
}

async fn article_update(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Path(id): Path<i64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    save_article(state, ctx, Some(id), form).await
}

async fn article_image_upload(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let mut data = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("image") {
            match field.bytes().await {
                Ok(bytes) => data = Some(bytes),
                Err(_) => {
                    return Ok(
                        Redirect::to(&format!("/admin/articles/{id}?error=image-size"))
                            .into_response(),
                    );
                }
            }
        }
    }
    let Some(data) = data.filter(|d| !d.is_empty()) else {
        return Ok(Redirect::to(&format!("/admin/articles/{id}")).into_response());
    };
    let target =
        match articles::set_image(&state.pool, id, &data, state.config.max_image_bytes).await {
            Ok(()) => format!("/admin/articles/{id}?saved=1"),
            Err(ArticleError::UnsupportedImage) => format!("/admin/articles/{id}?error=image-type"),
            Err(ArticleError::ImageTooLarge) => format!("/admin/articles/{id}?error=image-size"),
            Err(_) => format!("/admin/articles/{id}?error=internal"),
        };
    Ok(Redirect::to(&target).into_response())
}

async fn article_image_delete(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    articles::remove_image(&state.pool, id).await?;
    Ok(Redirect::to(&format!("/admin/articles/{id}?saved=1")).into_response())
}

async fn article_move(
    State(state): State<AppState>,
    _session: AdminSession,
    Path((id, dir)): Path<(i64, String)>,
) -> Result<Response, AppError> {
    articles::move_article(&state.pool, id, dir == "up").await?;
    Ok(Redirect::to("/admin/articles").into_response())
}

// ---------------------------------------------------------------------------
// Benutzer
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "admin/users.html")]
struct UserListTemplate {
    ctx: PageCtx,
    users: Vec<User>,
    name: String,
    error: Option<String>,
}

async fn user_list(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let users = users::list_all(&state.pool).await?;
    Ok(render(&UserListTemplate {
        ctx,
        users,
        name: String::new(),
        error: None,
    })?
    .into_response())
}

fn user_error_text(t: &Translator, e: &UserError) -> String {
    match e {
        UserError::NameEmpty => t.t("error-name-empty"),
        UserError::NameTooLong => t.tn("error-name-too-long", "max", users::MAX_NAME_LEN as i64),
        UserError::NameTaken => t.t("error-name-taken"),
        UserError::PinInvalid => t.t("error-pin-invalid"),
        UserError::PinMismatch => t.t("error-pin-mismatch"),
        UserError::NotFound => t.t("error-user-not-found"),
        UserError::Db => t.t("error-internal"),
    }
}

#[derive(Deserialize)]
struct UserCreateForm {
    name: String,
    #[serde(default)]
    pin: String,
}

async fn user_create(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(form): Form<UserCreateForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let pin = Some(form.pin.trim()).filter(|p| !p.is_empty());
    match users::create(&state.pool, &form.name, pin).await {
        Ok(user) => Ok(Redirect::to(&format!("/admin/users/{}?saved=1", user.id)).into_response()),
        Err(e) => {
            let error = Some(user_error_text(&ctx.t, &e));
            let users = users::list_all(&state.pool).await?;
            Ok(render(&UserListTemplate {
                ctx,
                users,
                name: form.name,
                error,
            })?
            .into_response())
        }
    }
}

#[derive(Template)]
#[template(path = "admin/user_edit.html")]
struct UserEditTemplate {
    ctx: PageCtx,
    user: User,
    transactions: Vec<TxRow>,
    error: Option<String>,
    saved: bool,
    correction_amount: String,
    correction_reason: String,
}

async fn render_user_edit(
    state: &AppState,
    ctx: PageCtx,
    id: i64,
    error: Option<String>,
    saved: bool,
    correction: (String, String),
) -> Result<Response, AppError> {
    let user = users::get(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let filter = TxFilter {
        user: Some(id),
        ..Default::default()
    };
    let transactions = query_transactions(state, &filter, Some(30), 0).await?;
    Ok(render(&UserEditTemplate {
        ctx,
        user,
        transactions,
        error,
        saved,
        correction_amount: correction.0,
        correction_reason: correction.1,
    })?
    .into_response())
}

async fn user_edit(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Path(id): Path<i64>,
    Query(q): Query<SavedQuery>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    render_user_edit(&state, ctx, id, None, q.saved.is_some(), Default::default()).await
}

#[derive(Deserialize)]
struct UserUpdateForm {
    name: String,
    #[serde(default)]
    language: String,
    active: Option<String>,
}

async fn user_update(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Path(id): Path<i64>,
    Form(form): Form<UserUpdateForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let lang = Some(form.language.as_str()).filter(|l| state.locales.is_available(l));
    match users::update(&state.pool, id, &form.name, form.active.is_some(), lang).await {
        Ok(()) => Ok(Redirect::to(&format!("/admin/users/{id}?saved=1")).into_response()),
        Err(e) => {
            let error = Some(user_error_text(&ctx.t, &e));
            render_user_edit(&state, ctx, id, error, false, Default::default()).await
        }
    }
}

#[derive(Deserialize)]
struct UserPinForm {
    #[serde(default)]
    pin: String,
}

/// PIN zurücksetzen: leeres Feld entfernt die PIN, sonst wird die neue PIN gesetzt.
async fn user_pin(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Path(id): Path<i64>,
    Form(form): Form<UserPinForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let pin = Some(form.pin.trim()).filter(|p| !p.is_empty());
    match users::set_pin(&state.pool, id, pin).await {
        Ok(()) => {
            tracing::info!(
                "Admin „{}“ hat die PIN von Benutzer #{id} zurückgesetzt",
                session.admin.username
            );
            Ok(Redirect::to(&format!("/admin/users/{id}?saved=1")).into_response())
        }
        Err(e) => {
            let error = Some(user_error_text(&ctx.t, &e));
            render_user_edit(&state, ctx, id, error, false, Default::default()).await
        }
    }
}

#[derive(Deserialize)]
struct CorrectionForm {
    amount: String,
    reason: String,
}

async fn user_correction(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Path(id): Path<i64>,
    Form(form): Form<CorrectionForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let result = match money::parse_cents(&form.amount) {
        Err(_) => Err(ctx.t.t("error-amount-invalid")),
        Ok(cents) => ledger::correction(&state.pool, id, cents, &form.reason, session.admin.id)
            .await
            .map_err(|e| match e {
                LedgerError::ReasonRequired => ctx.t.t("error-reason-required"),
                LedgerError::InvalidAmount => ctx.t.t("error-amount-invalid"),
                LedgerError::UserNotFound => ctx.t.t("error-user-not-found"),
                other => {
                    tracing::error!("Korrektur: {other}");
                    ctx.t.t("error-internal")
                }
            }),
    };
    match result {
        Ok(_) => Ok(Redirect::to(&format!("/admin/users/{id}?saved=1")).into_response()),
        Err(error) => {
            render_user_edit(
                &state,
                ctx,
                id,
                Some(error),
                false,
                (form.amount, form.reason),
            )
            .await
        }
    }
}

// ---------------------------------------------------------------------------
// Buchungen
// ---------------------------------------------------------------------------

/// Eine Buchung mit Namen für die Anzeige.
#[derive(Debug, sqlx::FromRow)]
pub struct TxRow {
    pub id: i64,
    pub created_at: String,
    pub user_id: i64,
    pub user_name: String,
    pub amount_cents: i64,
    pub kind: TxKind,
    pub article_name: Option<String>,
    pub note: Option<String>,
    pub admin_name: Option<String>,
    pub reverses_id: Option<i64>,
    /// ID der Storno-Buchung, falls diese Buchung storniert wurde
    pub reversed_by: Option<i64>,
}

impl TxRow {
    pub fn can_reverse(&self) -> bool {
        self.kind != TxKind::Reversal && self.reversed_by.is_none()
    }

    /// Übersetzungsschlüssel für die Buchungsart
    pub fn kind_key(&self) -> &'static str {
        kind_key(self.kind)
    }
}

fn kind_key(kind: TxKind) -> &'static str {
    match kind {
        TxKind::Purchase => "tx-kind-purchase",
        TxKind::Deposit => "tx-kind-deposit",
        TxKind::Correction => "tx-kind-correction",
        TxKind::Reversal => "tx-kind-reversal",
    }
}

/// Filter der Buchungsübersicht (aus den Query-Parametern).
#[derive(Debug, Default, Deserialize)]
pub struct TxFilterQuery {
    #[serde(default)]
    user: String,
    #[serde(default)]
    article: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    from: String,
    #[serde(default)]
    to: String,
    #[serde(default)]
    page: String,
    /// gesetzt nach einem Storno (für die Erfolgsmeldung)
    #[serde(default)]
    reversed: String,
}

/// Geprüfte Filterwerte.
#[derive(Debug, Default, Clone)]
pub struct TxFilter {
    user: Option<i64>,
    article: Option<i64>,
    kind: Option<TxKind>,
    /// UTC-Grenzen im Datenbankformat
    from: Option<String>,
    to: Option<String>,
}

impl TxFilterQuery {
    fn parse(&self, tz: Tz) -> TxFilter {
        let date = |s: &str| NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok();
        TxFilter {
            user: self.user.trim().parse().ok(),
            article: self.article.trim().parse().ok(),
            kind: TxKind::parse(self.kind.trim()),
            from: date(&self.from).map(|d| local_day_start(tz, d)),
            // „bis“ schließt den ganzen Tag ein
            to: date(&self.to)
                .and_then(|d| d.succ_opt())
                .map(|d| local_day_start(tz, d)),
        }
    }

    fn page(&self) -> i64 {
        self.page.trim().parse::<i64>().unwrap_or(1).max(1)
    }

    /// Query-String ohne Seitenzahl (für Blättern und CSV-Link)
    fn query_string(&self) -> String {
        [
            ("user", &self.user),
            ("article", &self.article),
            ("kind", &self.kind),
            ("from", &self.from),
            ("to", &self.to),
        ]
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}={}", percent_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
    }
}

/// Beginn eines Kalendertages in der Vereins-Zeitzone, als UTC-Zeitstempel.
fn local_day_start(tz: Tz, date: NaiveDate) -> String {
    let naive = date.and_hms_opt(0, 0, 0).unwrap_or_default();
    let local = tz
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| Utc.from_utc_datetime(&naive));
    db::to_db_time(local)
}

fn push_filters(qb: &mut QueryBuilder<Sqlite>, f: &TxFilter) {
    qb.push(" WHERE 1 = 1");
    if let Some(user) = f.user {
        qb.push(" AND t.user_id = ").push_bind(user);
    }
    if let Some(article) = f.article {
        qb.push(" AND t.article_id = ").push_bind(article);
    }
    if let Some(kind) = f.kind {
        qb.push(" AND t.kind = ").push_bind(kind.as_str());
    }
    if let Some(from) = &f.from {
        qb.push(" AND t.created_at >= ").push_bind(from.clone());
    }
    if let Some(to) = &f.to {
        qb.push(" AND t.created_at < ").push_bind(to.clone());
    }
}

async fn query_transactions(
    state: &AppState,
    f: &TxFilter,
    limit: Option<i64>,
    offset: i64,
) -> Result<Vec<TxRow>, AppError> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT t.id, t.created_at, t.user_id, u.name AS user_name, t.amount_cents, t.kind,
                COALESCE(t.article_name, a.name) AS article_name, t.note,
                ad.username AS admin_name, t.reverses_id, r.id AS reversed_by
         FROM transactions t
         JOIN users u ON u.id = t.user_id
         LEFT JOIN articles a ON a.id = t.article_id
         LEFT JOIN admins ad ON ad.id = t.admin_id
         LEFT JOIN transactions r ON r.reverses_id = t.id",
    );
    push_filters(&mut qb, f);
    qb.push(" ORDER BY t.id DESC");
    if let Some(limit) = limit {
        qb.push(" LIMIT ")
            .push_bind(limit)
            .push(" OFFSET ")
            .push_bind(offset);
    }
    Ok(qb.build_query_as::<TxRow>().fetch_all(&state.pool).await?)
}

/// Auswahlmöglichkeit in einem Filter-Dropdown.
struct FilterOption {
    value: String,
    label: String,
    selected: bool,
}

#[derive(Template)]
#[template(path = "admin/transactions.html")]
struct TransactionListTemplate {
    ctx: PageCtx,
    rows: Vec<TxRow>,
    users: Vec<FilterOption>,
    articles: Vec<FilterOption>,
    kinds: Vec<FilterOption>,
    from: String,
    to: String,
    total_count: i64,
    total_cents: i64,
    page: i64,
    has_next: bool,
    query: String,
    message: Option<String>,
}

async fn transaction_list(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Query(q): Query<TxFilterQuery>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let filter = q.parse(state.config.timezone);
    let page = q.page();

    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT COUNT(*), COALESCE(SUM(t.amount_cents), 0) FROM transactions t",
    );
    push_filters(&mut qb, &filter);
    let (total_count, total_cents): (i64, i64) = qb.build_query_as().fetch_one(&state.pool).await?;

    let rows = query_transactions(&state, &filter, Some(PAGE_SIZE), (page - 1) * PAGE_SIZE).await?;
    let has_next = page * PAGE_SIZE < total_count;

    let users = users::list_all(&state.pool)
        .await?
        .into_iter()
        .map(|u| FilterOption {
            selected: filter.user == Some(u.id),
            value: u.id.to_string(),
            label: u.name,
        })
        .collect();
    let articles = articles::list_all(&state.pool)
        .await?
        .into_iter()
        .map(|a| FilterOption {
            selected: filter.article == Some(a.id),
            value: a.id.to_string(),
            label: a.name,
        })
        .collect();
    let kinds = TxKind::ALL
        .iter()
        .map(|&k| FilterOption {
            selected: filter.kind == Some(k),
            value: k.as_str().to_string(),
            label: ctx.t.t(kind_key(k)),
        })
        .collect();
    let message = (!q.reversed.is_empty()).then(|| ctx.t.t("admin-reversed"));
    Ok(render(&TransactionListTemplate {
        query: q.query_string(),
        from: q.from.clone(),
        to: q.to.clone(),
        ctx,
        rows,
        users,
        articles,
        kinds,
        total_count,
        total_cents,
        page,
        has_next,
        message,
    })?
    .into_response())
}

/// CSV-Export mit denselben Filtern wie die Übersicht.
///
/// Für deutsche/französische Tabellenkalkulationen wird `;` als Trennzeichen
/// und `,` als Dezimalzeichen verwendet, für Englisch `,` und `.`.
async fn transaction_csv(
    State(state): State<AppState>,
    ctx: PageCtx,
    _session: AdminSession,
    Query(q): Query<TxFilterQuery>,
) -> Result<Response, AppError> {
    let filter = q.parse(state.config.timezone);
    let rows = query_transactions(&state, &filter, None, 0).await?;
    let t = &ctx.t;
    let delimiter = if t.decimal_separator() == "," {
        b';'
    } else {
        b','
    };

    let mut wtr = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());
    let header = [
        "csv-id",
        "csv-time",
        "csv-user",
        "csv-kind",
        "csv-article",
        "csv-amount",
        "csv-amount-cents",
        "csv-note",
        "csv-admin",
        "csv-reverses",
    ]
    .map(|k| t.t(k));
    let csv_err = |e: csv::Error| AppError::Internal(format!("CSV: {e}"));
    wtr.write_record(&header).map_err(csv_err)?;
    for r in &rows {
        let local_time = db::parse_db_time(&r.created_at)
            .map(|d| {
                d.with_timezone(&state.config.timezone)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| r.created_at.clone());
        wtr.write_record([
            r.id.to_string(),
            local_time,
            r.user_name.clone(),
            t.t(r.kind_key()),
            r.article_name.clone().unwrap_or_default(),
            t.number(r.amount_cents),
            r.amount_cents.to_string(),
            r.note.clone().unwrap_or_default(),
            r.admin_name.clone().unwrap_or_default(),
            r.reverses_id.map(|i| i.to_string()).unwrap_or_default(),
        ])
        .map_err(csv_err)?;
    }
    let data = wtr
        .into_inner()
        .map_err(|e| AppError::Internal(format!("CSV: {e}")))?;
    // BOM, damit Excel UTF-8 erkennt
    let mut body = vec![0xEF, 0xBB, 0xBF];
    body.extend_from_slice(&data);

    let filename = format!("cratetab-{}.csv", Utc::now().format("%Y-%m-%d"));
    let mut res = body.into_response();
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    Ok(res)
}

#[derive(Deserialize)]
struct ReverseForm {
    #[serde(default)]
    note: String,
    #[serde(default)]
    back: String,
}

async fn transaction_reverse(
    State(state): State<AppState>,
    session: AdminSession,
    Path(id): Path<i64>,
    Form(form): Form<ReverseForm>,
) -> Result<Response, AppError> {
    match ledger::reverse_by_admin(&state.pool, id, session.admin.id, Some(&form.note)).await {
        Ok(_) => tracing::info!(
            "Admin „{}“ hat Buchung #{id} storniert",
            session.admin.username
        ),
        Err(LedgerError::AlreadyReversed | LedgerError::NotReversible) => {}
        Err(e) => return Err(e.into()),
    }
    let back = Some(form.back)
        .filter(|b| is_local_path(b) && b.starts_with("/admin"))
        .unwrap_or_else(|| "/admin/transactions".into());
    let sep = if back.contains('?') { '&' } else { '?' };
    Ok(Redirect::to(&format!("{back}{sep}reversed=1")).into_response())
}

// ---------------------------------------------------------------------------
// Statistik
// ---------------------------------------------------------------------------

/// Umsatz eines Artikels in einem Zeitraum.
#[derive(Debug, Default, Clone)]
struct ArticleStat {
    name: String,
    quantity: i64,
    revenue_cents: i64,
}

/// Kennzahlen eines Monats.
#[derive(Debug, Default)]
struct MonthStat {
    /// z. B. "2026-10"
    label: String,
    revenue_cents: i64,
    deposits_cents: i64,
    articles: Vec<ArticleStat>,
}

/// Eine Buchung für die Statistik:
/// (Zeitpunkt, Art, Betrag, Artikel-ID, Artikelname, Admin-ID)
type StatRow = (
    String,
    TxKind,
    i64,
    Option<i64>,
    Option<String>,
    Option<i64>,
);

#[derive(Template)]
#[template(path = "admin/stats.html")]
struct StatsTemplate {
    ctx: PageCtx,
    summary: BalanceSummary,
    year: i32,
    years: Vec<i32>,
    year_revenue_cents: i64,
    year_deposits_cents: i64,
    year_articles: Vec<ArticleStat>,
    months: Vec<MonthStat>,
}

#[derive(Deserialize)]
struct StatsQuery {
    year: Option<i32>,
}

async fn stats(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Query(q): Query<StatsQuery>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let tz = state.config.timezone;
    let this_year = Utc::now().with_timezone(&tz).year();
    let year = q.year.unwrap_or(this_year);

    // Jahre mit Buchungen für die Auswahl
    let first: Option<String> = sqlx::query_scalar("SELECT MIN(created_at) FROM transactions")
        .fetch_one(&state.pool)
        .await?;
    let first_year = first
        .as_deref()
        .and_then(db::parse_db_time)
        .map(|d| d.with_timezone(&tz).year())
        .unwrap_or(this_year);
    let years: Vec<i32> = (first_year.min(year)..=this_year.max(year)).rev().collect();

    let start = NaiveDate::from_ymd_opt(year, 1, 1).map(|d| local_day_start(tz, d));
    let end = NaiveDate::from_ymd_opt(year + 1, 1, 1).map(|d| local_day_start(tz, d));
    let (Some(start), Some(end)) = (start, end) else {
        return Err(AppError::NotFound);
    };

    // Käufe/Stornos von Käufen und Aufladungen des Jahres; Aggregation in Rust,
    // damit Monatsgrenzen in der Vereins-Zeitzone korrekt sind.
    let rows: Vec<StatRow> = sqlx::query_as(
        "SELECT t.created_at, t.kind, t.amount_cents, t.article_id,
                COALESCE(a.name, t.article_name), t.admin_id
         FROM transactions t LEFT JOIN articles a ON a.id = t.article_id
         WHERE t.created_at >= ? AND t.created_at < ?",
    )
    .bind(start)
    .bind(end)
    .fetch_all(&state.pool)
    .await?;

    // Monat (1–12) -> Artikel-ID -> Statistik
    let mut by_month: BTreeMap<u32, (i64, BTreeMap<i64, ArticleStat>)> = BTreeMap::new();
    for (created, kind, amount, article_id, name, admin_id) in rows {
        let Some(month) = db::parse_db_time(&created).map(|d| d.with_timezone(&tz).month()) else {
            continue;
        };
        let entry = by_month.entry(month).or_default();
        match (kind, article_id) {
            (TxKind::Purchase | TxKind::Reversal, Some(aid)) => {
                let stat = entry.1.entry(aid).or_default();
                stat.name = name.unwrap_or_default();
                stat.quantity += if kind == TxKind::Purchase { 1 } else { -1 };
                stat.revenue_cents -= amount;
            }
            (TxKind::Deposit, _) => entry.0 += amount,
            // Storno einer Aufladung am Kiosk
            (TxKind::Reversal, None) if admin_id.is_none() => entry.0 += amount,
            _ => {}
        }
    }

    let mut year_articles: BTreeMap<String, ArticleStat> = BTreeMap::new();
    let mut months = Vec::new();
    for (month, (deposits, articles)) in by_month.into_iter().rev() {
        let mut list: Vec<ArticleStat> = articles
            .into_values()
            .filter(|a| a.quantity != 0 || a.revenue_cents != 0)
            .collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.revenue_cents));
        for a in &list {
            let y = year_articles
                .entry(a.name.clone())
                .or_insert_with(|| ArticleStat {
                    name: a.name.clone(),
                    ..Default::default()
                });
            y.quantity += a.quantity;
            y.revenue_cents += a.revenue_cents;
        }
        months.push(MonthStat {
            label: format!("{year}-{month:02}"),
            revenue_cents: list.iter().map(|a| a.revenue_cents).sum(),
            deposits_cents: deposits,
            articles: list,
        });
    }
    let mut year_articles: Vec<ArticleStat> = year_articles.into_values().collect();
    year_articles.sort_by_key(|a| std::cmp::Reverse(a.revenue_cents));

    Ok(render(&StatsTemplate {
        ctx,
        summary: balance_summary(&state).await?,
        year,
        years,
        year_revenue_cents: months.iter().map(|m| m.revenue_cents).sum(),
        year_deposits_cents: months.iter().map(|m| m.deposits_cents).sum(),
        year_articles,
        months,
    })?
    .into_response())
}

// ---------------------------------------------------------------------------
// Admin-Zugänge
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "admin/admins.html")]
struct AdminListTemplate {
    ctx: PageCtx,
    admins: Vec<Admin>,
    me: i64,
    error: Option<String>,
    saved: bool,
}

async fn render_admins(
    state: &AppState,
    ctx: PageCtx,
    session: &AdminSession,
    error: Option<String>,
    saved: bool,
) -> Result<Response, AppError> {
    let admins = admins::list(&state.pool).await?;
    Ok(render(&AdminListTemplate {
        ctx,
        admins,
        me: session.admin.id,
        error,
        saved,
    })?
    .into_response())
}

async fn admin_list(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Query(q): Query<SavedQuery>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    render_admins(&state, ctx, &session, None, q.saved.is_some()).await
}

#[derive(Deserialize)]
struct AdminCreateForm {
    username: String,
    password: String,
}

async fn admin_create(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(form): Form<AdminCreateForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    if form.username.trim().is_empty() {
        let e = ctx.t.t("error-name-empty");
        return render_admins(&state, ctx, &session, Some(e), false).await;
    }
    if form.password.chars().count() < admins::MIN_PASSWORD_LEN {
        let e = ctx.t.tn(
            "error-password-short",
            "min",
            admins::MIN_PASSWORD_LEN as i64,
        );
        return render_admins(&state, ctx, &session, Some(e), false).await;
    }
    match admins::create(&state.pool, &form.username, &form.password).await {
        Ok(_) => Ok(Redirect::to("/admin/admins?saved=1").into_response()),
        Err(_) => {
            let e = ctx.t.t("error-name-taken");
            render_admins(&state, ctx, &session, Some(e), false).await
        }
    }
}

#[derive(Deserialize)]
struct PasswordForm {
    current: String,
    password: String,
    password_repeat: String,
}

async fn admin_password(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(form): Form<PasswordForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let error = if admins::login(&state.pool, &session.admin.username, &form.current)
        .await?
        .is_none()
    {
        Some(ctx.t.t("error-password-wrong"))
    } else if form.password.chars().count() < admins::MIN_PASSWORD_LEN {
        Some(ctx.t.tn(
            "error-password-short",
            "min",
            admins::MIN_PASSWORD_LEN as i64,
        ))
    } else if form.password != form.password_repeat {
        Some(ctx.t.t("error-password-mismatch"))
    } else {
        None
    };
    if let Some(e) = error {
        return render_admins(&state, ctx, &session, Some(e), false).await;
    }
    admins::set_password(&state.pool, session.admin.id, &form.password).await?;
    Ok(Redirect::to("/admin/admins?saved=1").into_response())
}

async fn admin_toggle(
    State(state): State<AppState>,
    session: AdminSession,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    // Sich selbst aussperren geht nicht
    if id != session.admin.id
        && let Some(admin) = admins::get(&state.pool, id).await?
    {
        admins::set_active(&state.pool, id, !admin.active).await?;
    }
    Ok(Redirect::to("/admin/admins?saved=1").into_response())
}
