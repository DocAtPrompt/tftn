#import "@local/tftn-fachartikel:0.2.0": *

#show: fachartikel.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

= Einleitung

Hier beginnt der Text. Zitate werden aus `literatur.bib` eingefügt, etwa
@beispielEinfuehrung2026[S. 3].

#hinweis[Titel, Autorinnen und Autoren, Datum, Zusammenfassung und Literatur stehen im Fach
  „Dokument“ der Seitenleiste.]
