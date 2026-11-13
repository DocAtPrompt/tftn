// tftn-plakat: wissenschaftliches Plakat (Poster) im Hochformat A0 bis A2, Kopfband mit Titel,
// Text in Spalten.

#import "@local/tftn-kern:0.2.0": hinweis

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (ohne-titel: "Ohne Titel", kontakt: "Kontakt", literatur: "Literatur"),
  en: (ohne-titel: "Untitled", kontakt: "Contact", literatur: "References"),
)

/// Grundgröße der Schrift je Format; alles andere skaliert damit.
#let _groesse = (a0: 26pt, a1: 19pt, a2: 14pt)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

/// Kasten mit Überschrift, z. B. für das Wichtigste auf einen Blick.
///
/// - titel (str, content): Überschrift des Kastens
/// - body (content): Inhalt
/// -> content
#let kasten(titel: [], body) = block(
  width: 100%,
  inset: 0.8em,
  radius: 0.3em,
  stroke: 0.08em + luma(160),
  breakable: false,
)[*#titel* #body]

/// Setzt das Plakat: `#show: plakat.with(daten: toml("dokument.toml"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; ohne sie keine Literatur
/// - body (content): der Text aus `main.typ`, in Abschnitte mit `=` gegliedert
/// -> content
#let plakat(daten: (:), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let format = daten.at("format", default: "a0")
  let gross = _groesse.at(format, default: 26pt)
  let farbe = rgb(daten.at("farbe", default: "#1f4e79"))
  let autoren = daten.at("autoren", default: ())
  let titel = daten.at("titel", default: wort.ohne-titel)
  let literatur = daten.at("literatur", default: (:))
  let dateien = literatur.at("dateien", default: ())
  let stil = literatur.at("stil", default: "american-psychological-association")
  set document(title: titel, author: autoren)
  set page(paper: format, margin: (x: 2 * gross, top: 0pt, bottom: 2 * gross))
  set text(font: "Source Sans 3", size: gross, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(justify: true, leading: 0.55em, spacing: 0.9em)
  show heading: it => block(
    width: 100%,
    fill: farbe,
    inset: (x: 0.5em, y: 0.35em),
    above: 1.2em,
    below: 0.6em,
    text(fill: white, weight: "bold", size: 1.15em, it.body),
  )
  let stil = if pfad != none and lower(stil).ends-with(".csl") { pfad(stil) } else { stil }
  set bibliography(style: stil)

  // Kopfband über die ganze Breite: aus dem Rand hinaus bis an die Papierkanten
  pad(x: -2 * gross, block(width: 100%, inset: (x: 2 * gross, y: 1.6 * gross), fill: farbe)[
    #set text(fill: white)
    #set par(justify: false)
    #tftn-feld("titel")[#text(size: 3.2 * gross, weight: "bold")[#titel]]
    #if "untertitel" in daten [ \ #tftn-feld("untertitel")[#text(
      size: 1.6 * gross,
    )[#daten.untertitel]]]
    #v(0.6em)
    #tftn-feld("autoren")[#text(size: 1.3 * gross)[#autoren.join(", ")]]
    #if "institution" in daten [ \ #tftn-feld("institution")[#daten.institution]]
  ])
  v(1.5 * gross)
  columns(daten.at("spalten", default: 3), gutter: 2 * gross, {
    body
    if pfad != none and dateien.len() > 0 {
      set text(size: 0.75em)
      bibliography(dateien.map(pfad), title: wort.literatur)
    }
  })
  if "kontakt" in daten {
    place(bottom + left, dy: 1.2 * gross, text(size: 0.8em, fill: luma(70))[
      #wort.kontakt: #tftn-feld("kontakt")[#daten.kontakt]
    ])
  }
}
