// tftn-rechnung: Rechnung mit Anschrift im Fenster eines DL-Kuverts, Posten und Summen.
// Die Rechnungsnummer vergibt der Benutzer selbst; TFTN zählt nicht mit.

#import "@local/tftn-kern:0.2.0": datum-text

/// Beschriftungen je Sprache; andere Sprachen als Deutsch bekommen Englisch.
#let beschriftungen = (
  de: (
    titel: "Rechnung",
    nummer: "Rechnungsnummer",
    datum: "Rechnungsdatum",
    leistung: "Leistungszeitraum",
    pos: "Pos.",
    text: "Bezeichnung",
    menge: "Menge",
    preis: "Einzelpreis",
    betrag: "Betrag",
    netto: "Summe netto",
    ust: "USt {satz} %",
    gesamt: "Gesamtbetrag",
    zahlbar: "Zahlbar ohne Abzug bis {datum}.",
    seite: "Seite {n} von {gesamt}",
    bank: "Bankverbindung",
    uid: "UID",
    gruss: "Mit freundlichen Grüßen",
    heilbehandlung: "Umsatzsteuerfrei gemäß § 6 Abs. 1 Z 19 UStG (Heilbehandlung im Bereich der Humanmedizin).",
    kleinunternehmer: "Umsatzsteuerbefreit – Kleinunternehmer gemäß § 6 Abs. 1 Z 27 UStG.",
    ohne-pfad: "Bild „{datei}“: main.typ braucht pfad: p => path(p) (Handbuch, Kapitel 1)",
    tausend: ".",
    komma: ",",
  ),
  en: (
    titel: "Invoice",
    nummer: "Invoice number",
    datum: "Invoice date",
    leistung: "Service period",
    pos: "No.",
    text: "Description",
    menge: "Qty",
    preis: "Unit price",
    betrag: "Amount",
    netto: "Net total",
    ust: "VAT {satz} %",
    gesamt: "Total",
    zahlbar: "Payable without deduction by {datum}.",
    seite: "Page {n} of {gesamt}",
    bank: "Bank details",
    uid: "VAT ID",
    gruss: "Kind regards",
    heilbehandlung: "Exempt from VAT under § 6 (1) 19 of the Austrian VAT Act (medical treatment).",
    kleinunternehmer: "Exempt from VAT as a small business under § 6 (1) 27 of the Austrian VAT Act.",
    ohne-pfad: "Image “{datei}”: main.typ needs pfad: p => path(p) (manual, chapter 1)",
    tausend: ",",
    komma: ".",
  ),
)

/// Marke für das Fach „Dokument“: Ein Klick in der Vorschau führt zum Formularfeld `name`.
#let tftn-feld(name, inhalt) = inhalt

/// Zahl aus `dokument.toml` als exakte Dezimalzahl (0,335 bleibt 0,335, nicht 0,33499…).
#let dez(x) = decimal(str(x))

/// Rundet auf Cent (kaufmännisch, halbe Cent weg von null) und gibt ganze Cent zurück.
#let cent(x) = int(calc.round(dez(x) * 100))

/// Ziffern mit Tausendertrennzeichen `t`.
#let gruppiert(ziffern, t) = {
  let s = str(ziffern)
  let teile = ()
  while s.len() > 3 {
    teile.insert(0, s.slice(s.len() - 3))
    s = s.slice(0, s.len() - 3)
  }
  teile.insert(0, s)
  teile.join(t)
}

/// Geldbetrag aus ganzen Cent, z. B. „1.234,50“ (de) bzw. „1,234.50“ (en).
#let geld(c, wort) = {
  let minus = if c < 0 { "−" } else { "" }
  let a = calc.abs(c)
  let rest = str(calc.rem(a, 100))
  (
    minus
      + gruppiert(calc.quo(a, 100), wort.tausend)
      + wort.komma
      + (if rest.len() < 2 { "0" } else { "" })
      + rest
  )
}

/// Zahl ohne überflüssige Nachkommastellen, mit dem Komma der Sprache.
#let zahl(x, wort) = {
  if calc.fract(x) == 0 { str(int(x)) } else { str(x).replace(".", wort.komma) }
}

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

