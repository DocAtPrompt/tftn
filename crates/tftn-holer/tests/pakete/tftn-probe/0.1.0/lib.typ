// Testpaket für PLAN-M2 AP9: nutzt ein zweites Paket, damit das Holen transitiv geprüft wird.
#import "@preview/tftn-probe-basis:0.1.0": rahmen

#let probe(body) = rahmen[Probe: #body]
