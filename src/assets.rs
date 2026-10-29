//! Statische Dateien (CSS, JavaScript, Flaggen, Logo, htmx, zxing).
//!
//! Alles unter `assets/` wird beim Kompilieren in die Binärdatei eingebettet.
//! Es werden keine externen Server (CDNs) benötigt.

use axum::{
    extract::Path,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
#[exclude = "*.LICENSE"]
pub struct Assets;

/// Version für Cache-Busting: `/static/app.css?v=0.1.0`
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Liefert eine eingebettete Datei aus: `GET /static/{*path}`
pub async fn serve(Path(path): Path<String>) -> Response {
    let Some(file) = Assets::get(&path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let mut res = file.data.into_owned().into_response();
    let headers = res.headers_mut();
    if let Ok(v) = HeaderValue::from_str(mime.as_ref()) {
        headers.insert(header::CONTENT_TYPE, v);
    }
    // Dateien ändern sich nur mit einer neuen Version (URLs tragen ?v=...)
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    res
}
