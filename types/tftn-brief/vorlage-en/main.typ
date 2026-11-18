#import "@local/tftn-brief:0.3.0": *

#let daten = toml(sys.inputs.at("daten", default: "dokument.toml"))
#show: brief.with(daten: daten, pfad: p => path(p))

The letter starts here. Sender, address, place, date, subject and closing are in the “Document”
pane of the sidebar; this file holds only the text.

A second paragraph shows the space between paragraphs.