/// Setzt die Rechnung: `#show: rechnung.with(daten: toml("dokument.toml"), pfad: p => path(p))`.
///
/// - daten (dictionary): Inhalt von `dokument.toml` (Schema `tftn/formular.schema.json`)
/// - pfad (function, none): macht aus einem Pfad im Projekt einen `path`; nötig für das Logo
/// - body (content): Text vor den Posten aus `main.typ`
/// -> content
#let rechnung(daten: (:), pfad: none, body) = {
  let sprache = daten.at("sprache", default: "de")
  let wort = beschriftungen.at(sprache, default: beschriftungen.en)
  let absender = daten.at("absender", default: (:))
  let name = absender.at("name", default: "")
  let bank = daten.at("bank", default: (:))
  let waehrung = daten.at("waehrung", default: "€")
  let nummer = daten.at("nummer", default: "")
  let logo = daten.at("logo", default: "")
  set document(title: wort.titel + " " + nummer, author: name)

  let fuss = {
    let teile = ()
    if bank.len() > 0 {
      teile.push(
        wort.bank
          + ": "
          + (
            bank.at("institut", default: ""),
            if "iban" in bank { "IBAN " + bank.iban },
            if "bic" in bank { "BIC " + bank.bic },
          )
            .filter(x => x != none and x != "")
            .join(" · "),
      )
    }
    if "uid" in absender { teile.push(wort.uid + " " + absender.uid) }
    teile
  }

  set page(
    paper: "a4",
    margin: (left: 25mm, right: 20mm, top: 25mm, bottom: 30mm),
    footer: context {
      set text(size: 8pt)
      let gesamt = counter(page).final().first()
      grid(
        columns: (1fr, auto),
        column-gutter: 1em,
        tftn-feld("bank")[#fuss.join(linebreak())],
        if gesamt > 1 {
          wort.seite.replace("{n}", str(here().page())).replace("{gesamt}", str(gesamt))
        },
      )
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
  // Anschriftfeld wie beim Brief (ÖNORM A 1080 / DIN 5008, Fenster links)
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

  grid(
    columns: (1fr, auto),
    align: (left + bottom, left + bottom),
    text(size: 18pt, weight: "bold", wort.titel),
    table(
      columns: 2,
      stroke: none,
      inset: (x: 0pt, y: 2pt),
      column-gutter: 1em,
      wort.nummer + ":", tftn-feld("nummer")[*#nummer*],
      ..if "datum" in daten {
        (wort.datum + ":", tftn-feld("datum")[#datum-text(daten.datum, sprache)])
      },
      ..if "leistung" in daten {
        (wort.leistung + ":", tftn-feld("leistung")[#daten.leistung])
      },
    ),
  )
  v(1em)
  if "anrede" in daten {
    par(tftn-feld("anrede")[#daten.anrede])
  }
  body

  // Posten: jede Zeile auf Cent gerundet, dann summiert
  let posten = daten.at("posten", default: ())
  let zeilen = posten.map(p => {
    let menge = p.at("menge", default: 1)
    (p, menge, cent(dez(menge) * dez(p.at("preis", default: 0))))
  })
  let netto = zeilen.map(z => z.at(2)).sum(default: 0)
  // Steuerbefreiung (Heilbehandlung, Kleinunternehmer): keine USt, Hinweis aus den Beschriftungen,
  // außer es steht ein eigener Steuerhinweis da.
  let befreiung = daten.at("befreiung", default: none)
  let befreit = befreiung in ("heilbehandlung", "kleinunternehmer")
  let satz = if befreit { 0 } else { daten.at("ust", default: 20) }
  let hinweis = daten.at("steuerhinweis", default: if befreit { wort.at(befreiung) })
  // netto ist schon in Cent: Steuer auf ganze Cent runden
  let steuer = int(calc.round(decimal(netto) * dez(satz) / 100))
  let mit-w(c) = geld(c, wort) + " " + waehrung

  v(0.5em)
  tftn-feld("posten", table(
    columns: (auto, 1fr, auto, auto, auto),
    align: (right, left, right, right, right),
    stroke: (x, y) => if y == 0 { (bottom: 0.6pt) } else { (bottom: 0.3pt + luma(200)) },
    inset: (x: 4pt, y: 5pt),
    table.header(
      ..(wort.pos, wort.text, wort.menge, wort.preis, wort.betrag).map(x => text(weight: "bold", x))
    ),
    ..zeilen
      .enumerate()
      .map(((i, (p, menge, betrag))) => (
        str(i + 1),
        p.at("text", default: ""),
        zahl(menge, wort) + if "einheit" in p { " " + p.einheit },
        geld(cent(p.at("preis", default: 0)), wort),
        geld(betrag, wort),
      ))
      .flatten(),
  ))

  // Summen rechtsbündig
  align(right, tftn-feld("ust", table(
    columns: 2,
    align: right,
    stroke: none,
    inset: (x: 4pt, y: 3pt),
    ..if satz > 0 {
      (
        wort.netto,
        mit-w(netto),
        wort.ust.replace("{satz}", zahl(satz, wort)),
        mit-w(steuer),
      )
    },
    table.hline(stroke: 0.6pt),
    text(weight: "bold", wort.gesamt), text(weight: "bold", mit-w(netto + steuer)),
  )))
  if hinweis != none {
    tftn-feld(
      if "steuerhinweis" in daten { "steuerhinweis" } else { "befreiung" },
      text(size: 9pt, hinweis),
    )
  }
  if "zahlungsziel" in daten and "datum" in daten {
    let bis = daten.datum + duration(days: daten.zahlungsziel)
    par(tftn-feld("zahlungsziel", wort.zahlbar.replace("{datum}", datum-text(bis, sprache))))
  }
  v(1em)
  tftn-feld("gruss")[#daten.at("gruss", default: wort.gruss)]
  v(10mm)
  tftn-feld("absender")[#name]
}
