//! Sprachumschaltung: `GET /lang/{code}?next=/pfad`
//!
//! Merkt die Sprache in einem Cookie (ein Jahr gültig) und leitet zurück zur
//! vorherigen Seite. Ist dabei ein Kiosk-Benutzer angemeldet (Seite `/u/{id}…`),
//! wird die Sprache außerdem als seine bevorzugte Sprache gespeichert.

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{CookieJar, SameSite};
use serde::Deserialize;

use super::{AppState, LANG_COOKIE, PageCtx, is_local_path, kiosk};
use crate::users;

#[derive(Deserialize)]
pub struct SwitchQuery {
    next: Option<String>,
}

pub async fn switch(
    State(state): State<AppState>,
    ctx: PageCtx,
    jar: CookieJar,
    Path(code): Path<String>,
    Query(q): Query<SwitchQuery>,
) -> Response {
    let next = q
        .next
        .filter(|n| is_local_path(n))
        .unwrap_or_else(|| "/".into());
    if !state.locales.is_available(&code) {
        return Redirect::to(&next).into_response();
    }

    // Auf einer Benutzerseite? Dann Sprache für diesen Benutzer merken.
    if let Some(user_id) = kiosk::user_id_from_path(&next)
        && let Ok(Some(user)) = kiosk::authorized_user(&state, &jar, user_id).await
        && let Err(e) = users::set_language(&state.pool, user.id, Some(&code)).await
    {
        tracing::warn!("Sprache für Benutzer {user_id} speichern: {e}");
    }

    let mut cookie = ctx.cookie(LANG_COOKIE, code, SameSite::Lax);
    cookie.set_max_age(cookie::time::Duration::days(365));
    (jar.add(cookie), Redirect::to(&next)).into_response()
}
