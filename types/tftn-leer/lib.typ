// Leeres Dokument (TFTN): Kopf- und Fußzeilen, Randtext.

// Doppelseitig: Die Seite hat Innen- und Außenrand.
#let _doppelseitig() = type(page.margin) == dictionary and "inside" in page.margin

// {seite} und {kapitel} durch die Seitenzahl bzw. die letzte Überschrift ersetzen.
#let _einsetzen(vorlage) = {
  let seite = str(counter(page).get().first())
  // Beginnt auf dieser Seite ein Kapitel, gilt es; sonst das letzte davor.
  let hier = query(heading.where(level: 1)).filter(h => h.location().page() == here().page())
  let davor = query(selector(heading.where(level: 1)).before(here()))
  let kapitel = if hier.len() > 0 { hier.first().body } else if davor.len() > 0 {
    davor.last().body
  } else { [] }
  vorlage.replace("{seite}", seite).split("{kapitel}").join(kapitel)
}

#let _zeile(innen, mitte, aussen, erste) = context {
  if erste or here().page() != 1 {
    let gerade = _doppelseitig() and calc.even(here().page())
    let (links, rechts) = if gerade { (aussen, innen) } else { (innen, aussen) }
    grid(
      columns: (1fr, auto, 1fr),
      align(left, _einsetzen(links)),
      align(center, _einsetzen(mitte)),
      align(right, _einsetzen(rechts)),
    )
  }
}

#let kopfzeile(innen: "", mitte: "", aussen: "", erste: true) = _zeile(innen, mitte, aussen, erste)

#let fusszeile(innen: "", mitte: "", aussen: "", erste: true) = _zeile(innen, mitte, aussen, erste)

// Breite eines Seitenrands; ohne Angabe die Vorgabe von Typst.
#let _rand(name) = {
  let m = page.margin
  if type(m) == dictionary and m.at(name, default: auto) != auto {
    m.at(name)
  } else if type(m) == length {
    m
  } else {
    2.5 / 21 * calc.min(page.width, page.height)
  }
}

#let _randnotiz_seite(body, breite, abstand) = box(context {
  let doppelt = _doppelseitig()
  let ziel = if doppelt and calc.even(here().page()) {
    _rand("outside") - abstand - breite
  } else {
    let seite = if page.flipped { page.height } else { page.width }
    seite - _rand(if doppelt { "outside" } else { "right" }) + abstand
  }
  place(dx: ziel - here().position().x, dy: -0.7em, box(width: breite, {
    set par(justify: false, leading: 0.5em)
    set text(size: 8pt)
    body
  }))
})

// Text am äußeren Rand, auf Höhe der Stelle im Absatz. Die Box hält den Absatz zusammen,
// die Stelle auf der Seite (`here`) bestimmt den Abstand zum Rand. Im HTML (Export nach Word)
// gibt es keinen Rand: Dort steht der Randtext in Klammern im Text.
#let randnotiz(body, breite: 2.2cm, abstand: 0.3cm) = context if target() == "html" {
  [ (#emph(body))]
} else { _randnotiz_seite(body, breite, abstand) }
