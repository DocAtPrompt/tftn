// tftn-befund: Befund oder Arztbrief mit Ordinationskopf, Patientenzeile und Kopfzeile auf
// Folgeseiten. Echte Patientendaten gehören nie in ein Git-Repository (KONZEPT E6).

#import "@local/tftn-kern:0.2.0": datum-text, hinweis

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (
    seite: "Seite {n} von {gesamt}",
    patient: "Patient/in",
    geb: "geb.",
    svnr: "SVNR",
    vertraulich: "Vertraulich – enthält Gesundheitsdaten",
    gruss: "Mit freundlichen Grüßen",
    ohne-pfad: "Bild „{datei}“: main.typ braucht pfad: p => path(p) (Handbuch, Kapitel 1)",
  ),
  en: (
    seite: "Page {n} of {gesamt}",
    patient: "Patient",
    geb: "born",
    svnr: "Insurance no.",
    vertraulich: "Confidential – contains health data",
    gruss: "Kind regards",
    ohne-pfad: "Image “{datei}”: main.typ needs pfad: p => path(p) (manual, chapter 1)",
  ),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

// Ein Datum als Text (etwa der Ersatztext „[geb. 1970]“, den TFTN einem KI-Assistenten zeigt)
// bleibt, wie es ist.
#let _kurzdatum(d, sprache) = if type(d) == str { d } else if sprache == "de" {
  d.display("[day].[month].[year]")
} else { d.display("[day] [month repr:short] [year]") }

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

/// Setzt den Befund: `#show: befund.with(daten: toml("dokument.toml"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; nötig für Logo und
///   Unterschrift als Bild
/// - body (content): Diagnosen, Befund, Beurteilung usw. aus `main.typ`
/// -> content
#let befund(daten: (:), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let praxis = daten.at("praxis", default: (:))
  let patient = daten.at("patient", default: (:))
  let pname = patient.at("name", default: "")
  let logo = daten.at("logo", default: "")
  let unterschrift-bild = daten.at("unterschrift_bild", default: "")
  let geboren = if "geburtsdatum" in patient {
    wort.geb + " " + _kurzdatum(patient.geburtsdatum, sprache)
  } else { none }
  set document(title: daten.at("titel", default: none), author: praxis.at("name", default: ""))
  set page(
    paper: "a4",
    margin: (left: 25mm, right: 20mm, top: 25mm, bottom: 22mm),
    header: context if here().page() > 1 {
      set text(size: 8pt)
      grid(
        columns: (1fr, auto),
        [#praxis.at("name", default: "") · #daten.at("titel", default: "")],
        [#wort.patient: #pname#if geboren != none [, #geboren]],
      )
      line(length: 100%, stroke: 0.4pt)
    },
    footer: context {
      set text(size: 7.5pt, fill: luma(90))
      let gesamt = counter(page).final().first()
      grid(
        columns: (1fr, auto),
        wort.vertraulich,
        wort.seite.replace("{n}", str(here().page())).replace("{gesamt}", str(gesamt)),
      )
    },
  )
  set text(font: "Source Sans 3", size: 10.5pt, lang: sprache, region: if sprache == "de" {
    "AT"
  } else { none })
  set par(leading: 0.6em, spacing: 1em)
  show heading: it => block(above: 1.3em, below: 0.6em, text(size: 10.5pt, weight: "bold", it.body))

  // Ordinationskopf, links davor das Logo
  grid(
    columns: if logo != "" { (auto, 1fr, auto) } else { (1fr, auto) },
    gutter: 6mm,
    ..if logo != "" {
      (
        tftn-feld("logo", _bild(
          logo,
          pfad,
          daten.at("logo_hoehe", default: 1.5) * 1cm,
          60mm,
          wort,
        )),
      )
    },
    [
      #tftn-feld("praxis")[#text(size: 14pt, weight: "bold")[#praxis.at("name", default: "")]]
      #if "fach" in praxis [ \ #tftn-feld("praxis")[#praxis.fach]]
    ],
    align(right, text(size: 8.5pt)[
      #for z in praxis.at("zeilen", default: ()) [#tftn-feld("praxis")[#z] \ ]
      #if "telefon" in praxis [#tftn-feld("praxis")[#praxis.telefon] \ ]
      #if "email" in praxis [#tftn-feld("praxis")[#praxis.email]]
    ]),
  )
  line(length: 100%, stroke: 0.6pt)
  if "empfaenger" in daten {
    v(4mm)
    tftn-feld("empfaenger")[#daten.empfaenger.join(linebreak())]
  }
  align(right)[
    #if "ort" in daten [#tftn-feld("ort")[#daten.ort], ]#if "datum" in daten [#tftn-feld(
      "datum",
    )[#datum-text(daten.datum, sprache)]]
  ]
  v(2mm)
  tftn-feld("titel")[#text(size: 15pt, weight: "bold")[#daten.at("titel", default: "")]]
  v(1mm)
  block(fill: luma(240), inset: (x: 3mm, y: 2mm), width: 100%, tftn-feld("patient")[
    #wort.patient: *#pname*#if geboren != none [ · #geboren]#if "svnr" in patient [ · #wort.svnr #patient.svnr]
  ])
  body
  v(1.2em)
  tftn-feld("gruss")[#daten.at("gruss", default: wort.gruss)]
  // Platz für die Unterschrift von Hand oder das Bild der Unterschrift
  if unterschrift-bild != "" {
    block(above: 2mm, below: 2mm, tftn-feld("unterschrift_bild", _bild(
      unterschrift-bild,
      pfad,
      14mm,
      70mm,
      wort,
    )))
  } else { v(12mm) }
  tftn-feld("unterschrift")[#daten.at("unterschrift", default: praxis.at("name", default: ""))]
}
