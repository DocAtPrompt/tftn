#import "@local/tftn-lebenslauf:0.1.0": *

#show: lebenslauf.with(daten: toml("dokument.toml"), pfad: p => path(p))

= Berufserfahrung

#station("2020–heute", "Tätigkeit", ort: "Arbeitgeber, Ort")[
  Aufgaben und Erfolge in ein, zwei Sätzen. Neue Stationen fügst du über „Einfügen“ ein.
]
#station("2015–2020", "Frühere Tätigkeit", ort: "Arbeitgeber, Ort")[]

= Ausbildung

#station("2009–2015", "Studium", ort: "Universität, Ort")[Abschluss und Schwerpunkt.]

= Kenntnisse

#station("Sprachen", "Deutsch, Englisch")[]
