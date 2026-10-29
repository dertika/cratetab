//! Kiosk-Oberfläche für Mitglieder.
//!
//! Ablauf: Startseite mit allen Benutzern → Benutzer antippen → ggf. PIN
//! eingeben → Artikel antippen oder Barcode scannen. Nach Inaktivität springt
//! das Gerät (per JavaScript) zurück zur Startseite; dort wird die Sitzung beendet.
//!
//! Buchungen werden per htmx abgeschickt. Die Antwort ist nur der Statusbereich
//! (Guthaben, Meldung, „Rückgängig“-Knopf), der auf der Seite ersetzt wird.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::cookie::{CookieJar, SameSite};
use chrono::Utc;
use fluent_bundle::FluentArgs;
use serde::Deserialize;

use super::{AppError, AppState, PageCtx, render};
use crate::{
    articles::{self, Article},
    i18n::Translator,
    ledger::{self, LedgerError, TxKind},
    money,
    sessions::{self, Owner},
    users::{self, PinCheck, User, UserError},
};

/// Cookie mit der Sitzung eines per PIN entsperrten Kiosk-Benutzers.
const KIOSK_COOKIE: &str = "kiosk_session";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(home))
        .route("/register", get(register_form).post(register))
        .route("/u/{id}", get(user_page))
        .route("/u/{id}/pin", get(pin_form).post(pin_submit))
        .route("/u/{id}/buy/{article}", post(buy))
        .route("/u/{id}/scan", post(scan))
        .route("/u/{id}/deposit", post(deposit))
        .route("/u/{id}/undo", post(undo))
        .route("/u/{id}/settings", get(settings))
        .route("/u/{id}/settings/pin", post(settings_pin))
        .route("/u/{id}/settings/language", post(settings_language))
        .route("/images/articles/{id}", get(article_image))
}

// ---------------------------------------------------------------------------
// Zugriffsschutz
// ---------------------------------------------------------------------------

/// Gültigkeit einer Kiosk-Sitzung: Inaktivitäts-Timeout plus etwas Puffer.
fn kiosk_ttl(state: &AppState) -> chrono::Duration {
    chrono::Duration::seconds(state.config.idle_timeout_secs as i64 + 30)
}

/// Liefert den Benutzer, wenn er aktiv ist und dieses Gerät auf ihn zugreifen
/// darf: Entweder hat er keine PIN, oder es gibt eine gültige Kiosk-Sitzung für ihn.
pub async fn authorized_user(
    state: &AppState,
    jar: &CookieJar,
    id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let Some(user) = users::get(&state.pool, id).await? else {
        return Ok(None);
    };
    if !user.active {
        return Ok(None);
    }
    if !user.has_pin() {
        return Ok(Some(user));
    }
    if let Some(cookie) = jar.get(KIOSK_COOKIE)
        && sessions::validate(&state.pool, cookie.value(), kiosk_ttl(state)).await?
            == Some(Owner::User(id))
    {
        return Ok(Some(user));
    }
    Ok(None)
}

/// `/u/42/settings` → `Some(42)`
pub fn user_id_from_path(path: &str) -> Option<i64> {
    let rest = path.strip_prefix("/u/")?;
    let id = rest.split(['/', '?']).next()?;
    id.parse().ok()
}

/// Zugriff verweigert: zur PIN-Abfrage (bzw. Startseite) schicken.
/// Bei htmx-Anfragen per `HX-Redirect`, damit die ganze Seite wechselt.
async fn deny(state: &AppState, headers: &HeaderMap, id: i64) -> Response {
    let target = match users::get(&state.pool, id).await {
        Ok(Some(u)) if u.active && u.has_pin() => format!("/u/{id}/pin"),
        _ => "/".to_string(),
    };
    if headers.contains_key("hx-request") {
        let mut res = StatusCode::OK.into_response();
        if let Ok(v) = HeaderValue::from_str(&target) {
            res.headers_mut().insert("hx-redirect", v);
        }
        res
    } else {
        Redirect::to(&target).into_response()
    }
}

/// Prüft den Zugriff in einem Handler: `let user = guard!(state, jar, headers, id);`
macro_rules! guard {
    ($state:expr, $jar:expr, $headers:expr, $id:expr) => {
        match authorized_user(&$state, &$jar, $id).await? {
            Some(user) => user,
            None => return Ok(deny(&$state, &$headers, $id).await),
        }
    };
}

// ---------------------------------------------------------------------------
// Startseite
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "kiosk/home.html")]
struct HomeTemplate {
    ctx: PageCtx,
    users: Vec<User>,
    /// Anfangsbuchstaben für die Buchstabenleiste
    letters: Vec<String>,
    self_registration: bool,
}

