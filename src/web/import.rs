//! Admin-Seite „Import“: fnordcredit-Export hochladen, Probelauf ansehen, bestätigen.
//!
//! Ablauf:
//! 1. `POST /admin/import` nimmt die Datei(en) entgegen, liest sie im Speicher
//!    (siehe [`fnordcredit::load_upload`]) und macht einen Probelauf.
//! 2. Der gelesene Export bleibt bis zu 30 Minuten im Arbeitsspeicher, die
//!    Seite zeigt den Bericht und einen Knopf „Jetzt importieren“.
//! 3. `POST /admin/import/confirm` führt den Import mit genau diesen Daten aus.

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
    fnordcredit::{self, Export, ImportReport},
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

async fn show(
    state: &AppState,
    ctx: PageCtx,
    report: Option<ImportReport>,
    files: Vec<String>,
    token: Option<String>,
    error: Option<String>,
) -> Result<Response, AppError> {
    let counts = counts(state).await?;
    Ok(render(&ImportTemplate {
        ctx,
        counts,
        report,
        files,
        token,
        error,
    })?
    .into_response())
}

async fn page(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
) -> Result<Response, AppError> {
    show(&state, ctx.admin(&session), None, Vec::new(), None, None).await
}

async fn upload(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);

    let mut uploads = Vec::new();
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                if field.name() != Some("files") {
                    continue;
                }
                let name = field.file_name().unwrap_or("upload").to_string();
                match field.bytes().await {
                    Ok(data) if !data.is_empty() => uploads.push((name, data.to_vec())),
                    Ok(_) => {}
                    Err(e) => {
                        let error = Some(ctx.t.t1("import-error", "error", e.to_string()));
                        return show(&state, ctx, None, Vec::new(), None, error).await;
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                let error = Some(ctx.t.t1("import-error", "error", e.to_string()));
                return show(&state, ctx, None, Vec::new(), None, error).await;
            }
        }
    }
    if uploads.is_empty() {
        let error = Some(ctx.t.t("import-no-file"));
        return show(&state, ctx, None, Vec::new(), None, error).await;
    }

    // Archiv lesen und JSON parsen ist CPU-Arbeit: nicht im async-Thread
    let export = match tokio::task::spawn_blocking(move || fnordcredit::load_upload(uploads)).await
    {
        Ok(Ok(export)) => export,
        Ok(Err(e)) => {
            let error = Some(ctx.t.t1("import-error", "error", format!("{e:#}")));
            return show(&state, ctx, None, Vec::new(), None, error).await;
        }
        Err(e) => return Err(AppError::Internal(format!("Import: {e}"))),
    };

    let report = match fnordcredit::import(&state.pool, &export, true).await {
        Ok(report) => report,
        Err(e) => {
            let error = Some(ctx.t.t1("import-error", "error", format!("{e:#}")));
            return show(&state, ctx, None, export.files, None, error).await;
        }
    };
    tracing::info!(
        "Admin „{}“: Probelauf fnordcredit-Import ({} Benutzer, {} Buchungen)",
        session.admin.username,
        report.users,
        report.transactions
    );

    let token = password::random_token(16);
    let files = export.files.clone();
    if let Ok(mut pending) = state.pending_import.lock() {
        *pending = Some(PendingImport {
            token: token.clone(),
            export,
            created: Instant::now(),
        });
    }
    show(&state, ctx, Some(report), files, Some(token), None).await
}

#[derive(Deserialize)]
struct ConfirmForm {
    token: String,
}

async fn confirm(
    State(state): State<AppState>,
    ctx: PageCtx,
    session: AdminSession,
    Form(form): Form<ConfirmForm>,
) -> Result<Response, AppError> {
    let ctx = ctx.admin(&session);
    // Den wartenden Export herausnehmen: jeder Upload kann nur einmal importiert werden
    let pending = state.pending_import.lock().ok().and_then(|mut p| {
        let ok = p
            .as_ref()
            .is_some_and(|p| p.token == form.token && p.created.elapsed() < PENDING_TTL);
        if ok { p.take() } else { None }
    });
    let Some(pending) = pending else {
        let error = Some(ctx.t.t("import-expired"));
        return show(&state, ctx, None, Vec::new(), None, error).await;
    };

    match fnordcredit::import(&state.pool, &pending.export, false).await {
        Ok(report) => {
            tracing::info!(
                "Admin „{}“ hat aus fnordcredit importiert: {} Benutzer, {} Artikel, {} Buchungen",
                session.admin.username,
                report.users,
                report.articles,
                report.transactions
            );
            show(&state, ctx, Some(report), pending.export.files, None, None).await
        }
        Err(e) => {
            let error = Some(ctx.t.t1("import-error", "error", format!("{e:#}")));
            show(&state, ctx, None, pending.export.files, None, error).await
        }
    }
}
