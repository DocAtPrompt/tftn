#import "@local/tftn-fachbuch:0.1.0": *

#show: fachbuch.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

#include "chapters/01-introduction.typ"
#include "chapters/02-basics.typ"
