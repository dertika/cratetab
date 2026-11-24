//! Admin-Seite „Einstellungen“ unter `/admin/settings`.
//!
//! Zeigt die aktuell gültigen Werte und bei jedem Feld, ob er im Admin-Bereich
//! gespeichert ist oder aus der Konfiguration (Umgebungsvariable) stammt.
//! Gespeicherte Werte gelten sofort, ohne Neustart.

use askama::Template;
use axum::{
    Form, Router,
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{AppError, AppState, PageCtx, admin::AdminSession, render};
use crate::settings::{self, Settings, SettingsInput};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings", get(page).post(save))
        .route("/settings/reset", post(reset))
}

#[derive(Template)]
#[template(path = "admin/settings.html")]
struct SettingsTemplate {
    ctx: PageCtx,
    /// Gültige Einstellungen (für die Herkunft der Werte)
    current: Settings,
    /// Werte im Formular (bei einem Fehler die Eingaben)
    form: SettingsInput,
    error: Option<String>,
    saved: bool,
}

#[derive(Deserialize)]
struct SavedQuery {
    saved: Option<String>,
}

async fn page(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Query(q): Query<SavedQuery>,
) -> Result<Response, AppError> {
    let current = state.settings();
    let form = SettingsInput::from_settings(&current);
    Ok(render(&SettingsTemplate {
        ctx: ctx.admin(&session),
        current,
        form,
        error: None,
        saved: q.saved.is_some(),
    })?
    .into_response())
}

#[derive(Deserialize)]
struct SettingsForm {
    club_name: String,
    min_balance: String,
    #[serde(default)]
    max_balance: String,
    max_deposit: String,
    #[serde(default)]
    deposit_presets: String,
    self_registration: Option<String>,
    idle_timeout: String,
    undo_window: String,
    default_language: String,
}

async fn save(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(f): Form<SettingsForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    let input = SettingsInput {
        club_name: f.club_name,
        min_balance: f.min_balance,
        max_balance: f.max_balance,
        max_deposit: f.max_deposit,
        deposit_presets: f.deposit_presets,
        self_registration: f.self_registration.is_some(),
        idle_timeout: f.idle_timeout,
        undo_window: f.undo_window,
        default_language: f.default_language,
    };
    let current = state.settings();
    match input.validate(&current, |l| state.locales.is_available(l)) {
        Ok(new) => {
            settings::save(&state.pool, &new, &state.config, session.admin.id).await?;
            state.reload_settings().await?;
            tracing::info!(
                "Admin „{}“ hat die Einstellungen geändert",
                session.admin.username
            );
            Ok(Redirect::to("/admin/settings?saved=1").into_response())
        }
        Err(e) => {
            let error = Some(ctx.t.t(e.key()));
            Ok(render(&SettingsTemplate {
                ctx,
                current,
                form: input,
                error,
                saved: false,
            })?
            .into_response())
        }
    }
}

/// Alle gespeicherten Werte löschen: Danach gilt wieder die Konfiguration.
async fn reset(State(state): State<AppState>, session: AdminSession) -> Result<Response, AppError> {
    settings::reset(&state.pool).await?;
    state.reload_settings().await?;
    tracing::info!(
        "Admin „{}“ hat die Einstellungen auf die Konfiguration zurückgesetzt",
        session.admin.username
    );
    Ok(Redirect::to("/admin/settings?saved=1").into_response())
}
