#import "@local/tftn-rechnung:0.2.0": *

#let daten = toml(sys.inputs.at("daten", default: "dokument.toml"))
#show: rechnung.with(daten: daten)

für meine Leistungen erlaube ich mir zu verrechnen:
