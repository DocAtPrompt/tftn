#import "@local/tftn-plakat:0.1.0": *

#show: plakat.with(daten: toml("dokument.toml"), pfad: p => path(p))

= Background

The poster starts here. Title, authors, size, columns and colour are in the “Document” pane of
the sidebar. Each section starts with `=` and gets a coloured band.

= Method

Text flows from column to column. Add pictures with `#image("picture.png", width: 100%)`.

#kasten(titel: [At a glance:])[The main point in one sentence.]

= Results

Citations come from `literatur.bib`, for example @beispielPlakat2026.

= Conclusion

Short and large enough to read from two metres away.
