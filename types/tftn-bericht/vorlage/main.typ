#import "@local/tftn-bericht:0.1.0": *

#show: bericht.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

= Einleitung

Hier beginnt der Bericht. Titel, Autorinnen und Autoren, Datum und Literatur stehen im Fach
„Dokument“ der Seitenleiste. Zitate kommen aus `literatur.bib`, etwa @beispielBericht2026.

== Fragestellung

Unterabschnitte bekommen Nummern wie 1.1.

= Ergebnisse

Jedes Kapitel beginnt auf einer neuen Seite.
