//! Admin-Seite „Import“: fnordcredit-Export hochladen, Probelauf ansehen, bestätigen.
//!
//! Ablauf:
//! 1. `POST /admin/import` nimmt die Datei(en) entgegen, liest sie im Speicher
//!    (siehe [`fnordcredit::load_upload`]) und macht einen Probelauf.
//! 2. Der gelesene Export bleibt bis zu 30 Minuten im Arbeitsspeicher, die
//!    Seite zeigt den Bericht und einen Knopf „Jetzt importieren“.
//! 3. `POST /admin/import/confirm` führt den Import mit genau diesen Daten aus.
//!
//! Ist die Datenbank nicht leer, kann man „Ersetzen“ ankreuzen: Dann werden
//! Benutzer, Artikel und Buchungen gelöscht und durch den Import ersetzt
//! (Admin-Zugänge bleiben). Vorher legt Crate Tab eine Sicherung an; Löschen
//! und Import laufen in einer Datenbank-Transaktion.

use std::time::{Duration, Instant};

use askama::Template;
use axum::{
    Form, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;

use super::{AppError, AppState, PageCtx, admin::AdminSession, render};
use crate::{
    fnordcredit::{self, Export, ImportMode, ImportReport},
    password,
};

/// Wie lange ein hochgeladener Export auf die Bestätigung wartet.
const PENDING_TTL: Duration = Duration::from_secs(30 * 60);
/// Höchstgröße des Uploads.
const UPLOAD_LIMIT: usize = 64 * 1024 * 1024;

/// Ein hochgeladener Export, der auf die Bestätigung wartet.
pub struct PendingImport {
    token: String,
    export: Export,
    mode: ImportMode,
    created: Instant,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/import",
            get(page)
                .post(upload)
                .layer(DefaultBodyLimit::max(UPLOAD_LIMIT)),
        )
        .route("/import/confirm", post(confirm))
}

/// Anzahl der vorhandenen Datensätze (der Import geht nur in eine leere Datenbank).
#[derive(Debug, Default, sqlx::FromRow)]
struct DbCounts {
    users: i64,
    articles: i64,
    transactions: i64,
}

impl DbCounts {
    fn is_empty(&self) -> bool {
        self.users + self.articles + self.transactions == 0
    }
}

#[derive(Template)]
#[template(path = "admin/import.html")]
struct ImportTemplate {
    ctx: PageCtx,
    counts: DbCounts,
    report: Option<ImportReport>,
    files: Vec<String>,
    /// Gesetzt nach einem erfolgreichen Probelauf (für den Bestätigen-Knopf)
    token: Option<String>,
    /// Ersetzt der Import vorhandene Daten?
    replace: bool,
    error: Option<String>,
}

async fn counts(state: &AppState) -> Result<DbCounts, AppError> {
    Ok(sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM users) AS users, (SELECT COUNT(*) FROM articles) AS articles,
                (SELECT COUNT(*) FROM transactions) AS transactions",
    )
    .fetch_one(&state.pool)
    .await?)
}

/// Was die Seite anzeigt (Bericht, Bestätigung, Fehler).
#[derive(Default)]
struct View {
    report: Option<ImportReport>,
    files: Vec<String>,
    token: Option<String>,
    replace: bool,
    error: Option<String>,
}

impl View {
    fn error(error: String) -> Self {
        View {
            error: Some(error),
            ..Default::default()
        }
    }
}

async fn show(state: &AppState, ctx: PageCtx, view: View) -> Result<Response, AppError> {
    let counts = counts(state).await?;
    Ok(render(&ImportTemplate {
        ctx,
        counts,
        report: view.report,
        files: view.files,
        token: view.token,
        replace: view.replace,
        error: view.error,
    })?
    .into_response())
}

async fn page(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
) -> Result<Response, AppError> {
    show(&state, ctx.admin(&session), View::default()).await
}

