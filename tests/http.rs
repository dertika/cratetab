//! Durchstich-Tests über HTTP: Kiosk-Ablauf, PIN, Admin-Login, CSV, Sprachwahl.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use cratetab::{
    articles::{self, ArticleInput},
    config::Config,
    db, users,
    web::{self, AppState},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn app() -> (Router, AppState) {
    let pool = db::connect_memory().await.unwrap();
    // Eigenes Verzeichnis je Test, damit Sicherungen nicht im Projekt landen
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cratetab-http-{}-{n}", std::process::id()));
    let config = Config {
        club_name: "Testverein".into(),
        database_path: dir.join("cratetab.db"),
        admin_initial_password: Some("admin-passwort".into()),
        ..Config::default()
    };
    let state = cratetab::state_with_pool(config, pool).await.unwrap();
    (web::router(state.clone()), state)
}

struct Resp {
    status: StatusCode,
    location: Option<String>,
    cookies: Vec<String>,
    body: String,
}

async fn send(app: &Router, req: Request<Body>) -> Resp {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let location = res
        .headers()
        .get(header::LOCATION)
        .map(|v| v.to_str().unwrap().to_string());
    let cookies = res
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string())
        .collect();
    let body =
        String::from_utf8_lossy(&res.into_body().collect().await.unwrap().to_bytes()).into_owned();
    Resp {
        status,
        location,
        cookies,
        body,
    }
}

fn get(uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::get(uri);
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    b.body(Body::empty()).unwrap()
}

fn post(uri: &str, form: &str, cookie: Option<&str>) -> Request<Body> {
    let mut b = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("hx-request", "true");
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    b.body(Body::from(form.to_string())).unwrap()
}

