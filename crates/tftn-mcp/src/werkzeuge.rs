//! Die Werkzeuge, die TFTN über MCP anbietet (Name, Beschreibung, JSON-Schema der Argumente).

use serde_json::{Value, json};

pub fn werkzeuge() -> Vec<Value> {
    let leer = json!({ "type": "object", "properties": {} });
    vec![
        json!({
            "name": "status",
            "description": "Zustand des offenen Dokuments: Projektordner, Hauptdatei, Revision, ob \
                ungespeichert, Seitenzahl und die Meldungen (Fehler, Warnungen) des letzten Typst-Laufs.",
            "inputSchema": leer,
        }),
        json!({
            "name": "quelltext_lesen",
            "description": "Typst-Quelltext der Hauptdatei, so wie er gerade im Editor steht \
                (auch ungespeicherte Änderungen), mit Zeilennummern.",
            "inputSchema": leer,
        }),
        json!({
            "name": "text_ersetzen",
            "description": "Ersetzt im Quelltext genau die Stelle `alt` durch `neu`; `alt` muss \
                wörtlich und eindeutig vorkommen (sonst mehr Kontext angeben oder `alle: true`). \
                Die Änderung erscheint sofort im Editor (ein Rückgängig-Schritt). Antwort: Ergebnis \
                des Typst-Laufs danach.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "alt": { "type": "string", "description": "vorhandener Text, wörtlich" },
                    "neu": { "type": "string", "description": "Ersatz (leer: löschen)" },
                    "alle": { "type": "boolean", "description": "jedes Vorkommen ersetzen", "default": false },
                },
                "required": ["alt", "neu"],
            },
        }),
        json!({
            "name": "quelltext_setzen",
            "description": "Ersetzt den ganzen Quelltext der Hauptdatei (ein Rückgängig-Schritt). \
                Für kleine Änderungen besser `text_ersetzen`.",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"],
            },
        }),
        json!({
            "name": "seite_ansehen",
            "description": "Bild (PNG) einer Seite der Vorschau, um Satz und Layout zu prüfen.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "seite": { "type": "integer", "minimum": 1, "description": "Seitennummer ab 1" },
                    "ppi": { "type": "number", "minimum": 30, "maximum": 300, "default": 100 },
                },
                "required": ["seite"],
            },
        }),
        json!({
            "name": "auswahl_lesen",
            "description": "Wo im Editor die Schreibmarke steht (Zeile, Spalte der Hauptdatei) und was \
                der Benutzer gerade markiert hat, mit Zeilennummern. Für Bitten wie „formuliere das \
                um“ oder „was hältst du von diesem Absatz“.",
            "inputSchema": leer,
        }),
        json!({
            "name": "gliederung_lesen",
            "description": "Überschriften (mit Ebene, Zeile und Wörtern) und Kommentare (`//`, \
                `/* */`, mit Zeile) der Hauptdatei und der Dateien, die sie mit `#include` \
                einbindet (etwa die Kapitel eines Buchs).",
            "inputSchema": leer,
        }),
        json!({
            "name": "dateien_auflisten",
            "description": "Dateien im Projektordner (ohne versteckte, `_`-Ordner und `packages/`), \
                mit Größe und ob lesbar bzw. schreibbar. Kapitel, Literatur (.bib), \
                Trennausnahmen (trennungen.txt), Bilder.",
            "inputSchema": leer,
        }),
        json!({
            "name": "datei_lesen",
            "description": "Text einer Datei im Projektordner, mit Zeilennummern. Für die \
                Hauptdatei kommt der Stand im Editor (wie `quelltext_lesen`).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pfad": { "type": "string", "description": "relativ zum Projektordner, z. B. kapitel/01-einleitung.typ" },
                },
                "required": ["pfad"],
            },
        }),
        json!({
            "name": "datei_schreiben",
            "description": "Schreibt eine Textdatei im Projektordner ganz (legt sie samt Ordner an, \
                wenn es sie nicht gibt). Nicht für die Hauptdatei (`text_ersetzen`) und die \
                Formulardaten (`formular_setzen`). Den alten Inhalt sichert TFTN unter \
                .fassungen/dateien/. Neue Kapitel muss die Hauptdatei noch mit `#include` einbinden.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pfad": { "type": "string", "description": "relativ zum Projektordner" },
                    "text": { "type": "string" },
                },
                "required": ["pfad", "text"],
            },
        }),
        json!({
            "name": "datei_ersetzen",
            "description": "Ersetzt in einer Textdatei des Projekts genau die Stelle `alt` durch \
                `neu` (wie `text_ersetzen`, aber für andere Dateien als die Hauptdatei). Alter \
                Inhalt gesichert unter .fassungen/dateien/.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pfad": { "type": "string", "description": "relativ zum Projektordner" },
                    "alt": { "type": "string", "description": "vorhandener Text, wörtlich" },
                    "neu": { "type": "string", "description": "Ersatz (leer: löschen)" },
                    "alle": { "type": "boolean", "description": "jedes Vorkommen ersetzen", "default": false },
                },
                "required": ["pfad", "alt", "neu"],
            },
        }),
        json!({
            "name": "literatur_lesen",
            "description": "Einträge der Literaturdateien des Dokuments (Schlüssel, Art, Felder) und \
                ob sie im Text zitiert sind. Zitiert wird im Text mit `@schluessel` oder \
                `@schluessel[S. 12]`.",
            "inputSchema": leer,
        }),
        json!({
            "name": "literatur_eintrag_speichern",
            "description": "Legt einen Eintrag in einer .bib-Datei des Dokuments an oder ändert ihn \
                (mit `alt`: sein bisheriger Schlüssel). Geschrieben wird nur dieser Eintrag; der Rest \
                der Datei bleibt, die alte Fassung sichert TFTN unter .fassungen/dateien/. Felder in \
                BibTeX-Namen (author, title, year oder date, journal, publisher, doi …), Personen mit \
                „ and “ getrennt („Nachname, Vorname and …“). Löschen geht nicht über MCP.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "datei": { "type": "string", "description": "Literaturdatei (Standard: die einzige .bib des Dokuments)" },
                    "alt": { "type": "string", "description": "bisheriger Schlüssel, wenn ein vorhandener Eintrag geändert wird" },
                    "art": { "type": "string", "description": "article, book, incollection, inproceedings, online, misc …" },
                    "schluessel": { "type": "string", "description": "z. B. mueller2026" },
                    "felder": { "type": "object", "description": "Feldname → Wert; beim Ändern alle Felder, die bleiben sollen" },
                },
                "required": ["art", "schluessel", "felder"],
            },
        }),
        json!({
            "name": "fassung_sichern",
            "description": "Hält den jetzigen Stand der Hauptdatei als Fassung fest (vor größeren \
                Umbauten); der Benutzer holt Fassungen in TFTN über „Fassungen“ zurück.",
            "inputSchema": leer,
        }),
        json!({
            "name": "rechtschreibung_pruefen",
            "description": "Unbekannte Wörter im Fließtext nach der Sprache des Dokuments, mit Zeile \
                und Vorschlägen (wie die rote Unterwellung in TFTN). Fachwörter und Namen können \
                richtig sein, obwohl das Wörterbuch sie nicht kennt.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pfad": { "type": "string", "description": "andere .typ-Datei des Projekts (Standard: Hauptdatei)" },
                },
            },
        }),
        json!({
            "name": "formular_lesen",
            "description": "Formular des Dokumenttyps (Fach „Dokument“): Felder mit Name, \
                Beschriftung, Art und Pflicht, dazu die aktuellen Werte aus der Datendatei \
                (meist dokument.toml) und die Meldungen der Prüfung. Titel, Autorinnen und Autoren, \
                Datum und Ähnliches stehen hier, nicht im Quelltext.",
            "inputSchema": leer,
        }),
        json!({
            "name": "formular_setzen",
            "description": "Setzt Werte im Formular, geprüft wie im Fach „Dokument“; alles in einem \
                Schritt (ein Rückgängig-Schritt), bei einem ungültigen Wert wird nichts geschrieben. \
                Nur die genannten Felder ändern sich. Gruppen als Objekt (nur die genannten \
                Unterfelder), Listen von Einträgen als Array von Objekten (ersetzt die ganze Liste), \
                Datum als JJJJ-MM-TT, `null` entfernt einen Wert. Feldnamen aus `formular_lesen`.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "werte": {
                        "type": "object",
                        "description": "Feldname → Wert, z. B. {\"titel\": \"…\", \"autoren\": [\"…\"]}",
                    },
                },
                "required": ["werte"],
            },
        }),
        json!({
            "name": "typen_auflisten",
            "description": "Dokumenttypen (Vorlagen), aus denen `dokument_anlegen` ein neues Dokument macht.",
            "inputSchema": leer,
        }),
        json!({
            "name": "dokument_anlegen",
            "description": "Legt aus einem Dokumenttyp ein neues Projekt an und öffnet es in TFTN. \
                Je nach Einstellung muss der Benutzer danach bestätigen, dass MCP aktiv bleibt.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "typ": { "type": "string", "description": "Name aus `typen_auflisten`, z. B. tftn-brief" },
                    "name": { "type": "string", "description": "Name des neuen Projektordners" },
                    "ordner": { "type": "string", "description": "Elternordner (Standard: Dokumente)" },
                },
                "required": ["typ", "name"],
            },
        }),
        json!({
            "name": "dokument_oeffnen",
            "description": "Öffnet ein anderes TFTN-Projekt (Ordner mit tftn.toml). Das offene wird \
                vorher gespeichert. Je nach Einstellung muss der Benutzer danach bestätigen, dass MCP \
                aktiv bleibt.",
            "inputSchema": {
                "type": "object",
                "properties": { "ordner": { "type": "string" } },
                "required": ["ordner"],
            },
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jedes_werkzeug_hat_name_beschreibung_und_objektschema() {
        let w = werkzeuge();
        let mut namen: Vec<&str> = w.iter().map(|w| w["name"].as_str().unwrap()).collect();
        for x in &w {
            assert!(!x["description"].as_str().unwrap().is_empty());
            assert_eq!(x["inputSchema"]["type"], "object");
            for p in x["inputSchema"]["required"]
                .as_array()
                .into_iter()
                .flatten()
            {
                assert!(x["inputSchema"]["properties"][p.as_str().unwrap()].is_object());
            }
        }
        namen.sort();
        namen.dedup();
        assert_eq!(namen.len(), w.len(), "Namen eindeutig");
    }
}