async fn home(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
) -> Result<Response, AppError> {
    // Wer auf der Startseite landet, ist fertig: Kiosk-Sitzung beenden.
    let jar = match jar.get(KIOSK_COOKIE) {
        Some(c) => {
            sessions::delete(&state.pool, c.value()).await?;
            jar.remove(ctx.cookie(KIOSK_COOKIE, String::new(), SameSite::Strict))
        }
        None => jar,
    };
    let users = users::list_active(&state.pool).await?;
    let mut letters: Vec<String> = users.iter().map(User::initial).collect();
    letters.dedup();
    let ctx = ctx.with_idle(state.config.idle_timeout_secs);
    let page = render(&HomeTemplate {
        ctx,
        users,
        letters,
        self_registration: state.config.self_registration,
    })?;
    Ok((jar, page).into_response())
}

// ---------------------------------------------------------------------------
// Selbstregistrierung
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "kiosk/register.html")]
struct RegisterTemplate {
    ctx: PageCtx,
    name: String,
    error: Option<String>,
}

#[derive(Deserialize)]
struct RegisterForm {
    name: String,
    #[serde(default)]
    pin: String,
    #[serde(default)]
    pin_repeat: String,
}

async fn register_form(State(state): State<AppState>, ctx: PageCtx) -> Result<Response, AppError> {
    if !state.config.self_registration {
        return Err(AppError::NotFound);
    }
    let ctx = ctx.with_idle(state.config.idle_timeout_secs * 3);
    Ok(render(&RegisterTemplate {
        ctx,
        name: String::new(),
        error: None,
    })?
    .into_response())
}

async fn register(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    Form(form): Form<RegisterForm>,
) -> Result<Response, AppError> {
    if !state.config.self_registration {
        return Err(AppError::NotFound);
    }
    let pin = form.pin.trim();
    let result = if !pin.is_empty() && pin != form.pin_repeat.trim() {
        Err(UserError::PinMismatch)
    } else {
        users::create(&state.pool, &form.name, Some(pin).filter(|p| !p.is_empty())).await
    };
    match result {
        Ok(user) => {
            tracing::info!(
                "Neuer Benutzer „{}“ (#{}) am Kiosk angelegt",
                user.name,
                user.id
            );
            // Mit PIN: gleich entsperren, damit niemand die PIN zweimal tippen muss
            let jar = if user.has_pin() {
                let token =
                    sessions::create(&state.pool, Owner::User(user.id), kiosk_ttl(&state)).await?;
                jar.add(ctx.cookie(KIOSK_COOKIE, token, SameSite::Strict))
            } else {
                jar
            };
            Ok((jar, Redirect::to(&format!("/u/{}", user.id))).into_response())
        }
        Err(e) => {
            let error = Some(user_error_text(&ctx.t, &e));
            let ctx = ctx.with_idle(state.config.idle_timeout_secs * 3);
            Ok(render(&RegisterTemplate {
                ctx,
                name: form.name,
                error,
            })?
            .into_response())
        }
    }
}

/// Übersetzt Fehler bei Benutzerdaten.
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

// ---------------------------------------------------------------------------
// PIN-Abfrage
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "kiosk/pin.html")]
struct PinTemplate {
    ctx: PageCtx,
    user: User,
    error: Option<String>,
}

#[derive(Deserialize)]
struct PinForm {
    pin: String,
}

async fn pin_form(
    State(state): State<AppState>,
    ctx: PageCtx,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = users::get(&state.pool, id)
        .await?
        .filter(|u| u.active)
        .ok_or(AppError::NotFound)?;
    if !user.has_pin() {
        return Ok(Redirect::to(&format!("/u/{id}")).into_response());
    }
    let ctx = ctx
        .for_user(&user, &state.locales)
        .with_idle(state.config.idle_timeout_secs);
    Ok(render(&PinTemplate {
        ctx,
        user,
        error: None,
    })?
    .into_response())
}

