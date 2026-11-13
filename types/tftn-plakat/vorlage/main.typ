#import "@local/tftn-plakat:0.1.0": *

#show: plakat.with(daten: toml("dokument.toml"), pfad: p => path(p))

= Hintergrund

Hier beginnt das Plakat. Titel, Autorinnen und Autoren, Format, Spalten und Farbe stehen im Fach
„Dokument“ der Seitenleiste. Jeder Abschnitt beginnt mit `=` und bekommt ein farbiges Band.

= Methode

Text fließt von Spalte zu Spalte. Bilder bindest du mit `#image("bild.png", width: 100%)` ein.

#kasten(titel: [Auf einen Blick:])[Das Wichtigste in einem Satz.]

= Ergebnisse

Zitate kommen aus `literatur.bib`, etwa @beispielPlakat2026.

= Schlussfolgerung

Kurz und groß genug, um es aus zwei Metern zu lesen.
