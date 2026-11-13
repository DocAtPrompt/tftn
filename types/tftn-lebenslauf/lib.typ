// tftn-lebenslauf: Lebenslauf (CV) mit Kopf, Kontakt, Abschnitten und Stationen in zwei Spalten.

#import "@local/tftn-kern:0.2.0": datum-text, hinweis

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (geboren: "geboren am", seite: "Seite {n} von {gesamt}"),
  en: (geboren: "born", seite: "Page {n} of {gesamt}"),
)

#let _farbe = rgb("#1f4e79")

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

/// Eine Station: Zeitraum links, Tätigkeit und Ort rechts, darunter die Beschreibung.
///
/// - zeit (str, content): Zeitraum, z. B. "2020–heute"
/// - titel (str, content): Tätigkeit oder Abschluss
/// - ort (str, content, none): Arbeitgeber, Schule, Ort
/// - body (content): Beschreibung, darf leer sein
/// -> content
#let station(zeit, titel, ort: none, body) = block(breakable: false, above: 0.9em, grid(
  columns: (32mm, 1fr),
  column-gutter: 5mm,
  text(fill: luma(80), zeit),
  [
    *#titel*#if ort != none [ · #ort]
    #if body != [] [ \ #text(size: 0.92em, body)]
  ],
))

/// Setzt den Lebenslauf: `#show: lebenslauf.with(daten: toml("dokument.toml"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; ohne sie kein Foto
/// - body (content): Abschnitte (`=`) mit Stationen aus `main.typ`
/// -> content
#let lebenslauf(daten: (:), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let name = daten.at("name", default: "")
  let kontakt = daten.at("kontakt", default: (:))
  set document(title: name, author: name)
  set page(
    paper: "a4",
    margin: (x: 22mm, top: 20mm, bottom: 22mm),
    footer: context {
      let gesamt = counter(page).final().first()
      if gesamt > 1 {
        align(right, text(
          size: 8pt,
          fill: luma(90),
          wort.seite.replace("{n}", str(here().page())).replace("{gesamt}", str(gesamt)),
        ))
      }
    },
  )
  set text(font: "Source Sans 3", size: 10.5pt, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(leading: 0.6em, spacing: 0.8em)
  show heading: it => block(above: 1.6em, below: 0.4em, width: 100%)[
    #text(size: 12.5pt, weight: "bold", fill: _farbe, it.body)
    #v(-0.5em)
    #line(length: 100%, stroke: 0.6pt + _farbe)
  ]

  let kopf = [
    #tftn-feld("name")[#text(size: 24pt, weight: "bold")[#name]]
    #if "beruf" in daten [ \ #tftn-feld("beruf")[#text(size: 13pt, fill: luma(60))[#daten.beruf]]]
    #v(3mm)
    #set text(size: 9.5pt)
    #let teile = (
      kontakt.at("zeilen", default: ()).join(", "),
      kontakt.at("telefon", default: none),
      kontakt.at("email", default: none),
      kontakt.at("web", default: none),
    ).filter(z => z != none and z != "")
    #tftn-feld("kontakt")[#teile.join(" · ")]
    #if "geburtsdatum" in daten [ \ #tftn-feld("geburtsdatum")[#wort.geboren #datum-text(
        daten.geburtsdatum,
        sprache,
      )]]
  ]
  if pfad != none and "foto" in daten {
    grid(
      columns: (1fr, 32mm),
      column-gutter: 6mm,
      kopf, tftn-feld("foto", image(pfad(daten.foto), width: 32mm)),
    )
  } else { kopf }
  body
  if "ort" in daten or "datum" in daten {
    v(2em)
    [#if "ort" in daten [#tftn-feld("ort")[#daten.ort], ]#if "datum" in daten [#tftn-feld(
        "datum",
      )[#datum-text(daten.datum, sprache)]]]
  }
}
