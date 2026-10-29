//! Web-Schicht: Router, gemeinsamer Seitenkontext, Fehlerbehandlung.
//!
//! Aufbau:
//! * [`kiosk`] – Oberfläche für Mitglieder (Startseite, Buchen, Aufladen …)
//! * [`admin`] – Admin-Bereich unter `/admin`
//! * [`lang`]  – Sprachumschaltung per Flagge
//!
//! Seiten werden serverseitig mit Askama-Templates (`templates/`) gerendert.
//! Für Interaktivität ohne Neuladen sorgt htmx: Buttons schicken z. B. einen
//! POST und ersetzen nur einen Teil der Seite durch die Antwort.

pub mod admin;
pub mod kiosk;
pub mod lang;

use std::sync::Arc;

use askama::Template;
use axum::{
    Router,
    extract::{FromRequestParts, OriginalUri, Request, State},
    http::{HeaderName, HeaderValue, Method, StatusCode, header, request::Parts},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

use crate::{
    assets,
    config::Config,
    i18n::{Locales, Translator},
    users::User,
};

/// Name des Cookies für die gewählte Sprache
pub const LANG_COOKIE: &str = "lang";

/// Gemeinsamer Zustand aller Handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub locales: Arc<Locales>,
}

/// Baut den kompletten Router der Anwendung.
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(kiosk::routes())
        .nest("/admin", admin::routes(&state.config))
        .route("/lang/{code}", get(lang::switch))
        .route("/static/{*path}", get(assets::serve))
        .route("/favicon.ico", get(favicon))
        .route("/health", get(health))
        .fallback(not_found)
        .layer(middleware::from_fn(check_origin))
        .layer(middleware::from_fn(security_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// `GET /health` – für Docker-Healthcheck und Monitoring.
async fn health(State(state): State<AppState>) -> Response {
    match sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (StatusCode::OK, "ok").into_response(),
        Err(e) => {
            tracing::error!("Healthcheck: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "database error").into_response()
        }
    }
}

async fn favicon() -> Response {
    assets::serve(axum::extract::Path("img/logo.svg".to_string())).await
}

async fn not_found() -> AppError {
    AppError::NotFound
}

/// Schutz gegen Cross-Site-Requests (CSRF): Formulare dürfen nur von Seiten
/// dieses Servers abgeschickt werden. Browser senden bei POST den
/// `Origin`-Header mit; passt er nicht zum `Host`, wird abgelehnt.
/// (Mitglieder ohne PIN haben kein Cookie, SameSite allein schützt sie nicht.)
async fn check_origin(req: Request, next: Next) -> Response {
    if req.method() != Method::GET && req.method() != Method::HEAD {
        let header = |name| {
            req.headers()
                .get(name)
                .and_then(|v: &HeaderValue| v.to_str().ok())
        };
        if let (Some(origin), Some(host)) = (header(header::ORIGIN), header(header::HOST)) {
            // "https://getraenke.example.org" -> "getraenke.example.org"
            let origin_host = origin.split("://").nth(1).unwrap_or(origin);
            let forwarded = header(HeaderName::from_static("x-forwarded-host"));
            if origin_host != host && Some(origin_host) != forwarded {
                tracing::warn!("Anfrage von fremder Herkunft abgelehnt: {origin}");
                return StatusCode::FORBIDDEN.into_response();
            }
        }
    }
    next.run(req).await
}

/// Setzt Sicherheits-Header für alle Antworten.
///
/// Die Content-Security-Policy erlaubt nur Skripte und Styles vom eigenen
/// Server – deshalb steht im HTML kein Inline-JavaScript.
async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; \
             style-src 'self'; script-src 'self'; frame-ancestors 'none'; base-uri 'self'; \
             form-action 'self'",
        ),
    );
    h.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    h.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    h.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(self), microphone=(), geolocation=()"),
    );
    res
}

/// Informationen, die jede Seite braucht: Übersetzer, Vereinsname, aktuelle
/// URL (für die Sprachumschaltung) usw. Wird automatisch aus der Anfrage gebaut,
/// indem ein Handler `ctx: PageCtx` als Parameter nimmt.
#[derive(Clone)]
pub struct PageCtx {
    pub t: Translator,
    pub club_name: String,
    /// Aktueller Pfad inkl. Query, z. B. `/admin/users?x=1`
    pub path: String,
    /// Läuft die Anfrage über HTTPS (laut Reverse Proxy)?
    pub secure: bool,
    /// Kiosk-Seiten: nach so vielen Sekunden Inaktivität zurück zur Startseite
    pub idle_timeout: Option<u64>,
    /// Angemeldeter Admin (für die Navigation im Admin-Bereich)
    pub admin_name: Option<String>,
    /// Asset-Version für Cache-Busting
    pub version: &'static str,
}

