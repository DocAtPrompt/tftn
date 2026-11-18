#import "@local/tftn-rechnung:0.3.0": *

#let daten = toml(sys.inputs.at("daten", default: "dokument.toml"))
#show: rechnung.with(daten: daten, pfad: p => path(p))

für meine Leistungen erlaube ich mir zu verrechnen:
