#import "@local/tftn-lebenslauf:0.1.0": *

#show: lebenslauf.with(daten: toml("dokument.toml"), pfad: p => path(p))

= Experience

#station("2020–present", "Position", ort: "Employer, place")[
  Duties and achievements in a sentence or two. Add new entries with “Insert”.
]
#station("2015–2020", "Earlier position", ort: "Employer, place")[]

= Education

#station("2009–2015", "University", ort: "University, place")[Degree and focus.]

= Skills

#station("Languages", "German, English")[]