async fn pin_submit(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    Path(id): Path<i64>,
    Form(form): Form<PinForm>,
) -> Result<Response, AppError> {
    let user = users::get(&state.pool, id)
        .await?
        .filter(|u| u.active)
        .ok_or(AppError::NotFound)?;
    let ctx = ctx
        .for_user(&user, &state.locales)
        .with_idle(state.config.idle_timeout_secs);
    let error = match users::check_pin(&state.pool, id, form.pin.trim()).await? {
        PinCheck::Ok => {
            // Eine evtl. vorhandene Sitzung eines anderen Benutzers beenden
            if let Some(old) = jar.get(KIOSK_COOKIE) {
                sessions::delete(&state.pool, old.value()).await?;
            }
            let token = sessions::create(&state.pool, Owner::User(id), kiosk_ttl(&state)).await?;
            let jar = jar.add(ctx.cookie(KIOSK_COOKIE, token, SameSite::Strict));
            return Ok((jar, Redirect::to(&format!("/u/{id}"))).into_response());
        }
        PinCheck::Wrong => ctx.t.t("pin-wrong"),
        PinCheck::Locked => ctx.t.tn("pin-locked", "minutes", users::PIN_LOCK_MINUTES),
    };
    Ok(render(&PinTemplate {
        ctx,
        user,
        error: Some(error),
    })?
    .into_response())
}

// ---------------------------------------------------------------------------
// Benutzerseite: Artikel, Guthaben, Aufladen
// ---------------------------------------------------------------------------

/// Eine Meldung nach einer Aktion.
pub struct Flash {
    /// „ok“ oder „error“ (CSS-Klasse)
    pub kind: &'static str,
    pub text: String,
}

impl Flash {
    fn ok(text: String) -> Self {
        Flash { kind: "ok", text }
    }
    fn error(text: String) -> Self {
        Flash {
            kind: "error",
            text,
        }
    }
}

/// Rückgängig-Knopf: Beschreibung der Buchung und verbleibende Sekunden.
pub struct UndoView {
    pub label: String,
    pub seconds_left: i64,
}

/// Inhalt des Statusbereichs.
pub struct StatusView {
    pub balance_cents: i64,
    pub min_balance_cents: i64,
    pub flash: Option<Flash>,
    pub undo: Option<UndoView>,
}

impl StatusView {
    pub fn is_negative(&self) -> bool {
        self.balance_cents < 0
    }
}

/// Schnellwahl-Knopf für die Aufladung.
pub struct Preset {
    /// Wert für das Formular, z. B. "5.00"
    pub value: String,
    pub cents: i64,
}

#[derive(Template)]
#[template(path = "kiosk/user.html")]
struct UserTemplate {
    ctx: PageCtx,
    user: User,
    articles: Vec<Article>,
    status: StatusView,
    presets: Vec<Preset>,
    max_deposit_cents: i64,
}

#[derive(Template)]
#[template(path = "kiosk/status.html")]
struct StatusTemplate {
    ctx: PageCtx,
    user: User,
    status: StatusView,
}

/// Baut den Statusbereich aus dem aktuellen Datenbankstand.
async fn status_view(
    state: &AppState,
    t: &Translator,
    user_id: i64,
    flash: Option<Flash>,
) -> Result<StatusView, AppError> {
    let balance_cents = users::get(&state.pool, user_id)
        .await?
        .map(|u| u.balance_cents)
        .unwrap_or_default();
    let now = Utc::now();
    let window = state.config.undo_window_secs;
    let undo = match ledger::last_undoable(&state.pool, user_id, window, now).await? {
        Some(tx) => {
            let what = match tx.kind {
                TxKind::Purchase => match tx.article_id {
                    Some(aid) => articles::get(&state.pool, aid, t.lang())
                        .await?
                        .map(|a| a.name)
                        .or(tx.article_name.clone())
                        .unwrap_or_default(),
                    None => tx.article_name.clone().unwrap_or_default(),
                },
                _ => t.t("tx-kind-deposit"),
            };
            let age = crate::db::parse_db_time(&tx.created_at)
                .map(|c| (now - c).num_seconds())
                .unwrap_or(window);
            Some(UndoView {
                label: format!("{what} ({})", t.money(tx.amount_cents)),
                seconds_left: (window - age).max(1),
            })
        }
        None => None,
    };
    Ok(StatusView {
        balance_cents,
        min_balance_cents: state.config.min_balance_cents,
        flash,
        undo,
    })
}

async fn user_page(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = guard!(state, jar, headers, id);
    let ctx = ctx
        .for_user(&user, &state.locales)
        .with_idle(state.config.idle_timeout_secs);
    let articles = articles::list_active(&state.pool, ctx.t.lang()).await?;
    let status = status_view(&state, &ctx.t, id, None).await?;
    let presets = state
        .config
        .deposit_presets_cents
        .iter()
        .map(|&cents| Preset {
            value: money::format_plain(cents),
            cents,
        })
        .collect();
    Ok(render(&UserTemplate {
        ctx,
        user,
        articles,
        status,
        presets,
        max_deposit_cents: state.config.max_deposit_cents,
    })?
    .into_response())
}

/// Antwort für htmx: nur den Statusbereich neu rendern.
async fn respond_status(
    state: &AppState,
    ctx: PageCtx,
    user: User,
    flash: Flash,
) -> Result<Response, AppError> {
    let status = status_view(state, &ctx.t, user.id, Some(flash)).await?;
    Ok(render(&StatusTemplate { ctx, user, status })?.into_response())
}

/// Übersetzt Buchungsfehler in eine verständliche Meldung.
fn ledger_error_text(t: &Translator, e: &LedgerError, min_balance: i64) -> Option<String> {
    Some(match e {
        LedgerError::InsufficientBalance { .. } => {
            t.t1("error-insufficient-balance", "min", t.money(min_balance))
        }
        LedgerError::ArticleNotFound | LedgerError::ArticleInactive => {
            t.t("error-article-unavailable")
        }
        LedgerError::UserInactive | LedgerError::UserNotFound => t.t("error-user-not-found"),
        LedgerError::InvalidAmount => t.t("error-amount-invalid"),
        LedgerError::AmountTooLarge => t.t("error-amount-too-large"),
        LedgerError::NothingToUndo | LedgerError::AlreadyReversed | LedgerError::NotReversible => {
            t.t("error-nothing-to-undo")
        }
        LedgerError::ReasonRequired => t.t("error-reason-required"),
        LedgerError::TransactionNotFound => t.t("error-internal"),
        LedgerError::Db(_) => return None,
    })
}

/// Kauf eines Artikels durchführen und Statusbereich zurückgeben.
async fn do_purchase(
    state: &AppState,
    ctx: PageCtx,
    user: User,
    article_id: i64,
) -> Result<Response, AppError> {
    let min = state.config.min_balance_cents;
    let flash = match ledger::purchase(&state.pool, user.id, article_id, min).await {
        Ok(tx) => {
            let name = articles::get(&state.pool, article_id, ctx.t.lang())
                .await?
                .map(|a| a.name)
                .unwrap_or_default();
            let mut args = FluentArgs::new();
            args.set("article", name);
            args.set("price", ctx.t.money(-tx.amount_cents));
            Flash::ok(ctx.t.ta("purchase-ok", Some(&args)))
        }
        Err(e) => Flash::error(ledger_error_text(&ctx.t, &e, min).ok_or(e)?),
    };
    respond_status(state, ctx, user, flash).await
}

async fn buy(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path((id, article_id)): Path<(i64, i64)>,
) -> Result<Response, AppError> {
    let user = guard!(state, jar, headers, id);
    let ctx = ctx.for_user(&user, &state.locales);
    do_purchase(&state, ctx, user, article_id).await
}

#[derive(Deserialize)]
struct ScanForm {
    code: String,
}

async fn scan(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Form(form): Form<ScanForm>,
) -> Result<Response, AppError> {
    let user = guard!(state, jar, headers, id);
    let ctx = ctx.for_user(&user, &state.locales);
    let code = articles::normalize_barcode(&form.code);
    match articles::find_by_barcode(&state.pool, &code).await? {
        Some(article_id) => do_purchase(&state, ctx, user, article_id).await,
        None => {
            let text = ctx.t.t1(
                "scan-unknown",
                "code",
                code.chars().take(64).collect::<String>(),
            );
            respond_status(&state, ctx, user, Flash::error(text)).await
        }
    }
}

#[derive(Deserialize)]
struct DepositForm {
    amount: String,
}

async fn deposit(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Form(form): Form<DepositForm>,
) -> Result<Response, AppError> {
    let user = guard!(state, jar, headers, id);
    let ctx = ctx.for_user(&user, &state.locales);
    let flash = match money::parse_cents(&form.amount) {
        Err(_) => Flash::error(ctx.t.t("error-amount-invalid")),
        Ok(cents) => {
            match ledger::deposit(&state.pool, id, cents, state.config.max_deposit_cents).await {
                Ok(tx) => Flash::ok(
                    ctx.t
                        .t1("deposit-ok", "amount", ctx.t.money(tx.amount_cents)),
                ),
                Err(LedgerError::AmountTooLarge) => Flash::error(ctx.t.t1(
                    "error-deposit-max",
                    "max",
                    ctx.t.money(state.config.max_deposit_cents),
                )),
                Err(e) => Flash::error(
                    ledger_error_text(&ctx.t, &e, state.config.min_balance_cents).ok_or(e)?,
                ),
            }
        }
    };
    respond_status(&state, ctx, user, flash).await
}