async fn upload(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);

    let mut uploads = Vec::new();
    let mut mode = ImportMode::EmptyOnly;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if field.name() == Some("replace") {
                    mode = ImportMode::Replace;
                    continue;
                }
                if field.name() != Some("files") {
                    continue;
                }
                let name = field.file_name().unwrap_or("upload").to_string();
                match field.bytes().await {
                    Ok(data) if !data.is_empty() => uploads.push((name, data.to_vec())),
                    Ok(_) => {}
                    Err(e) => {
                        let error = ctx.t.t1("import-error", "error", e.to_string());
                        return show(&state, ctx, View::error(error)).await;
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                let error = ctx.t.t1("import-error", "error", e.to_string());
                return show(&state, ctx, View::error(error)).await;
            }
        }
    }
    if uploads.is_empty() {
        let error = ctx.t.t("import-no-file");
        return show(&state, ctx, View::error(error)).await;
    }

    // Archiv lesen und JSON parsen ist CPU-Arbeit: nicht im async-Thread
    let export = match tokio::task::spawn_blocking(move || fnordcredit::load_upload(uploads)).await
    {
        Ok(Ok(export)) => export,
        Ok(Err(e)) => {
            let error = ctx.t.t1("import-error", "error", format!("{e:#}"));
            return show(&state, ctx, View::error(error)).await;
        }
        Err(e) => return Err(AppError::Internal(format!("Import: {e}"))),
    };

    // Probelauf: rollt am Ende zurück, beim Ersetzen wird also auch nichts gelöscht
    let report = match fnordcredit::import(&state.pool, &export, true, mode).await {
        Ok(report) => report,
        Err(e) => {
            let view = View {
                files: export.files,
                error: Some(ctx.t.t1("import-error", "error", format!("{e:#}"))),
                ..Default::default()
            };
            return show(&state, ctx, view).await;
        }
    };
    tracing::info!(
        "Admin „{}“: Probelauf fnordcredit-Import ({} Benutzer, {} Buchungen, ersetzen: {})",
        session.admin.username,
        report.users,
        report.transactions,
        mode == ImportMode::Replace
    );

    let token = password::random_token(16);
    let files = export.files.clone();
    if let Ok(mut pending) = state.pending_import.lock() {
        *pending = Some(PendingImport {
            token: token.clone(),
            export,
            mode,
            created: Instant::now(),
        });
    }
    let view = View {
        report: Some(report),
        files,
        token: Some(token),
        replace: mode == ImportMode::Replace,
        error: None,
    };
    show(&state, ctx, view).await
}

#[derive(Deserialize)]
struct ConfirmForm {
    token: String,
    /// Ankreuzfeld „Ich weiß, dass die vorhandenen Daten gelöscht werden“
    confirm_replace: Option<String>,
}

async fn confirm(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(form): Form<ConfirmForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);

    // Den wartenden Export herausnehmen: jeder Upload kann nur einmal importiert
    // werden. Fehlt beim Ersetzen das Häkchen, bleibt er liegen.
    let mut missing_ack = false;
    let pending = state.pending_import.lock().ok().and_then(|mut p| {
        let valid = p
            .as_ref()
            .is_some_and(|p| p.token == form.token && p.created.elapsed() < PENDING_TTL);
        if !valid {
            return None;
        }
        if p.as_ref().is_some_and(|p| p.mode == ImportMode::Replace)
            && form.confirm_replace.is_none()
        {
            missing_ack = true;
            return None;
        }
        p.take()
    });
    let Some(pending) = pending else {
        let key = if missing_ack {
            "import-replace-ack-missing"
        } else {
            "import-expired"
        };
        let error = ctx.t.t(key);
        return show(&state, ctx, View::error(error)).await;
    };

    // Vor dem Ersetzen die bisherige Datenbank sichern
    let mut backup = None;
    if pending.mode == ImportMode::Replace {
        match fnordcredit::backup_before_replace(&state.pool, &state.config.database_path).await {
            Ok(path) => {
                tracing::info!("Sicherung vor dem Ersetzen: {}", path.display());
                backup = Some(path.display().to_string());
            }
            Err(e) => {
                let view = View {
                    files: pending.export.files,
                    error: Some(ctx.t.t1("import-backup-failed", "error", format!("{e:#}"))),
                    ..Default::default()
                };
                return show(&state, ctx, view).await;
            }
        }
    }

    match fnordcredit::import(&state.pool, &pending.export, false, pending.mode).await {
        Ok(mut report) => {
            report.backup = backup;
            tracing::info!(
                "Admin „{}“ hat aus fnordcredit importiert: {} Benutzer, {} Artikel, {} Buchungen \
                 (ersetzt: {} Benutzer, {} Artikel, {} Buchungen)",
                session.admin.username,
                report.users,
                report.articles,
                report.transactions,
                report.replaced_users,
                report.replaced_articles,
                report.replaced_transactions
            );
            let view = View {
                report: Some(report),
                files: pending.export.files,
                replace: pending.mode == ImportMode::Replace,
                ..Default::default()
            };
            show(&state, ctx, view).await
        }
        Err(e) => {
            let view = View {
                files: pending.export.files,
                error: Some(ctx.t.t1("import-error", "error", format!("{e:#}"))),
                ..Default::default()
            };
            show(&state, ctx, view).await
        }
    }
}
