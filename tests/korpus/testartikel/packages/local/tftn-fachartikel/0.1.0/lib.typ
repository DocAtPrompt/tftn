// tftn-fachartikel: Satz eines Fachartikels aus main.typ (Text) und dokument.toml (Formular).

#import "@local/tftn-kern:0.1.0": datum-at, hinweis, trennungen

/// Setzt den ganzen Artikel: `#show: fachartikel.with(daten: toml("dokument.toml"))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - trennliste (str, array): Inhalt von `trennungen.txt`
/// - stil (str): Zitierstil (CSL-Datei oder eingebauter Name), Platzhalter bis E7 entschieden ist
/// - body (content): der Text aus `main.typ`
/// -> content
#let fachartikel(
  daten: (:),
  trennliste: (),
  stil: "american-psychological-association",
  body,
) = {
  let autoren = daten.at("autoren", default: ())
  set document(
    title: daten.at("titel", default: none),
    author: autoren.map(a => a.name),
    keywords: daten.at("schlagworte", default: ()),
  )
  set page(paper: "a4", margin: (x: 25mm, y: 25mm), numbering: "1")
  set text(font: "Libertinus Serif", size: 11pt, lang: "de", region: "AT")
  set par(justify: true, leading: 0.65em)
  set heading(numbering: "1.1")
  set math.equation(numbering: "(1)")
  show raw: set text(font: "DejaVu Sans Mono", size: 9pt)
  set bibliography(style: stil, title: "Literatur")
  show: trennungen(trennliste)

  // Titelei (Text aus dokument.toml; Klicks darauf springen nicht in main.typ, KONZEPT 4.4)
  align(center)[
    #text(size: 17pt, weight: "bold", daten.at("titel", default: "Ohne Titel"))
    #if "untertitel" in daten [ \ #text(size: 13pt, daten.untertitel) ]
    #v(0.6em)
    #autoren.map(a => a.name).join(" · ")
    #let orte = autoren.map(a => a.at("institution", default: ())).flatten().dedup()
    #if orte.len() > 0 [ \ #text(size: 9pt, orte.join(" · ")) ]
    #if "datum" in daten [ \ #text(size: 9pt, datum-at(daten.datum)) ]
  ]
  v(1em)
  if "abstract" in daten or "schlagworte" in daten {
    block(inset: (x: 1.5em), text(size: 9.5pt)[
      #if "abstract" in daten [*Zusammenfassung.* #eval(daten.abstract, mode: "markup")]
      #if "schlagworte" in daten [

        *Schlagwörter:* #daten.schlagworte.join(" · ")
      ]
    ])
  }
  body
}
