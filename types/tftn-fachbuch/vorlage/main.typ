#import "@local/tftn-fachbuch:0.1.0": *

#show: fachbuch.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

#include "kapitel/01-einleitung.typ"
#include "kapitel/02-grundlagen.typ"