#[tokio::test]
async fn health_and_assets() {
    let (app, _) = app().await;
    let r = send(&app, get("/health", None)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body, "ok");
    for path in [
        "/static/app.css",
        "/static/app.js",
        "/static/vendor/htmx.min.js",
        "/static/vendor/zxing-browser.min.js",
        "/static/img/logo.svg",
        "/static/flags/de.svg",
    ] {
        assert_eq!(
            send(&app, get(path, None)).await.status,
            StatusCode::OK,
            "{path}"
        );
    }
    assert_eq!(
        send(&app, get("/static/nope.js", None)).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn home_page_language_selection() {
    let (app, state) = app().await;
    users::create(&state.pool, "Ferris", None).await.unwrap();

    // Standard: Deutsch
    let r = send(&app, get("/", None)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body.contains("Crate Tab"));
    assert!(r.body.contains("Die Getränkekasse für euren Verein"));
    assert!(r.body.contains("Testverein"));
    assert!(r.body.contains("Ferris"));
    assert!(r.body.contains(r#"lang="de""#));

    // Accept-Language
    let req = Request::get("/")
        .header(header::ACCEPT_LANGUAGE, "fr-FR,fr;q=0.9")
        .body(Body::empty())
        .unwrap();
    assert!(send(&app, req).await.body.contains("La caisse à boissons"));

    // Cookie hat Vorrang
    let req = Request::get("/")
        .header(header::ACCEPT_LANGUAGE, "fr-FR")
        .header(header::COOKIE, "lang=en")
        .body(Body::empty())
        .unwrap();
    assert!(
        send(&app, req)
            .await
            .body
            .contains("The drinks tab for your club")
    );

    // Umschalten setzt Cookie und leitet nur auf lokale Pfade weiter
    let r = send(&app, get("/lang/en?next=/register", None)).await;
    assert_eq!(r.location.as_deref(), Some("/register"));
    assert!(r.cookies.iter().any(|c| c == "lang=en"));
    let r = send(&app, get("/lang/en?next=//evil.example", None)).await;
    assert_eq!(r.location.as_deref(), Some("/"));
}

#[tokio::test]
async fn kiosk_purchase_flow_with_pin() {
    let (app, state) = app().await;
    let user = users::create(&state.pool, "Ben", Some("1234"))
        .await
        .unwrap();
    let mate = articles::save(
        &state.pool,
        None,
        &ArticleInput {
            name: "Mate".into(),
            price_cents: 150,
            active: true,
            barcodes: vec!["4029764001807".into()],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Ohne PIN-Eingabe: Umleitung zur PIN-Abfrage
    let r = send(&app, get(&format!("/u/{}", user.id), None)).await;
    assert_eq!(r.location, Some(format!("/u/{}/pin", user.id)));
    let r = send(&app, post(&format!("/u/{}/buy/{mate}", user.id), "", None)).await;
    assert!(r.body.is_empty(), "htmx-Anfrage ohne Zugriff bucht nichts");

    // Falsche PIN
    let r = send(&app, post(&format!("/u/{}/pin", user.id), "pin=0000", None)).await;
    assert!(r.body.contains("PIN ist falsch"));

    // Richtige PIN -> Sitzung
    let r = send(&app, post(&format!("/u/{}/pin", user.id), "pin=1234", None)).await;
    assert_eq!(r.location, Some(format!("/u/{}", user.id)));
    let cookie = r
        .cookies
        .iter()
        .find(|c| c.starts_with("kiosk_session="))
        .unwrap()
        .clone();

    let r = send(&app, get(&format!("/u/{}", user.id), Some(&cookie))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body.contains("Hallo Ben!"));

    // Aufladen und kaufen
    let r = send(
        &app,
        post(
            &format!("/u/{}/deposit", user.id),
            "amount=5%2C00",
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("5,00 €"), "{}", r.body);
    let r = send(
        &app,
        post(&format!("/u/{}/buy/{mate}", user.id), "", Some(&cookie)),
    )
    .await;
    assert!(r.body.contains("Mate gebucht"));
    assert!(r.body.contains("3,50 €"));
    assert!(r.body.contains("Rückgängig"));

    // Per Barcode kaufen, unbekannter Barcode
    let r = send(
        &app,
        post(
            &format!("/u/{}/scan", user.id),
            "code=4029764001807",
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("2,00 €"));
    let r = send(
        &app,
        post(
            &format!("/u/{}/scan", user.id),
            "code=123456789",
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("Unbekannter Barcode 123456789"));

    // Rückgängig
    let r = send(
        &app,
        post(&format!("/u/{}/undo", user.id), "", Some(&cookie)),
    )
    .await;
    assert!(r.body.contains("Buchung storniert"));
    assert!(r.body.contains("3,50 €"));

    // Startseite beendet die Sitzung
    send(&app, get("/", Some(&cookie))).await;
    let r = send(&app, get(&format!("/u/{}", user.id), Some(&cookie))).await;
    assert_eq!(r.location, Some(format!("/u/{}/pin", user.id)));
}

#[tokio::test]
async fn self_registration() {
    let (app, state) = app().await;
    let r = send(
        &app,
        post("/register", "name=Neu&pin=1234&pin_repeat=1234", None),
    )
    .await;
    let user = users::list_active(&state.pool)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(user.name, "Neu");
    assert!(user.has_pin());
    assert_eq!(r.location, Some(format!("/u/{}", user.id)));
    assert!(r.cookies.iter().any(|c| c.starts_with("kiosk_session=")));

    let r = send(&app, post("/register", "name=neu", None)).await;
    assert!(r.body.contains("schon vergeben"));
    let r = send(
        &app,
        post("/register", "name=X&pin=1234&pin_repeat=9999", None),
    )
    .await;
    assert!(r.body.contains("stimmen nicht überein"));
}

#[tokio::test]
async fn admin_login_and_csv() {
    let (app, state) = app().await;
    let user = users::create(&state.pool, "Clara", None).await.unwrap();
    cratetab::ledger::deposit(&state.pool, user.id, 1234, 10_000, None)
        .await
        .unwrap();

    // Ohne Anmeldung -> Login
    let r = send(&app, get("/admin/users", None)).await;
    assert_eq!(
        r.location.as_deref(),
        Some("/admin/login?next=/admin/users")
    );

    let r = send(
        &app,
        post("/admin/login", "username=admin&password=falsch", None),
    )
    .await;
    assert!(r.body.contains("Benutzername oder Passwort falsch"));

    let r = send(
        &app,
        post(
            "/admin/login",
            "username=admin&password=admin-passwort&next=/admin",
            None,
        ),
    )
    .await;
    assert_eq!(r.location.as_deref(), Some("/admin"));
    let cookie = r
        .cookies
        .iter()
        .find(|c| c.starts_with("admin_session="))
        .unwrap()
        .clone();

    for page in [
        "/admin",
        "/admin/articles",
        "/admin/articles/new",
        "/admin/users",
        "/admin/transactions",
        "/admin/stats",
        "/admin/admins",
    ] {
        let r = send(&app, get(page, Some(&cookie))).await;
        assert_eq!(r.status, StatusCode::OK, "{page}: {}", r.body);
    }

    // Korrektur ohne Begründung wird abgelehnt
    let r = send(
        &app,
        post(
            &format!("/admin/users/{}/correction", user.id),
            "amount=-1&reason=",
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("Begründung"));
    let r = send(
        &app,
        post(
            &format!("/admin/users/{}/correction", user.id),
            "amount=-2%2C34&reason=Z%C3%A4hlfehler",
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);

    let r = send(&app, get("/admin/transactions.csv", Some(&cookie))).await;
    assert!(r.body.starts_with('\u{feff}'), "BOM für Excel");
    assert!(r.body.contains("Nr;Zeit;Benutzer;Art"));
    assert!(r.body.contains("Clara;Aufladung;;12,34;1234"));
    assert!(
        r.body
            .contains("Clara;Korrektur;;-2,34;-234;Zählfehler;admin")
    );

    // Filter: nur Korrekturen
    let r = send(
        &app,
        get("/admin/transactions.csv?kind=correction", Some(&cookie)),
    )
    .await;
    assert!(!r.body.contains("Aufladung"));
    assert_eq!(
        users::get(&state.pool, user.id)
            .await
            .unwrap()
            .unwrap()
            .balance_cents,
        1000
    );
}

#[tokio::test]
async fn foreign_origin_is_rejected() {
    let (app, state) = app().await;
    let user = users::create(&state.pool, "Ohne PIN", None).await.unwrap();
    let req = Request::post(format!("/u/{}/deposit", user.id))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::HOST, "kasse.example.org")
        .header(header::ORIGIN, "https://evil.example")
        .body(Body::from("amount=5"))
        .unwrap();
    assert_eq!(send(&app, req).await.status, StatusCode::FORBIDDEN);

    let req = Request::post(format!("/u/{}/deposit", user.id))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(header::HOST, "kasse.example.org")
        .header(header::ORIGIN, "https://kasse.example.org")
        .header("hx-request", "true")
        .body(Body::from("amount=5"))
        .unwrap();
    assert_eq!(send(&app, req).await.status, StatusCode::OK);
    assert_eq!(
        users::get(&state.pool, user.id)
            .await
            .unwrap()
            .unwrap()
            .balance_cents,
        500
    );
}

#[tokio::test]
async fn pin_must_be_digits() {
    let (app, state) = app().await;
    let user = users::create(&state.pool, "Zahlen", Some("1234"))
        .await
        .unwrap();
    // Buchstaben werden abgelehnt, ohne als Fehlversuch zu zählen
    for _ in 0..10 {
        let r = send(&app, post(&format!("/u/{}/pin", user.id), "pin=abcd", None)).await;
        assert!(r.body.contains("nur aus Ziffern"), "{}", r.body);
    }
    let r = send(&app, post(&format!("/u/{}/pin", user.id), "pin=1234", None)).await;
    assert_eq!(
        r.location,
        Some(format!("/u/{}", user.id)),
        "keine Sperre durch Buchstaben-Eingaben"
    );

    // Registrierung: PIN mit Buchstaben wird abgelehnt
    let r = send(
        &app,
        post("/register", "name=Neu&pin=12ab&pin_repeat=12ab", None),
    )
    .await;
    assert!(r.body.contains("4 bis 8 Ziffern"), "{}", r.body);
}

/// Baut ein .tar.gz wie `tar czf fnordcredit-export.tar.gz fnordcredit-export`.
fn export_archive() -> Vec<u8> {
    let t =
        |e: f64| serde_json::json!({"$reql_type$": "TIME", "epoch_time": e, "timezone": "+00:00"});
    let users = serde_json::json!([
        {"name": "Ada", "credit": 3.5, "lastchanged": t(1_480_000_000.0),
         "pincode": "sha1$2b19e86b$1$7fccf43a2e1bd530ca8c1e573cb6b5b5059ba7d7"},
        {"name": "Bob", "credit": 10, "lastchanged": t(1_420_000_000.0)}
    ]);
    let txs = serde_json::json!([
        {"id": "1", "username": "Ada", "delta": 5, "credit": 5, "time": t(1_479_000_000.0), "product": null},
        {"id": "2", "username": "Ada", "delta": -1.5, "credit": 3.5, "time": t(1_479_000_100.0), "product": "Mate"}
    ]);
    let products = serde_json::json!([{"id": "p", "name": "Mate", "price": 1.5, "ean": "4029764001807", "order": 1}]);

    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    for (name, value) in [
        ("users", users),
        ("transactions", txs),
        ("products", products),
    ] {
        let data = serde_json::to_vec(&value).unwrap();
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(
                &mut header,
                format!("fnordcredit-export/fnordcredit/{name}.json"),
                data.as_slice(),
            )
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

fn multipart_upload(
    uri: &str,
    file_name: &str,
    data: &[u8],
    cookie: &str,
    replace: bool,
) -> Request<Body> {
    let boundary = "----cratetab-test";
    let mut body = Vec::new();
    if replace {
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"replace\"\r\n\r\n1\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"files\"; filename=\"{file_name}\"\r\n\
             Content-Type: application/gzip\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    Request::post(uri)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(header::COOKIE, cookie)
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn admin_upload_import() {
    let (app, state) = app().await;
    let r = send(
        &app,
        post(
            "/admin/login",
            "username=admin&password=admin-passwort",
            None,
        ),
    )
    .await;
    let cookie = r
        .cookies
        .iter()
        .find(|c| c.starts_with("admin_session="))
        .unwrap()
        .clone();

    let r = send(&app, get("/admin/import", Some(&cookie))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body.contains("Hochladen und prüfen"));

    // Upload -> Probelauf, noch nichts gespeichert
    let r = send(
        &app,
        multipart_upload(
            "/admin/import",
            "fnordcredit-export.tar.gz",
            &export_archive(),
            &cookie,
            false,
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(r.body.contains("Probelauf"), "{}", r.body);
    assert!(
        r.body.contains("Ausgleichsbuchungen"),
        "Bob braucht einen Ausgleich"
    );
    assert!(r.body.contains("Die Summen aller Guthaben stimmen überein"));
    assert!(users::list_all(&state.pool).await.unwrap().is_empty());
    let token = r
        .body
        .split("name=\"token\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("Token für die Bestätigung")
        .to_string();

    // Falsches Token
    let r = send(
        &app,
        post("/admin/import/confirm", "token=falsch", Some(&cookie)),
    )
    .await;
    assert!(r.body.contains("abgelaufen"));

    // Bestätigen -> Import
    let r = send(
        &app,
        post(
            "/admin/import/confirm",
            &format!("token={token}"),
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("Import abgeschlossen"), "{}", r.body);
    let users = users::list_all(&state.pool).await.unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users.iter().map(|u| u.balance_cents).sum::<i64>(), 1350);

    // Derselbe Upload kann nicht noch einmal importiert werden
    let r = send(
        &app,
        post(
            "/admin/import/confirm",
            &format!("token={token}"),
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("abgelaufen"));
    // Datenbank ist jetzt nicht mehr leer -> Upload nur noch mit „Ersetzen“
    let r = send(&app, get("/admin/import", Some(&cookie))).await;
    assert!(r.body.contains("Die Datenbank ist nicht leer"));
    assert!(r.body.contains("name=\"replace\""));
    let upload = |replace| {
        multipart_upload(
            "/admin/import",
            "fnordcredit-export.tar.gz",
            &export_archive(),
            &cookie,
            replace,
        )
    };
    let r = send(&app, upload(false)).await;
    assert!(r.body.contains("nicht leer"), "ohne Ersetzen abgelehnt");

    // Ein Benutzer kommt dazu, dann wird ersetzt
    users::create(&state.pool, "Wird gelöscht", None)
        .await
        .unwrap();
    let r = send(&app, upload(true)).await;
    assert!(r.body.contains("Beim Import werden gelöscht"), "{}", r.body);
    assert_eq!(
        users::list_all(&state.pool).await.unwrap().len(),
        3,
        "Probelauf löscht nichts"
    );
    let token = r
        .body
        .split("name=\"token\" value=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap()
        .to_string();
    // Ohne Häkchen: abgelehnt, der Upload bleibt gültig
    let r = send(
        &app,
        post(
            "/admin/import/confirm",
            &format!("token={token}"),
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("Bitte bestätigen"), "{}", r.body);
    let r = send(
        &app,
        post(
            "/admin/import/confirm",
            &format!("token={token}&confirm_replace=1"),
            Some(&cookie),
        ),
    )
    .await;
    assert!(r.body.contains("Import abgeschlossen"), "{}", r.body);
    assert!(
        r.body.contains("Sicherung der bisherigen Datenbank"),
        "{}",
        r.body
    );
    let names: Vec<String> = users::list_all(&state.pool)
        .await
        .unwrap()
        .into_iter()
        .map(|u| u.name)
        .collect();
    assert_eq!(names.len(), 2);
    assert!(!names.contains(&"Wird gelöscht".to_string()));

    // Ohne Anmeldung kein Zugriff
    let r = send(&app, get("/admin/import", None)).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn admin_settings_apply_immediately() {
    let (app, state) = app().await;
    let user = users::create(&state.pool, "Sparfuchs", None).await.unwrap();
    let r = send(
        &app,
        post(
            "/admin/login",
            "username=admin&password=admin-passwort",
            None,
        ),
    )
    .await;
    let cookie = r
        .cookies
        .iter()
        .find(|c| c.starts_with("admin_session="))
        .unwrap()
        .clone();

    let r = send(&app, get("/admin/settings", Some(&cookie))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body.contains("Testverein"));
    assert!(
        r.body
            .contains("aus der Konfiguration (CRATETAB_CLUB_NAME)")
    );

    // Ungültige Eingabe wird abgelehnt und nichts gespeichert
    let bad = "club_name=X&min_balance=5&max_balance=&max_deposit=100&deposit_presets=5+10&idle_timeout=60&undo_window=60&default_language=de";
    let r = send(&app, post("/admin/settings", bad, Some(&cookie))).await;
    assert!(r.body.contains("darf nicht positiv sein"), "{}", r.body);

    // Speichern: Vereinsname, Minimalguthaben 0, Höchstguthaben 20 €, keine Selbstregistrierung
    let good = "club_name=Hackspace+e.+V.&min_balance=0&max_balance=20&max_deposit=50&deposit_presets=2+5&idle_timeout=90&undo_window=30&default_language=de";
    let r = send(&app, post("/admin/settings", good, Some(&cookie))).await;
    assert_eq!(r.location.as_deref(), Some("/admin/settings?saved=1"));
    let r = send(&app, get("/admin/settings", Some(&cookie))).await;
    assert!(r.body.contains("im Admin-Bereich gespeichert"));

    // Wirkt sofort am Kiosk
    let r = send(&app, get("/", None)).await;
    assert!(r.body.contains("Hackspace e. V."));
    assert!(!r.body.contains("/register"), "Selbstregistrierung ist aus");
    assert_eq!(
        send(&app, get("/register", None)).await.status,
        StatusCode::NOT_FOUND
    );
    let r = send(&app, get(&format!("/u/{}", user.id), None)).await;
    assert!(
        r.body.contains("2,00 €") && r.body.contains("5,00 €"),
        "neue Schnellwahl-Beträge"
    );
    assert!(r.body.contains(r#"data-idle-timeout="90""#));
    let r = send(
        &app,
        post(&format!("/u/{}/deposit", user.id), "amount=15", None),
    )
    .await;
    assert!(r.body.contains("15,00 €"));
    let r = send(
        &app,
        post(&format!("/u/{}/deposit", user.id), "amount=10", None),
    )
    .await;
    assert!(r.body.contains("Höchstguthaben von 20,00 €"), "{}", r.body);

    // Zurücksetzen: wieder die Konfiguration
    let r = send(&app, post("/admin/settings/reset", "", Some(&cookie))).await;
    assert_eq!(r.status, StatusCode::SEE_OTHER);
    let r = send(&app, get("/", None)).await;
    assert!(r.body.contains("Testverein"));
}
