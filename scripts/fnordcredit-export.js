// Exportiert die Daten aus fnordcredit (RethinkDB) für den Import in Crate Tab –
// ohne Python. Nutzt den RethinkDB-Treiber und die config.js der vorhandenen
// fnordcredit-Installation und schreibt dieselben Dateien wie „rethinkdb export“.
//
// Aufruf im Verzeichnis der fnordcredit-Installation (dort, wo config.js liegt):
//
//   cd /pfad/zu/fnordcredit
//   node /root/fnordcredit-export.js /root/fnordcredit-export
//
// Ergebnis: /root/fnordcredit-export/<datenbank>/users.json, transactions.json, products.json
//
// Bewusst in altem JavaScript (ES5) geschrieben, damit es auch mit der alten
// Node.js-Version des fnordcredit-Servers läuft.
'use strict';

var fs = require('fs');
var path = require('path');

var base = process.cwd();
var r, config;
try {
    r = require(path.join(base, 'node_modules', 'rethinkdb'));
    config = require(path.join(base, 'config.js'));
} catch (e) {
    console.error('Bitte im Verzeichnis der fnordcredit-Installation aufrufen ' +
        '(dort, wo config.js und node_modules/rethinkdb liegen).\n' + e.message);
    process.exit(1);
}

var settings = config.rethinkdb || {};
var dbName = settings.db || 'fnordcredit';
var outDir = path.resolve(process.argv[2] || 'fnordcredit-export');
var tableDir = path.join(outDir, dbName);

function mkdir(dir) {
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir);
    }
}

function fail(what, err) {
    console.error('Fehler bei ' + what + ': ' + (err && err.message ? err.message : err));
    process.exit(1);
}

mkdir(outDir);
mkdir(tableDir);

r.connect({
    host: settings.host || 'localhost',
    port: settings.port || 28015,
    db: dbName,
    authKey: settings.authKey || ''
}, function (err, conn) {
    if (err) { return fail('der Verbindung zu RethinkDB', err); }

    r.db(dbName).tableList().run(conn, function (err, tables) {
        if (err) { return fail('der Tabellenliste', err); }
        var wanted = ['users', 'transactions', 'products'].filter(function (t) {
            return tables.indexOf(t) !== -1;
        });
        if (wanted.indexOf('users') === -1 || wanted.indexOf('transactions') === -1) {
            return fail('der Datenbank „' + dbName + '“',
                'Tabellen users/transactions nicht gefunden (vorhanden: ' + tables.join(', ') + ')');
        }

        var i = 0;
        (function next() {
            if (i >= wanted.length) {
                conn.close();
                console.log('Fertig: ' + tableDir);
                return;
            }
            var table = wanted[i++];
            // timeFormat "raw": Zeitstempel so wie „rethinkdb export“ ($reql_type$: TIME)
            r.db(dbName).table(table).run(conn, { timeFormat: 'raw' }, function (err, cursor) {
                if (err) { return fail('Tabelle ' + table, err); }
                cursor.toArray(function (err, rows) {
                    if (err) { return fail('Tabelle ' + table, err); }
                    var file = path.join(tableDir, table + '.json');
                    fs.writeFileSync(file, JSON.stringify(rows, null, 1));
                    console.log(table + ': ' + rows.length + ' Einträge → ' + file);
                    next();
                });
            });
        })();
    });
});
