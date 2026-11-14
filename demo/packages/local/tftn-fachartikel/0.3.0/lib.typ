// tftn-fachartikel: Satz eines Fachartikels aus main.typ (Text) und dokument.toml (Formular).

#import "@local/tftn-kern:0.2.0": datum-text, hinweis, trennungen

/// Beschriftungen des Satzes je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (
    ohne-titel: "Ohne Titel",
    abstract: "Zusammenfassung",
    schlagworte: "Schlagwörter",
    literatur: "Literatur",
  ),
  en: (
    ohne-titel: "Untitled",
    abstract: "Abstract",
    schlagworte: "Keywords",
    literatur: "References",
  ),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau auf `inhalt` führt zum Formularfeld
/// `name` (PLAN-M2 AP7). Ändert am Satz nichts.
///
/// - name (str): Name des Felds in dokument.toml
/// - inhalt (content): der gesetzte Wert
/// -> content
#let tftn-feld(name, inhalt) = inhalt

/// Setzt den ganzen Artikel:
/// `#show: fachartikel.with(daten: toml("dokument.toml"), trennliste: read("trennungen.txt"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`); `sprache`
///   (`"de"` oder `"en"`, Vorgabe `"de"`) bestimmt Satzsprache und Beschriftungen
/// - trennliste (str, array): Inhalt von `trennungen.txt`
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; ohne sie kein Literaturverzeichnis
/// - body (content): der Text aus `main.typ`
/// -> content
#let fachartikel(
  daten: (:),
  trennliste: (),
  pfad: none,
  body,
) = {
  let autoren = daten.at("autoren", default: ())
  let literatur = daten.at("literatur", default: (:))
  let dateien = literatur.at("dateien", default: ())
  let stil = literatur.at("stil", default: "american-psychological-association")
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  set document(
    title: daten.at("titel", default: none),
    author: autoren.map(a => a.name),
    keywords: daten.at("schlagworte", default: ()),
  )
  set page(paper: "a4", margin: (x: 25mm, y: 25mm), numbering: "1")
  set text(font: "Libertinus Serif", size: 11pt, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(justify: true, leading: 0.65em)
  set heading(numbering: "1.1")
  set math.equation(numbering: "(1)")
  show raw: set text(font: "DejaVu Sans Mono", size: 9pt)
  // Ein eigener Stil liegt als .csl-Datei im Projekt, sonst ist es ein eingebauter Name.
  // (Eine set-Regel in einem if-Block gälte nur dort, darum vorher ausrechnen.)
  let stil = if pfad != none and lower(stil).ends-with(".csl") { pfad(stil) } else { stil }
  set bibliography(style: stil)
  show: trennungen(trennliste)

  // Titelei (Text aus dokument.toml); tftn-feld führt einen Klick zum Formularfeld. Werte stehen
  // als `[#wert]` im Markup: Nur so tragen sie eine Stelle in dieser Datei, und der Klick findet
  // die Marke (Text aus einem Code-Argument hätte keine).
  align(center)[
    #tftn-feld("titel", text(size: 17pt, weight: "bold")[#daten.at(
      "titel",
      default: wort.ohne-titel,
    )])
    #if "untertitel" in daten [ \ #tftn-feld("untertitel", text(size: 13pt)[#daten.untertitel]) ]
    #v(0.6em)
    #tftn-feld("autoren")[#autoren.map(a => a.name).join(" · ")]
    #let orte = autoren.map(a => a.at("institution", default: ())).flatten().dedup()
    #if orte.len() > 0 [ \ #tftn-feld("autoren", text(size: 9pt)[#orte.join(" · ")]) ]
    #if "datum" in daten [ \ #tftn-feld("datum", text(size: 9pt)[#datum-text(
      daten.datum,
      sprache,
    )]) ]
  ]
  v(1em)
  if "abstract" in daten or "schlagworte" in daten {
    block(inset: (x: 1.5em), text(size: 9.5pt)[
      #if "abstract" in daten {
        tftn-feld("abstract")[*#wort.abstract.* #eval(daten.abstract, mode: "markup")]
      }
      #if "schlagworte" in daten [

        #tftn-feld("schlagworte")[*#wort.schlagworte:* #daten.schlagworte.join(" · ")]
      ]
    ])
  }
  body
  if pfad != none and dateien.len() > 0 {
    bibliography(dateien.map(pfad), title: wort.literatur)
  }
}
