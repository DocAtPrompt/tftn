// tftn-bericht: längerer Bericht oder Abschlussarbeit mit Titelseite, Inhaltsverzeichnis,
// nummerierten Kapiteln und Literatur.

#import "@local/tftn-kern:0.2.0": datum-text, hinweis, trennungen

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (ohne-titel: "Ohne Titel", inhalt: "Inhaltsverzeichnis", literatur: "Literaturverzeichnis"),
  en: (ohne-titel: "Untitled", inhalt: "Contents", literatur: "References"),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

/// Setzt den Bericht:
/// `#show: bericht.with(daten: toml("dokument.toml"), trennliste: read("trennungen.txt"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - trennliste (str, array): Inhalt von `trennungen.txt`
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; ohne sie keine Literatur
/// - body (content): der Text aus `main.typ`
/// -> content
#let bericht(daten: (:), trennliste: (), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let autoren = daten.at("autoren", default: ())
  let literatur = daten.at("literatur", default: (:))
  let dateien = literatur.at("dateien", default: ())
  let stil = literatur.at("stil", default: "american-psychological-association")
  let titel = daten.at("titel", default: wort.ohne-titel)
  set document(title: titel, author: autoren)
  set text(font: "Libertinus Serif", size: 11.5pt, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(justify: true, leading: 0.7em, spacing: 1.1em)
  set heading(numbering: "1.1")
  set math.equation(numbering: "(1)")
  show raw: set text(font: "DejaVu Sans Mono", size: 9pt)
  show heading.where(level: 1): it => {
    pagebreak(weak: true)
    v(8mm)
    text(size: 18pt, it)
    v(4mm)
  }
  let stil = if pfad != none and lower(stil).ends-with(".csl") { pfad(stil) } else { stil }
  set bibliography(style: stil)
  show: trennungen(trennliste)

  // Titelseite ohne Seitenzahl
  page(margin: (x: 30mm, y: 35mm), numbering: none)[
    #if "institution" in daten [#tftn-feld("institution")[#text(size: 12pt)[#daten.institution]]]
    #v(1fr)
    #tftn-feld("titel")[#text(size: 26pt, weight: "bold")[#titel]]
    #if "untertitel" in daten [ \ #v(2mm) #tftn-feld("untertitel")[#text(
        size: 15pt,
      )[#daten.untertitel]]]
    #v(10mm)
    #tftn-feld("autoren")[#text(size: 13pt)[#autoren.join(linebreak())]]
    #v(1fr)
    #if "datum" in daten [#tftn-feld("datum")[#datum-text(daten.datum, sprache)]]
  ]
  set page(
    paper: "a4",
    margin: (left: 30mm, right: 25mm, y: 25mm),
    numbering: "1",
    header: context {
      if here().page() > 1 {
        set text(size: 8.5pt, fill: luma(80))
        align(right, titel)
      }
    },
  )
  counter(page).update(1)
  if daten.at("inhaltsverzeichnis", default: true) {
    outline(title: wort.inhalt, indent: auto)
    pagebreak()
  }
  body
  if pfad != none and dateien.len() > 0 {
    show heading.where(level: 1): it => {
      pagebreak(weak: true)
      v(8mm)
      text(size: 18pt, it.body)
      v(4mm)
    }
    bibliography(dateien.map(pfad), title: wort.literatur)
  }
}