impl PageCtx {
    /// Link zum Umschalten auf eine andere Sprache; danach geht es zurück zur aktuellen Seite.
    pub fn lang_url(&self, code: impl AsRef<str>) -> String {
        format!(
            "/lang/{}?next={}",
            code.as_ref(),
            percent_encode(&self.path)
        )
    }

    /// Wechselt auf die bevorzugte Sprache eines Benutzers, falls er eine hat.
    pub fn for_user(mut self, user: &User, locales: &Arc<Locales>) -> Self {
        if let Some(lang) = &user.language
            && locales.is_available(lang)
        {
            self.t = Translator::new(locales.clone(), lang);
        }
        self
    }

    /// Kiosk-Seiten springen nach Inaktivität zurück zur Startseite.
    pub fn with_idle(mut self, secs: u64) -> Self {
        self.idle_timeout = Some(secs);
        self
    }

    /// Erzeugt ein Cookie mit sicheren Standardwerten.
    pub fn cookie(
        &self,
        name: &'static str,
        value: String,
        same_site: SameSite,
    ) -> Cookie<'static> {
        Cookie::build((name, value))
            .path("/")
            .http_only(true)
            .secure(self.secure)
            .same_site(same_site)
            .build()
    }
}

impl FromRequestParts<AppState> for PageCtx {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        // 1. Cookie, 2. Accept-Language des Browsers, 3. Standardsprache
        let lang = jar
            .get(LANG_COOKIE)
            .map(|c| c.value().to_string())
            .filter(|l| state.locales.is_available(l))
            .unwrap_or_else(|| {
                let accept = parts
                    .headers
                    .get(header::ACCEPT_LANGUAGE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                state.locales.negotiate(accept)
            });
        let secure = parts
            .headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("https"));
        let path = original_path(parts);
        Ok(PageCtx {
            t: Translator::new(state.locales.clone(), &lang),
            club_name: state.config.club_name.clone(),
            path,
            secure,
            idle_timeout: None,
            admin_name: None,
            version: assets::VERSION,
        })
    }
}

/// Vollständiger Pfad der Anfrage inkl. Query. In verschachtelten Routern
/// (`/admin`) enthält `parts.uri` nur den Rest, daher die Original-URI.
pub fn original_path(parts: &Parts) -> String {
    let uri = parts
        .extensions
        .get::<OriginalUri>()
        .map(|o| &o.0)
        .unwrap_or(&parts.uri);
    uri.path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "/".into())
}

/// Kodiert einen Pfad für die Verwendung als Query-Parameter.
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Ist `next` ein sicheres Weiterleitungsziel (nur Pfade auf diesem Server)?
pub fn is_local_path(next: &str) -> bool {
    next.starts_with('/') && !next.starts_with("//") && !next.contains('\\')
}

/// Rendert ein Template zu einer HTML-Antwort.
pub fn render(tpl: &impl Template) -> Result<Html<String>, AppError> {
    Ok(Html(tpl.render()?))
}

/// Fehler, die in Handlern auftreten können.
#[derive(Debug)]
pub enum AppError {
    NotFound,
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(format!("Datenbank: {e}"))
    }
}

impl From<askama::Error> for AppError {
    fn from(e: askama::Error) -> Self {
        AppError::Internal(format!("Template: {e}"))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(format!("{e:#}"))
    }
}

impl From<crate::ledger::LedgerError> for AppError {
    fn from(e: crate::ledger::LedgerError) -> Self {
        AppError::Internal(format!("Buchung: {e}"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => (
                StatusCode::NOT_FOUND,
                Html(ERROR_PAGE.replace("{msg}", "404")),
            )
                .into_response(),
            AppError::Internal(msg) => {
                tracing::error!("{msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(ERROR_PAGE.replace("{msg}", "500")),
                )
                    .into_response()
            }
        }
    }
}

/// Minimale Fehlerseite (bewusst ohne Template, damit sie immer funktioniert).
const ERROR_PAGE: &str = r#"<!doctype html><html><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Crate Tab</title><link rel="stylesheet" href="/static/app.css"></head>
<body><main class="fault-page"><img src="/static/img/logo.svg" alt="" width="96" height="96">
<h1>{msg}</h1><p><a class="btn" href="/">⟵ Crate Tab</a></p></main></body></html>"#;
