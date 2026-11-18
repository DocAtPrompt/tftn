// tftn-brief: Brief mit Anschrift im Fenster eines DL-Kuverts (nach ÖNORM A 1080 und DIN 5008).

#import "@local/tftn-kern:0.2.0": datum-text, hinweis

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (
    seite: "Seite {n} von {gesamt}",
    gruss: "Mit freundlichen Grüßen",
    ohne-pfad: "Bild „{datei}“: main.typ braucht pfad: p => path(p) (Handbuch, Kapitel 1)",
  ),
  en: (
    seite: "Page {n} of {gesamt}",
    gruss: "Kind regards",
    ohne-pfad: "Image “{datei}”: main.typ needs pfad: p => path(p) (manual, chapter 1)",
  ),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

/// Ein Bild aus dem Projekt, `hoehe` hoch und höchstens `breite` breit. Ein Paket erreicht
/// Projektdateien nur über einen `path`, den `main.typ` erzeugt (`pfad: p => path(p)`); fehlt
/// `pfad` (eine `main.typ` von vor 0.3.0), steht statt des Bilds ein sichtbarer Hinweis.
#let _bild(datei, pfad, hoehe, breite, wort) = if pfad == none {
  box(width: 45mm, stroke: 0.6pt + red, inset: 2pt, text(
    size: 7pt,
    fill: red,
    wort.ohne-pfad.replace(
      "{datei}",
      datei,
    ),
  ))
} else {
  context {
    let b = image(pfad(datei), height: hoehe)
    if measure(b).width > breite { image(pfad(datei), width: breite) } else { b }
  }
}

/// Setzt den Brief: `#show: brief.with(daten: toml("dokument.toml"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; nötig für Logo und
///   Unterschrift als Bild
/// - body (content): der Brieftext aus `main.typ`, mit Anrede
/// -> content
#let brief(daten: (:), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let absender = daten.at("absender", default: (:))
  let name = absender.at("name", default: "")
  let logo = daten.at("logo", default: "")
  let unterschrift-bild = daten.at("unterschrift_bild", default: "")
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

  // Absender rechts oben, darüber das Logo
  place(top + right, stack(
    spacing: 4mm,
    ..if logo != "" {
      (
        align(right, tftn-feld("logo", _bild(
          logo,
          pfad,
          daten.at("logo_hoehe", default: 1.5) * 1cm,
          80mm,
          wort,
        ))),
      )
    },
    align(left, text(size: 9pt)[
      #tftn-feld("absender")[*#name*]
      #for z in absender.at("zeilen", default: ()) [ \ #tftn-feld("absender")[#z] ]
      #if "telefon" in absender [ \ #tftn-feld("absender")[#absender.telefon] ]
      #if "email" in absender [ \ #tftn-feld("absender")[#absender.email] ]
    ]),
  ))
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
  if "anrede" in daten {
    par(tftn-feld("anrede")[#daten.anrede])
  }
  body
  v(1em)
  tftn-feld("gruss")[#daten.at("gruss", default: wort.gruss)]
  // Platz für die Unterschrift von Hand oder das Bild der Unterschrift
  if unterschrift-bild != "" {
    block(above: 2mm, below: 2mm, tftn-feld("unterschrift_bild", _bild(
      unterschrift-bild,
      pfad,
      15mm,
      70mm,
      wort,
    )))
  } else { v(14mm) }
  tftn-feld("unterschrift")[#daten.at("unterschrift", default: name)]
}
