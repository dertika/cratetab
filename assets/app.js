/* Crate Tab – Browser-Logik
 *
 * Kein Framework, nur ein paar kleine Bausteine:
 *   1. Inaktivität: Kiosk-Seiten springen nach einiger Zeit zur Startseite zurück
 *   2. Suche und Buchstabenfilter auf der Startseite
 *   3. USB-Barcodescanner (verhält sich wie eine Tastatur, endet mit Enter)
 *   4. Kamera-Scanner (BarcodeDetector oder lokal ausgeliefertes zxing)
 *   5. Ziffernblock für Touchscreens
 *   6. Kleinkram: Rückgängig-Knopf ausblenden, Bestätigungsdialoge, Auto-Submit
 *
 * Wegen der Content-Security-Policy gibt es kein Inline-JavaScript im HTML;
 * Texte und Einstellungen kommen über data-Attribute am <body>.
 */
(function () {
  "use strict";

  var body = document.body;

  /** Kurze Meldung unten am Bildschirm anzeigen. */
  function toast(text) {
    var el = document.getElementById("toast");
    if (!el || !text) return;
    el.textContent = text;
    el.hidden = false;
    clearTimeout(toast.timer);
    toast.timer = setTimeout(function () { el.hidden = true; }, 4000);
  }

  /** Der erste sichtbare Ziffernblock (eingeklappte zählen nicht). */
  function visibleKeypad() {
    var pads = document.querySelectorAll(".keypad");
    for (var i = 0; i < pads.length; i++) {
      var pad = pads[i];
      if (pad.closest("details:not([open])")) continue;
      if (pad.getClientRects().length > 0) return pad;
    }
    return null;
  }

  /** Ist gerade ein Eingabefeld fokussiert? */
  function isTyping() {
    var el = document.activeElement;
    if (!el) return false;
    var tag = el.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
  }

  // -------------------------------------------------------------------------
  // 1. Inaktivität
  // -------------------------------------------------------------------------
  var idleSeconds = parseInt(body.dataset.idleTimeout || "0", 10);
  if (idleSeconds > 0) {
    var idleTimer;
    var resetIdle = function () {
      clearTimeout(idleTimer);
      idleTimer = setTimeout(function () {
        if (location.pathname === "/") {
          // Startseite: nur Suche zurücksetzen
          var search = document.getElementById("user-filter");
          if (search && search.value) { search.value = ""; applyFilter(); }
          resetIdle();
        } else {
          location.href = "/";
        }
      }, idleSeconds * 1000);
    };
    ["pointerdown", "keydown", "touchstart", "scroll", "input"].forEach(function (ev) {
      document.addEventListener(ev, resetIdle, { passive: true });
    });
    document.addEventListener("htmx:afterRequest", resetIdle);
    resetIdle();
  }

  // -------------------------------------------------------------------------
  // 2. Suche und Buchstabenfilter
  // -------------------------------------------------------------------------
  var activeLetter = "";

  function applyFilter() {
    document.querySelectorAll("[data-filter]").forEach(function (input) {
      var container = document.querySelector(input.dataset.filter);
      if (!container) return;
      var q = input.value.trim().toLowerCase();
      var visible = 0;
      container.querySelectorAll("[data-name]").forEach(function (item) {
        var matchText = !q || item.dataset.name.indexOf(q) !== -1;
        var matchLetter = !activeLetter || item.dataset.letter === activeLetter;
        var show = matchText && matchLetter;
        item.hidden = !show;
        if (show) visible++;
      });
      var empty = document.getElementById("user-filter-empty");
      if (empty) empty.hidden = visible > 0 || !container.querySelector("[data-name]");
    });
  }

  document.querySelectorAll("[data-filter]").forEach(function (input) {
    input.addEventListener("input", applyFilter);
  });

  document.querySelectorAll("[data-letter-filter] .letter").forEach(function (btn) {
    btn.addEventListener("click", function () {
      activeLetter = btn.dataset.letter;
      btn.parentElement.querySelectorAll(".letter").forEach(function (b) {
        b.classList.toggle("active", b === btn);
      });
      applyFilter();
    });
  });

  // -------------------------------------------------------------------------
  // 3. + 4. Barcodes
  // -------------------------------------------------------------------------

  /** Verarbeitet einen gescannten Code, egal ob USB-Scanner oder Kamera. */
  function handleCode(code, appendTarget) {
    code = (code || "").trim();
    if (!code) return;

    // Admin: Code in ein Textfeld anhängen (Barcodes eines Artikels)
    var target = appendTarget || document.querySelector("[data-scan-append]");
    if (target) {
      var lines = target.value.split(/\r?\n/).map(function (l) { return l.trim(); }).filter(Boolean);
      if (lines.indexOf(code) === -1) lines.push(code);
      target.value = lines.join("\n");
      return;
    }

    // Kiosk-Benutzerseite: Kauf per Barcode
    var url = body.dataset.scanUrl;
    if (url && window.htmx) {
      window.htmx.ajax("POST", url, { target: "#status", swap: "outerHTML", values: { code: code } });
      return;
    }

    // Startseite: erst Benutzer wählen
    toast(body.dataset.scanHint);
  }

  // USB-Scanner: tippt sehr schnell (wenige ms pro Zeichen) und schickt Enter.
  // Wir sammeln schnelle Tastendrücke und werten sie bei Enter aus – aber nur,
  // wenn gerade kein Eingabefeld fokussiert ist (sonst tippt dort ein Mensch).
  var scanBuffer = "";
  var lastKeyTime = 0;
  var MAX_GAP_MS = 60;      // Abstand zwischen Zeichen eines Scanners
  var MIN_LENGTH = 4;       // kürzere Eingaben sind sicher kein Barcode

  document.addEventListener("keydown", function (e) {
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    if (isTyping()) return;
    if (visibleKeypad()) return; // sichtbarer Ziffernblock bekommt die Tasten, siehe unten
    var now = Date.now();
    if (now - lastKeyTime > MAX_GAP_MS) scanBuffer = "";
    lastKeyTime = now;

    if (e.key === "Enter") {
      if (scanBuffer.length >= MIN_LENGTH) {
        e.preventDefault();
        handleCode(scanBuffer);
      }
      scanBuffer = "";
    } else if (e.key.length === 1) {
      scanBuffer += e.key;
    }
  });

  // Kamera-Scanner
  var ZXING_URL = "/static/vendor/zxing-browser.min.js";
  var zxingLoading = null;

  function loadZxing() {
    if (window.ZXingBrowser) return Promise.resolve(window.ZXingBrowser);
    if (!zxingLoading) {
      zxingLoading = new Promise(function (resolve, reject) {
        var s = document.createElement("script");
        s.src = ZXING_URL;
        s.onload = function () { resolve(window.ZXingBrowser); };
        s.onerror = reject;
        document.head.appendChild(s);
      });
    }
    return zxingLoading;
  }

  /** Prüft, ob der native BarcodeDetector EAN-Codes kann (Chrome/Android). */
  function nativeDetector() {
    if (!("BarcodeDetector" in window)) return Promise.resolve(null);
    return window.BarcodeDetector.getSupportedFormats().then(function (formats) {
      var wanted = ["ean_13", "ean_8", "upc_a", "upc_e", "code_128", "code_39"].filter(function (f) {
        return formats.indexOf(f) !== -1;
      });
      if (wanted.indexOf("ean_13") === -1) return null;
      return new window.BarcodeDetector({ formats: wanted });
    }).catch(function () { return null; });
  }

  function openCamera(appendTarget) {
    if (!window.isSecureContext || !navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
      toast(body.dataset.cameraInsecure);
      return;
    }

    var overlay = document.createElement("div");
    overlay.className = "camera-overlay";
    var title = document.createElement("p");
    title.className = "camera-msg";
    title.textContent = body.dataset.cameraTitle;
    var frame = document.createElement("div");
    frame.className = "camera-frame";
    var video = document.createElement("video");
    video.setAttribute("playsinline", "");
    video.muted = true;
    frame.appendChild(video);
    var close = document.createElement("button");
    close.type = "button";
    close.className = "btn btn-secondary";
    close.textContent = body.dataset.cameraClose;
    overlay.appendChild(title);
    overlay.appendChild(frame);
    overlay.appendChild(close);
    document.body.appendChild(overlay);

    var stream = null;
    var zxingControls = null;
    var stopped = false;

    function stop() {
      if (stopped) return;
      stopped = true;
      if (zxingControls) zxingControls.stop();
      if (stream) stream.getTracks().forEach(function (t) { t.stop(); });
      overlay.remove();
    }
    function found(code) {
      if (stopped) return;
      if (navigator.vibrate) navigator.vibrate(80);
      stop();
      handleCode(code, appendTarget);
    }
    function fail() {
      stop();
      toast(body.dataset.cameraUnavailable);
    }
    close.addEventListener("click", stop);

    var constraints = { video: { facingMode: { ideal: "environment" } }, audio: false };

    nativeDetector().then(function (detector) {
      if (detector) {
        // Variante 1: eingebauter BarcodeDetector
        return navigator.mediaDevices.getUserMedia(constraints).then(function (s) {
          stream = s;
          video.srcObject = s;
          return video.play();
        }).then(function () {
          var tick = function () {
            if (stopped) return;
            detector.detect(video).then(function (codes) {
              if (codes.length) found(codes[0].rawValue);
              else setTimeout(tick, 120);
            }).catch(function () { setTimeout(tick, 250); });
          };
          tick();
        });
      }
      // Variante 2: zxing (lokal ausgeliefert, nur bei Bedarf geladen)
      return loadZxing().then(function (ZX) {
        var reader = new ZX.BrowserMultiFormatOneDReader();
        return reader.decodeFromConstraints(constraints, video, function (result) {
          if (result) found(result.getText());
        }).then(function (controls) {
          zxingControls = controls;
          if (stopped) controls.stop();
        });
      });
    }).catch(fail);
  }

  document.addEventListener("click", function (e) {
    var btn = e.target.closest("[data-camera-scan]");
    if (!btn) return;
    e.preventDefault();
    var target = btn.dataset.scanTarget ? document.querySelector(btn.dataset.scanTarget) : null;
    openCamera(target);
  });

  // -------------------------------------------------------------------------
  // 5. Ziffernblock
  // -------------------------------------------------------------------------
  function keypadInput(key, input) {
    if (!input) return;
    var max = parseInt(input.getAttribute("maxlength") || "12", 10);
    if (key === "back") input.value = input.value.slice(0, -1);
    else if (key === "clear") input.value = "";
    else if (input.value.length < max) input.value += key;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }

  document.addEventListener("click", function (e) {
    var key = e.target.closest(".keypad .key");
    if (!key) return;
    var pad = key.closest(".keypad");
    keypadInput(key.dataset.key, document.getElementById(pad.dataset.keypadFor));
  });

  // Ist ein Ziffernblock sichtbar, gehen Tastendrücke ins zugehörige Feld,
  // auch wenn es nicht fokussiert ist (z. B. Kiosk mit USB-Ziffernblock).
  document.addEventListener("keydown", function (e) {
    if (isTyping() || e.ctrlKey || e.altKey || e.metaKey) return;
    var pad = visibleKeypad();
    if (!pad) return;
    var input = document.getElementById(pad.dataset.keypadFor);
    if (!input) return;
    if (/^[0-9]$/.test(e.key)) { keypadInput(e.key, input); e.preventDefault(); }
    else if (e.key === "Backspace") { keypadInput("back", input); e.preventDefault(); }
    else if (e.key === "Enter" && input.form) { input.form.requestSubmit(); e.preventDefault(); }
  });

  // -------------------------------------------------------------------------
  // 6. Kleinkram
  // -------------------------------------------------------------------------

  // „Rückgängig“-Knopf nach Ablauf der Frist ausblenden
  function armExpiry(root) {
    (root || document).querySelectorAll("[data-expires-in]").forEach(function (el) {
      var secs = parseInt(el.dataset.expiresIn, 10);
      setTimeout(function () { el.remove(); }, secs * 1000);
    });
  }
  armExpiry();
  document.addEventListener("htmx:afterSwap", function (e) {
    armExpiry(e.detail.target && e.detail.target.parentElement);
  });

  // Freien Betrag nach erfolgreicher Aufladung leeren
  document.addEventListener("htmx:afterRequest", function (e) {
    var form = e.detail.elt;
    if (form && form.matches && form.matches("form[data-reset-on-success]") && e.detail.successful) {
      form.reset();
      var details = form.closest("details");
      if (details) details.open = false;
    }
  });

  // Bestätigung vor kritischen Formularen: <form data-confirm="Wirklich?">
  document.addEventListener("submit", function (e) {
    var form = e.target;
    if (form.dataset && form.dataset.confirm && !window.confirm(form.dataset.confirm)) {
      e.preventDefault();
    }
  }, true);

  // Auswahlfelder, die sofort absenden (z. B. Jahr in der Statistik)
  document.querySelectorAll("select[data-autosubmit]").forEach(function (sel) {
    sel.addEventListener("change", function () { sel.form.submit(); });
  });

  // Netzwerkfehler bei htmx-Anfragen sichtbar machen
  document.addEventListener("htmx:sendError", function () {
    toast(body.dataset.networkError);
  });
})();
