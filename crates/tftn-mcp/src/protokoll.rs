//! JSON-RPC 2.0 über stdin/stdout, eine Nachricht je Zeile (MCP-Transport „stdio“).

use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

use crate::Ergebnis;
use crate::werkzeuge::werkzeuge;

/// Was der Assistent beim Verbinden über TFTN erfährt.
const ANLEITUNG: &str = "TFTN ist ein Typst-Editor. Die Werkzeuge arbeiten im Dokument, das gerade in \
der laufenden TFTN-App offen ist; jede Änderung erscheint sofort im Editor und in der Vorschau und \
lässt sich dort mit Rückgängig zurücknehmen. Zuerst `status` und `quelltext_lesen` aufrufen, dann mit \
`text_ersetzen` gezielt ändern. Titel, Autorinnen und Autoren, Datum und ähnliche Angaben stehen bei \
den meisten Dokumenttypen im Formular (Fach „Dokument“), nicht im Quelltext: `formular_lesen` und \
`formular_setzen`. Bindet die Hauptdatei weitere Dateien ein (Kapitel mit `#include`), zeigt \
`gliederung_lesen` den Aufbau; diese Dateien gehen mit `datei_lesen`, `datei_ersetzen` und \
`datei_schreiben`. Was der Benutzer markiert hat, liefert `auswahl_lesen`. Literatur: `literatur_lesen` \
und `literatur_eintrag_speichern`. Vor größeren Umbauten `fassung_sichern`. Der Benutzer schaltet MCP \
in TFTN selbst ein und aus.";

/// Liest Anfragen aus `ein` und schreibt Antworten nach `aus`, bis `ein` endet.
/// `aufrufen(name, argumente)` führt ein Werkzeug aus (in der App: über die lokale Verbindung).
pub fn bruecke(
    ein: impl BufRead,
    mut aus: impl Write,
    mut aufrufen: impl FnMut(&str, Value) -> Ergebnis,
) -> io::Result<()> {
    for zeile in ein.lines() {
        let zeile = zeile?;
        if zeile.trim().is_empty() {
            continue;
        }
        if let Some(antwort) = beantworten(&zeile, &mut aufrufen) {
            writeln!(aus, "{antwort}")?;
            aus.flush()?;
        }
    }
    Ok(())
}

/// Eine Nachricht; `None` für Benachrichtigungen (ohne `id`) und Antworten des Gegenübers.
fn beantworten(zeile: &str, aufrufen: &mut impl FnMut(&str, Value) -> Ergebnis) -> Option<Value> {
    let nachricht: Value = match serde_json::from_str(zeile) {
        Ok(n) => n,
        Err(e) => return Some(fehler(Value::Null, -32700, &format!("kein JSON: {e}"))),
    };
    let id = nachricht.get("id").cloned()?;
    let Some(methode) = nachricht.get("method").and_then(Value::as_str) else {
        return None; // Antwort auf eine Anfrage, die wir nie stellen
    };
    let parameter = nachricht.get("params").cloned().unwrap_or(Value::Null);
    let ergebnis = match methode {
        "initialize" => json!({
            // Nur Werkzeuge: Das ist in allen bisherigen Protokollversionen gleich, daher die des Gegenübers.
            "protocolVersion": parameter
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-06-18"),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "tftn", "version": env!("CARGO_PKG_VERSION") },
            "instructions": ANLEITUNG,
        }),
        "ping" => json!({}),
        "tools/list" => json!({ "tools": werkzeuge() }),
        "tools/call" => {
            let Some(name) = parameter.get("name").and_then(Value::as_str) else {
                return Some(fehler(id, -32602, "Werkzeugname fehlt"));
            };
            if !werkzeuge().iter().any(|w| w["name"] == name) {
                return Some(fehler(id, -32602, &format!("unbekanntes Werkzeug: {name}")));
            }
            let argumente = parameter.get("arguments").cloned().unwrap_or(json!({}));
            serde_json::to_value(aufrufen(name, argumente)).unwrap_or(Value::Null)
        }
        _ => {
            return Some(fehler(
                id,
                -32601,
                &format!("unbekannte Methode: {methode}"),
            ));
        }
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": ergebnis }))
}

fn fehler(id: Value, code: i64, text: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": text } })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gespraech(eingabe: &str) -> Vec<Value> {
        let mut aus = Vec::new();
        let mut gerufen = Vec::new();
        bruecke(eingabe.as_bytes(), &mut aus, |name, args| {
            gerufen.push(name.to_string());
            Ergebnis::text(format!("{name} {args}"))
        })
        .unwrap();
        String::from_utf8(aus)
            .unwrap()
            .lines()
            .map(|z| serde_json::from_str(z).unwrap())
            .collect()
    }

    #[test]
    fn handschlag_werkzeugliste_und_aufruf() {
        let a = gespraech(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":"x","method":"tools/call","params":{"name":"status","arguments":{}}}"#,
            "\n",
        ));
        assert_eq!(a.len(), 3, "keine Antwort auf die Benachrichtigung");
        assert_eq!(a[0]["id"], 1);
        assert_eq!(a[0]["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(a[0]["result"]["serverInfo"]["name"], "tftn");
        assert!(a[0]["result"]["capabilities"]["tools"].is_object());
        let namen: Vec<&str> = a[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w["name"].as_str().unwrap())
            .collect();
        assert!(namen.contains(&"text_ersetzen"));
        assert_eq!(a[2]["id"], "x");
        assert_eq!(a[2]["result"]["content"][0]["type"], "text");
        assert_eq!(a[2]["result"]["content"][0]["text"], "status {}");
        assert_eq!(a[2]["result"]["isError"], false);
    }

    #[test]
    fn fehler_nach_json_rpc() {
        let a = gespraech(concat!(
            "kaputt\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"loeschen"}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":5,"method":"ping"}"#,
            "\n",
        ));
        assert_eq!(a[0]["error"]["code"], -32700);
        assert_eq!(a[0]["id"], Value::Null);
        assert_eq!(a[1]["error"]["code"], -32601);
        assert_eq!(a[2]["error"]["code"], -32602);
        assert_eq!(a[3]["result"], json!({}));
    }
}
