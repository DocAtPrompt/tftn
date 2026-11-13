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
  ),
  en: (
    seite: "Page {n} of {gesamt}",
    patient: "Patient",
    geb: "born",
    svnr: "Insurance no.",
    vertraulich: "Confidential – contains health data",
    gruss: "Kind regards",
  ),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

#let _kurzdatum(d, sprache) = if sprache == "de" {
  d.display("[day].[month].[year]")
} else { d.display("[day] [month repr:short] [year]") }

/// Setzt den Befund: `#show: befund.with(daten: toml("dokument.toml"))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - body (content): Diagnosen, Befund, Beurteilung usw. aus `main.typ`
/// -> content
#let befund(daten: (:), body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let praxis = daten.at("praxis", default: (:))
  let patient = daten.at("patient", default: (:))
  let pname = patient.at("name", default: "")
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

  // Ordinationskopf
  grid(
    columns: (1fr, auto),
    gutter: 6mm,
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
  v(12mm)
  tftn-feld("unterschrift")[#daten.at("unterschrift", default: praxis.at("name", default: ""))]
}
