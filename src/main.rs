//! Startpunkt von Crate Tab.
//!
//! Ohne Argumente startet der Webserver. Daneben gibt es ein paar
//! Hilfsbefehle für den Betrieb, siehe `cratetab help`.

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result, bail};
use cratetab::{admins, config::Config, db, ledger, seed, sessions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing_subscriber::EnvFilter;

const HELP: &str = "\
Crate Tab – Die Getränkekasse für euren Verein, geschrieben in Rust 🦀

Aufruf:
  cratetab [serve]                         Webserver starten (Standard)
  cratetab healthcheck                     Prüfen, ob der Server antwortet (für Docker)
  cratetab backup <ziel.db>                Konsistente Sicherung der Datenbank anlegen
  cratetab seed-demo                       Demo-Daten anlegen (nur in leerer Datenbank)
  cratetab check-balances [--repair]       Guthaben aus den Buchungen nachrechnen
  cratetab set-admin-password <name> <pw>  Admin-Passwort setzen (legt den Admin bei Bedarf an)
  cratetab help                            Diese Hilfe

Die Konfiguration erfolgt über Umgebungsvariablen (CRATETAB_*), siehe README.
";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("CRATETAB_LOG")
                .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        // Farben nur im Terminal, nicht in Docker-Logs
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = Config::from_env()?;

    match args.first().map(String::as_str) {
        None | Some("serve") => serve(config).await,
        Some("healthcheck") => healthcheck(&config).await,
        Some("backup") => {
            let target = args
                .get(1)
                .context("Ziel-Datei fehlt: cratetab backup <ziel.db>")?;
            let pool = db::connect(&config.database_path).await?;
            db::backup(&pool, &PathBuf::from(target)).await?;
            println!("Sicherung geschrieben: {target}");
            Ok(())
        }
        Some("seed-demo") => {
            let pool = db::connect(&config.database_path).await?;
            if seed::seed_if_empty(&pool).await? {
                println!("Demo-Daten angelegt.");
            } else {
                println!("Datenbank enthält bereits Benutzer – keine Demo-Daten angelegt.");
            }
            Ok(())
        }
        Some("check-balances") => {
            let pool = db::connect(&config.database_path).await?;
            let mismatches = ledger::verify_balances(&pool).await?;
            if mismatches.is_empty() {
                println!("Alle Guthaben stimmen mit den Buchungen überein.");
                return Ok(());
            }
            for m in &mismatches {
                println!(
                    "{} (#{}): gespeichert {} Cent, laut Buchungen {} Cent",
                    m.name, m.user_id, m.stored_cents, m.computed_cents
                );
            }
            if args.get(1).map(String::as_str) == Some("--repair") {
                let n = ledger::repair_balances(&pool).await?;
                println!("{n} Guthaben korrigiert.");
                Ok(())
            } else {
                bail!(
                    "{} Abweichung(en) gefunden (mit --repair korrigieren)",
                    mismatches.len()
                )
            }
        }
        Some("set-admin-password") => {
            let (Some(name), Some(pw)) = (args.get(1), args.get(2)) else {
                bail!("Aufruf: cratetab set-admin-password <name> <passwort>");
            };
            let pool = db::connect(&config.database_path).await?;
            let existing: Option<i64> =
                sqlx::query_scalar("SELECT id FROM admins WHERE username = ?")
                    .bind(name)
                    .fetch_optional(&pool)
                    .await?;
            match existing {
                Some(id) => {
                    admins::set_password(&pool, id, pw).await?;
                    admins::set_active(&pool, id, true).await?;
                    println!("Passwort für „{name}“ geändert.");
                }
                None => {
                    admins::create(&pool, name, pw).await?;
                    println!("Admin „{name}“ angelegt.");
                }
            }
            Ok(())
        }
        Some("help" | "--help" | "-h") => {
            print!("{HELP}");
            Ok(())
        }
        Some(other) => {
            eprint!("Unbekannter Befehl „{other}“\n\n{HELP}");
            std::process::exit(2);
        }
    }
}

/// Startet den Webserver und läuft, bis SIGTERM oder Strg+C kommt.
async fn serve(config: Config) -> Result<()> {
    let listen = config.listen;
    tracing::info!(
        "Crate Tab {} für „{}“, Datenbank {}",
        env!("CARGO_PKG_VERSION"),
        config.club_name,
        config.database_path.display()
    );
    let state = cratetab::build_state(config).await?;

    // Abgelaufene Sitzungen regelmäßig aufräumen
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            if let Err(e) = sessions::cleanup(&pool).await {
                tracing::warn!("Sitzungen aufräumen: {e}");
            }
        }
    });

    let app = cratetab::web::router(state.clone());
    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .with_context(|| format!("Port {listen} öffnen"))?;
    tracing::info!("Server läuft auf http://{listen}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Datenbank sauber schließen (schreibt das WAL in die Hauptdatei zurück)
    state.pool.close().await;
    tracing::info!("Server beendet");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Fragt `/health` auf dem eigenen Port ab. Exit-Code 0 = gesund.
/// Kommt ohne curl/wget aus, damit das Docker-Image minimal bleiben kann.
async fn healthcheck(config: &Config) -> Result<()> {
    let addr = format!("127.0.0.1:{}", config.listen.port());
    let check = async {
        let mut stream = tokio::net::TcpStream::connect(&addr).await?;
        stream
            .write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n")
            .await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        Ok::<_, std::io::Error>(buf)
    };
    let response = tokio::time::timeout(Duration::from_secs(5), check)
        .await
        .context("Zeitüberschreitung")??;
    let text = String::from_utf8_lossy(&response);
    if text.starts_with("HTTP/1.0 200") || text.starts_with("HTTP/1.1 200") {
        println!("ok");
        Ok(())
    } else {
        bail!("ungesund: {}", text.lines().next().unwrap_or_default())
    }
}
