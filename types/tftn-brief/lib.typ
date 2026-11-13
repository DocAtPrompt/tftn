// tftn-brief: Brief mit Anschrift im Fenster eines DL-Kuverts (nach ÖNORM A 1080 und DIN 5008).

#import "@local/tftn-kern:0.2.0": datum-text, hinweis

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (seite: "Seite {n} von {gesamt}", gruss: "Mit freundlichen Grüßen"),
  en: (seite: "Page {n} of {gesamt}", gruss: "Kind regards"),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

/// Setzt den Brief: `#show: brief.with(daten: toml("dokument.toml"))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - body (content): der Brieftext aus `main.typ`, mit Anrede
/// -> content
#let brief(daten: (:), body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let absender = daten.at("absender", default: (:))
  let name = absender.at("name", default: "")
  set document(title: daten.at("betreff", default: none), author: name)
  set page(
    paper: "a4",
    margin: (left: 25mm, right: 20mm, top: 25mm, bottom: 25mm),
    footer: context {
      let gesamt = counter(page).final().first()
      if gesamt > 1 {
        align(right, text(
          size: 8pt,
          wort.seite.replace("{n}", str(here().page())).replace("{gesamt}", str(gesamt)),
        ))
      }
    },
  )
  set text(font: "Source Sans 3", size: 11pt, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(leading: 0.65em, spacing: 1.2em)

  // Absender rechts oben
  place(top + right, align(left, text(size: 9pt)[
    #tftn-feld("absender")[*#name*]
    #for z in absender.at("zeilen", default: ()) [ \ #tftn-feld("absender")[#z] ]
    #if "telefon" in absender [ \ #tftn-feld("absender")[#absender.telefon] ]
    #if "email" in absender [ \ #tftn-feld("absender")[#absender.email] ]
  ]))
  // Anschriftfeld 45 mm von oben, 85 × 45 mm ab 20 mm vom Rand; die Schrift beginnt nach 5 mm
  // Innenabstand bündig mit dem Brieftext.
  place(top + left, dy: 20mm, block(width: 80mm, height: 45mm)[
    #text(size: 7pt)[#(
      (
        (name,) + absender.at("zeilen", default: ())
      )
        .filter(z => z != "")
        .join(" · ")
    )]
    #v(2mm)
    #tftn-feld("empfaenger")[#daten.at("empfaenger", default: ()).join(linebreak())]
  ])
  v(72mm)
  align(right)[
    #if "ort" in daten [#tftn-feld("ort")[#daten.ort], ]#if "datum" in daten [#tftn-feld(
      "datum",
    )[#datum-text(daten.datum, sprache)]]
  ]
  v(1em)
  if "betreff" in daten {
    tftn-feld("betreff")[*#daten.betreff*]
    v(1em)
  }
  body
  v(1em)
  tftn-feld("gruss")[#daten.at("gruss", default: wort.gruss)]
  v(14mm)
  tftn-feld("unterschrift")[#daten.at("unterschrift", default: name)]
}
