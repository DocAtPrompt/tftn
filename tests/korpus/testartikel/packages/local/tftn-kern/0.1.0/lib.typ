// tftn-kern: Bausteine, die alle TFTN-Dokumenttypen teilen (KONZEPT 2.9, 5.4).

/// Monatsnamen in österreichischer Form.
#let monate-at = (
  "Jänner", "Februar", "März", "April", "Mai", "Juni",
  "Juli", "August", "September", "Oktober", "November", "Dezember",
)

/// Schreibt ein Datum österreichisch aus, z. B. „15. Jänner 2026“.
/// Typst kennt keine deutschen Monatsnamen (`datetime.display` liefert „January“).
///
/// - d (datetime): das Datum
/// - feber (bool): „Feber“ statt „Februar“
/// -> str
#let datum-at(d, feber: false) = {
  let monat = if feber and d.month() == 2 { "Feber" } else { monate-at.at(d.month() - 1) }
  str(d.day()) + ". " + monat + " " + str(d.year())
}

/// Liest eine Liste von Trennausnahmen. Ein Wort je Zeile, erlaubte Trennstellen mit `-`
/// markiert, z. B. `Sozial-versicherungs-träger`. Leere Zeilen und Zeilen mit `#` werden übersprungen.
///
/// - eintraege (str, array): Inhalt von `trennungen.txt` oder eine Liste
/// -> array
#let trennliste(eintraege) = {
  let zeilen = if type(eintraege) == str { eintraege.split("\n") } else { eintraege }
  zeilen.map(z => z.trim()).filter(z => z != "" and not z.starts-with("#"))
}

/// Trennausnahmen als Show-Regel: `#show: trennungen(read("trennungen.txt"))`.
/// Jedes gelistete Wort wird nur noch an den markierten Stellen getrennt. Typst hat dafür
/// keine eigene Funktion (Issue typst/typst#6877); die Regel setzt weiche Trennstriche und
/// schaltet die automatische Trennung für das Wort ab. Das Wort wird auch innerhalb längerer
/// Wörter gefunden (Zusammensetzungen, Beugungsformen); Groß-/Kleinschreibung zählt.
///
/// - eintraege (str, array): Inhalt von `trennungen.txt` oder eine Liste
/// -> function
#let trennungen(eintraege) = body => trennliste(eintraege).fold(body, (inhalt, e) => {
  show e.replace("-", ""): text(hyphenate: false, e.split("-").join("\u{ad}"))
  inhalt
})

#let hinweis-titel = (info: "Hinweis", warnung: "Achtung", merke: "Merke")

/// Hinweiskasten mit Überschrift und farbigem Hintergrund.
///
/// - farbe (color): Hintergrundfarbe
/// - art (str): `"info"`, `"warnung"` oder `"merke"`; bestimmt die Überschrift
/// - body (content): Inhalt
/// -> content
#let hinweis(farbe: luma(230), art: "info", body) = {
  assert(art in hinweis-titel, message: "hinweis: art muss info, warnung oder merke sein")
  block(fill: farbe, inset: 8pt, radius: 3pt, width: 100%, breakable: false)[
    *#hinweis-titel.at(art):* #body
  ]
}