async fn undo(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let user = guard!(state, jar, headers, id);
    let ctx = ctx.for_user(&user, &state.locales);
    let flash =
        match ledger::undo_last(&state.pool, id, state.config.undo_window_secs, Utc::now()).await {
            Ok(tx) => Flash::ok(ctx.t.t1("undo-ok", "amount", ctx.t.money(tx.amount_cents))),
            Err(e) => Flash::error(
                ledger_error_text(&ctx.t, &e, state.config.min_balance_cents).ok_or(e)?,
            ),
        };
    respond_status(&state, ctx, user, flash).await
}

// ---------------------------------------------------------------------------
// Einstellungen: PIN und Sprache
// ---------------------------------------------------------------------------

#[derive(Template)]
#[template(path = "kiosk/settings.html")]
struct SettingsTemplate {
    ctx: PageCtx,
    user: User,
    message: Option<Flash>,
}

async fn render_settings(
    state: &AppState,
    ctx: PageCtx,
    user_id: i64,
    message: Option<Flash>,
) -> Result<Response, AppError> {
    let user = users::get(&state.pool, user_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let ctx = ctx
        .for_user(&user, &state.locales)
        .with_idle(state.config.idle_timeout_secs * 2);
    Ok(render(&SettingsTemplate { ctx, user, message })?.into_response())
}

async fn settings(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let _user = guard!(state, jar, headers, id);
    render_settings(&state, ctx, id, None).await
}

#[derive(Deserialize)]
struct PinChangeForm {
    #[serde(default)]
    pin: String,
    #[serde(default)]
    pin_repeat: String,
    /// gesetzt, wenn der Knopf „PIN entfernen“ gedrückt wurde
    remove: Option<String>,
}

async fn settings_pin(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Form(form): Form<PinChangeForm>,
) -> Result<Response, AppError> {
    let user = guard!(state, jar, headers, id);
    let t = ctx.clone().for_user(&user, &state.locales).t;
    let result = if form.remove.is_some() {
        users::set_pin(&state.pool, id, None).await
    } else if form.pin.trim() != form.pin_repeat.trim() {
        Err(UserError::PinMismatch)
    } else {
        users::set_pin(&state.pool, id, Some(form.pin.trim())).await
    };
    let mut jar = jar;
    let message = match result {
        Ok(()) => {
            if form.remove.is_none() {
                // Neue PIN: Gerät bleibt für diesen Benutzer entsperrt
                if let Some(old) = jar.get(KIOSK_COOKIE) {
                    sessions::delete(&state.pool, old.value()).await?;
                }
                let token =
                    sessions::create(&state.pool, Owner::User(id), kiosk_ttl(&state)).await?;
                jar = jar.add(ctx.cookie(KIOSK_COOKIE, token, SameSite::Strict));
            }
            Flash::ok(t.t(if form.remove.is_some() {
                "pin-removed"
            } else {
                "pin-saved"
            }))
        }
        Err(e) => Flash::error(user_error_text(&t, &e)),
    };
    let page = render_settings(&state, ctx, id, Some(message)).await?;
    Ok((jar, page).into_response())
}

#[derive(Deserialize)]
struct LanguageForm {
    /// Sprachcode oder leer für „Sprache des Geräts“
    language: String,
}

async fn settings_language(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Form(form): Form<LanguageForm>,
) -> Result<Response, AppError> {
    let _user = guard!(state, jar, headers, id);
    let lang = Some(form.language.as_str()).filter(|l| state.locales.is_available(l));
    users::set_language(&state.pool, id, lang).await?;
    let user = users::get(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let t = ctx.clone().for_user(&user, &state.locales).t;
    render_settings(&state, ctx, id, Some(Flash::ok(t.t("language-saved")))).await
}

// ---------------------------------------------------------------------------
// Artikelbilder
// ---------------------------------------------------------------------------

async fn article_image(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let (mime, data) = articles::get_image(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut res = data.into_response();
    let h = res.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&mime) {
        h.insert(header::CONTENT_TYPE, v);
    }
    // Die URL enthält eine Version (?v=...), daher darf lange gecacht werden
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::user_id_from_path;

    #[test]
    fn parses_user_paths() {
        assert_eq!(user_id_from_path("/u/42"), Some(42));
        assert_eq!(user_id_from_path("/u/7/settings"), Some(7));
        assert_eq!(user_id_from_path("/u/7?x=1"), Some(7));
        assert_eq!(user_id_from_path("/admin"), None);
        assert_eq!(user_id_from_path("/u/abc"), None);
    }
}
