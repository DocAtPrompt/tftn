#import "@local/tftn-brief:0.3.0": *

#let daten = toml(sys.inputs.at("daten", default: "dokument.toml"))
#show: brief.with(daten: daten, pfad: p => path(p))

hier beginnt der Brief. Absender, Anschrift, Ort, Datum, Betreff und Grußformel stehen im Fach
„Dokument“ der Seitenleiste; hier steht nur der Text.

Ein zweiter Absatz zeigt den Abstand zwischen den Absätzen.
