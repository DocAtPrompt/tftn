#import "@local/tftn-fachartikel:0.2.0": *

#show: fachartikel.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
  pfad: p => path(p),
)

// Beispielprojekt für TFTN. Alles hier ist erfunden; ändern und ausprobieren ist ausdrücklich erwünscht.

= Worum es geht <einleitung>

TFTN ist ein Schreibwerkzeug für Typst. Links steht der Text, rechts die fertige Seite. Ein Klick
in die Vorschau springt an die passende Stelle im Text, und umgekehrt folgt die Vorschau der
Schreibmarke. Gute Typografie soll dabei nebenbei entstehen @beispielSchreiben2024[S. 12].

Zum Ausprobieren:
- Mit *fett* und _kursiv_ heben Sie Wörter hervor (Cmd/Strg+B, Cmd/Strg+I).
- Ein „/“ in einer leeren Zeile öffnet die Bausteine: Abbildung, Tabelle, Formel und mehr.
- Cmd/Strg+Umschalt+L fügt ein Zitat aus der Literaturdatei ein.
- Cmd/Strg+Umschalt+Eingabe schaltet die Schreibansicht ohne Ablenkung ein.

= Methode <methode>

== Formeln

Formeln stehen im Satz wie $a^2 + b^2 = c^2$ oder abgesetzt und nummeriert:

$ overline(x) = 1/n sum_(i=1)^n x_i $ <mittelwert>

@mittelwert lässt sich im Fach „Formel“ der Seitenleiste auch mit Knöpfen bearbeiten.

== Ablauf

@ablauf zeigt den Weg vom Text zum fertigen Dokument.#footnote[Der Export nach Word läuft über
Pandoc, das dafür installiert sein muss.]

#figure(
  image("abbildungen/ablauf.svg", width: 85%),
  caption: [Vom Text über die Vorschau zum PDF oder Word-Dokument.],
) <ablauf>

= Ergebnisse <ergebnisse>

Die Werte in @werte sind erfunden. Klicken Sie in eine Zelle und öffnen Sie die Seitenleiste
(Cmd/Strg+Umschalt+P): Dort lassen sich Zeilen und Spalten einfügen, verschieben und löschen.

#figure(
  table(
    columns: 3,
    align: (left, right, right),
    table.header[*Gruppe*][*n*][*Mittelwert*],
    [A], [24], [3,1],
    [B], [31], [2,7],
    [C], [18], [3,4],
  ),
  caption: [Erfundene Messwerte je Gruppe.],
) <werte>

= Diskussion <diskussion>

Eine sofortige Vorschau verkürzt den Weg zwischen Schreiben und Prüfen
#cite(<musterVorschau2025>, form: "prose"). Wer tiefer einsteigen will, findet eine Einführung bei
@probeTypst2026. Ganze Abschnitte, etwa @einleitung bis @ergebnisse, lassen sich in der
Gliederung verschieben.

#hinweis[Titel, Autorinnen und Autoren, Datum, Zusammenfassung und Literatur stehen im Fach
  „Dokument“ der Seitenleiste.]
