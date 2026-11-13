#import "@local/tftn-fachartikel:0.3.0": *

#show: fachartikel.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

= Introduction

The text starts here. Citations come from `literatur.bib`, for example
@exampleIntroduction2026[p. 3].

#hinweis[Title, authors, date, abstract and references are in the “Document” tab of the
  sidebar.]
