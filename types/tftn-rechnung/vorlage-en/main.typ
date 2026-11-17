#import "@local/tftn-rechnung:0.2.0": *

#let daten = toml(sys.inputs.at("daten", default: "dokument.toml"))
#show: rechnung.with(daten: daten)

Please find below the invoice for my services:
