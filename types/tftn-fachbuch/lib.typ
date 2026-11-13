// tftn-fachbuch: Buch mit Titelei (Titel, Impressum, Widmung, Inhalt), Kapiteln auf rechten
// Seiten, lebenden Kolumnentiteln und Literatur. Kapitel stehen je in einer eigenen Datei.

#import "@local/tftn-kern:0.2.0": datum-text, hinweis, trennungen

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (
    ohne-titel: "Ohne Titel",
    inhalt: "Inhaltsverzeichnis",
    literatur: "Literaturverzeichnis",
    kapitel: "Kapitel",
    auflage: "Auflage",
  ),
  en: (
    ohne-titel: "Untitled",
    inhalt: "Contents",
    literatur: "References",
    kapitel: "Chapter",
    auflage: "edition",
  ),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

// Beginnt auf dieser Seite ein Kapitel?
#let _kapitelseite() = query(heading.where(level: 1)).any(h => h.location().page() == here().page())

// Vor jedem Sprung auf die nächste rechte Seite steht eine Marke; so erkennt `_leerseite` die
// eingeschobenen Leerseiten (ohne Kopf und Fuß).
#let _sprung() = [#metadata(none) <tftn-sprung>]
#let _leerseite() = {
  let p = here().page()
  // Endet der Text auf p − 1 vor einem Sprung und beginnt auf p + 1 ein Kapitel, ist p leer.
  (
    query(<tftn-sprung>).any(m => m.location().page() == p - 1)
      and query(heading.where(level: 1)).any(h => h.location().page() == p + 1)
  )
}

// Titel des Kapitels, in dem diese Seite liegt.
#let _kapitel() = {
  let davor = query(selector(heading.where(level: 1)).before(here()))
  if davor.len() > 0 { davor.last().body } else { none }
}

/// Setzt das Buch:
/// `#show: fachbuch.with(daten: toml("dokument.toml"), trennliste: read("trennungen.txt"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - trennliste (str, array): Inhalt von `trennungen.txt`
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; ohne sie keine Literatur
/// - body (content): die Kapitel, meist als `#include "kapitel/…typ"`
/// -> content
#let fachbuch(daten: (:), trennliste: (), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let autoren = daten.at("autoren", default: ())
  let literatur = daten.at("literatur", default: (:))
  let dateien = literatur.at("dateien", default: ())
  let stil = literatur.at("stil", default: "american-psychological-association")
  let titel = daten.at("titel", default: wort.ohne-titel)
  set document(title: titel, author: autoren)
  set page(
    width: 170mm,
    height: 240mm,
    margin: (inside: 22mm, outside: 18mm, top: 20mm, bottom: 22mm),
    numbering: none,
  )
  set text(font: "Libertinus Serif", size: 10.5pt, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(justify: true, leading: 0.68em, spacing: 0.68em, first-line-indent: 1.2em)
  set heading(numbering: "1.1")
  set math.equation(numbering: "(1.1)")
  show raw: set text(font: "DejaVu Sans Mono", size: 8.5pt)
  show heading: set block(above: 1.4em, below: 0.8em)
  show heading.where(level: 1): it => {
    _sprung()
    pagebreak(to: "odd", weak: true)
    set par(first-line-indent: 0pt)
    v(30mm)
    if it.numbering != none {
      text(size: 11pt, upper(wort.kapitel + " " + counter(heading).display("1")))
      v(2mm)
    }
    text(size: 22pt, weight: "bold", it.body)
    v(12mm)
  }
  let stil = if pfad != none and lower(stil).ends-with(".csl") { pfad(stil) } else { stil }
  set bibliography(style: stil)
  show: trennungen(trennliste)

  // Titelseite
  {
    set par(first-line-indent: 0pt, justify: false)
    align(center)[
      #v(35mm)
      #tftn-feld("autoren")[#text(size: 13pt)[#autoren.join(", ")]]
      #v(12mm)
      #tftn-feld("titel")[#text(size: 24pt, weight: "bold")[#titel]]
      #if "untertitel" in daten [ \ #v(3mm) #tftn-feld("untertitel")[#text(
          size: 14pt,
        )[#daten.untertitel]]]
      #v(1fr)
      #if "verlag" in daten [#tftn-feld("verlag")[#daten.verlag]]
    ]
    pagebreak()
    // Impressum auf der Rückseite
    set text(size: 8.5pt)
    v(1fr)
    let jahr = daten.at("jahr", default: none)
    [© #if jahr != none [#tftn-feld("jahr")[#jahr]] #autoren.join(", ")]
    if "auflage" in daten [ \ #tftn-feld("auflage")[#daten.auflage]. #wort.auflage]
    if "verlag" in daten [ \ #tftn-feld("verlag")[#daten.verlag]#if (
        "ort" in daten
      ) [, #tftn-feld("ort")[#daten.ort]]]
    if "isbn" in daten [ \ ISBN #tftn-feld("isbn")[#daten.isbn]]
    if "widmung" in daten {
      pagebreak()
      v(40mm)
      align(center, emph(tftn-feld("widmung")[#eval(daten.widmung, mode: "markup")]))
    }
  }
  // Inhaltsverzeichnis mit römischen Seitenzahlen. Die Marke steht vor `set page`, denn die Regel
  // beginnt eine neue Seite.
  _sprung()
  set page(numbering: "i", footer: context if not (_kapitelseite() or _leerseite()) {
    align(if calc.odd(here().page()) { right } else { left }, counter(page).display("i"))
  })
  pagebreak(to: "odd")
  counter(page).update(1)
  {
    set par(first-line-indent: 0pt)
    show outline.entry.where(level: 1): set block(above: 1em)
    outline(title: wort.inhalt, indent: auto)
  }
  // Hauptteil: arabische Seitenzahlen, Kolumnentitel (gerade Seite Buchtitel, ungerade Kapitel)
  _sprung()
  set page(
    numbering: "1",
    header: context if not (_kapitelseite() or _leerseite()) {
      set text(size: 8.5pt, style: "italic")
      let gerade = calc.even(here().page())
      let zahl = counter(page).display("1")
      if gerade {
        grid(
          columns: (auto, 1fr),
          zahl, align(right, titel),
        )
      } else {
        grid(
          columns: (1fr, auto),
          _kapitel(), zahl,
        )
      }
    },
    footer: context if _kapitelseite() { align(center, counter(page).display("1")) },
  )
  pagebreak(to: "odd")
  counter(page).update(1)
  body
  if pfad != none and dateien.len() > 0 {
    set heading(numbering: none)
    bibliography(dateien.map(pfad), title: wort.literatur)
  }
}
