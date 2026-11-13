#import "@local/tftn-bericht:0.1.0": *

#show: bericht.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

= Introduction

The report starts here. Title, authors, date and references are in the “Document” pane of the
sidebar. Citations come from `literatur.bib`, for example @beispielBericht2026.

== Research question

Subsections get numbers such as 1.1.

= Results

Each chapter starts on a new page.
