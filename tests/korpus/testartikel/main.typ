// Synthetischer Testartikel für TFTN (PLAN-M1 0.5), Dokumenttyp tftn-fachartikel (AP2).
// Alle Personen, Daten und Quellen sind erfunden. Keine Patientendaten.
// Titel, Autoren, Datum, Zusammenfassung und Schlagwörter stehen in dokument.toml.

#import "@local/tftn-fachartikel:0.1.0": *

#show: fachartikel.with(
  daten: toml("dokument.toml"),
  trennliste: read("trennungen.txt"),
)

= Einleitung <einleitung>

Die Dokumentation in der hausärztlichen Praxis beansprucht einen großen Teil der Arbeitszeit
@haeuslerDokumentationsaufwand2018[S. 87]. Häusler schätzte den Anteil auf ein Viertel bis ein
Drittel; ähnliche Größenordnungen berichten #cite(<musterfrauReproduzierbarkeitAerztlicherBefunde2021>, form: "prose").
Dabei geht es nicht nur um den Inhalt, sondern auch um die Form: Briefköpfe, Schriftgrößen und
Seitenumbrüche müssen stimmen, bevor ein Befund das Haus verlässt.#footnote[Für Überweisungen gilt
zusätzlich, dass Name, Geburtsdatum und Sozialversicherungsnummer auf jeder Seite stehen sollen. Diese
Anforderung ist hier nur als Beispiel genannt.]

Typografische Grundregeln -- Zeilenlänge, Durchschuss, Silbentrennung -- sind seit Langem bekannt
@grosseSatzUndSchrift2019[S. 12--15]. Fachautorinnen und Fachautoren wenden sie aber selten bewusst
an. @methode beschreibt deshalb ein Verfahren, das die Gestaltung von der Schreibarbeit
trennt.

= Methode <methode>

== Datengrundlage

Grundlage sind 1.248 erfundene Befundberichte aus drei Jahrgängen. Jeder Bericht besteht aus einer
Vorlage, einer Datendatei und einer gemeinsamen Literaturdatei. @tab-jahrgaenge zeigt die
Verteilung.

#hinweis(farbe: rgb("#e8f1f2"), art: "merke")[Alle Befunde sind erfunden; Rückschlüsse auf echte
Praxen sind nicht möglich.]

#figure(
  table(
    columns: (auto, 1fr, 1fr, 1fr),
    align: (left, right, right, right),
    stroke: (x, y) => if y == 0 { (bottom: 0.7pt) } else { none },
    table.header[*Jahrgang*][*Befunde*][*Seiten (Ø)*][*Zitate (Ø)*],
    [2023], [402], [2,1], [0,4],
    [2024], [417], [2,3], [0,6],
    [2025], [429], [2,4], [0,9],
    table.hline(stroke: 0.4pt),
    [Summe/Ø], [1.248], [2,3], [0,6],
  ),
  caption: [Verteilung der erfundenen Befunde nach Jahrgang.],
) <tab-jahrgaenge>

== Messgröße

Als Aufwand $A$ gilt die Zeit vom ersten Tastendruck bis zum fertigen PDF. Für $n$ Befunde mit den
Einzelzeiten $t_i$ ergibt sich der Mittelwert

$ overline(t) = 1/n sum_(i=1)^n t_i $ <gl-mittel>

und die relative Ersparnis zwischen altem und neuem Verfahren

$ Delta = (overline(t)_"alt" - overline(t)_"neu") / overline(t)_"alt" dot 100 thin % . $ <gl-ersparnis>

@gl-ersparnis setzt voraus, dass beide Verfahren dieselben Befunde erzeugen. Zur
Kontrolle wird jedes PDF zweimal erzeugt und byteweise verglichen, wie es für Klartextwerkzeuge
vorgeschlagen wurde @smithTypesettingClinicalReports2023.

== Zitierweise

Zitate folgen den Konventionen der Fachgesellschaft
@fachgesellschaftLeitfadenBefundberichte2024 und einer Übersicht von Lüddecke
@lueddeckeZitierenInDerMedizin2020[S. 52]. Seitenangaben stehen im Zitat, nicht im
Literaturverzeichnis.#footnote[Mehrere Quellen an einer Stelle: @grosseSatzUndSchrift2019
@lueddeckeZitierenInDerMedizin2020.]

= Ergebnisse <ergebnisse>

Der mittlere Aufwand nach @gl-mittel sank von 14,2 auf 9,8 Minuten je Befund. Nach
@gl-ersparnis entspricht das einer Ersparnis von

$ Delta = ("14,2" - "9,8") / "14,2" dot 100 thin % approx 31 thin % . $

Alle 1.248 PDFs waren bei der zweiten Erzeugung bytegleich. Abweichungen traten nur auf, wenn
eine Schrift aus dem Betriebssystem statt aus dem Projektordner geladen wurde; das betraf 17 Fälle
auf einem Rechner mit älterer Schriftversion.

Größere Änderungen am Erscheinungsbild -- etwa eine neue Hausschrift -- ließen sich in einer
einzigen Zeile der Vorlage vornehmen. Ein typischer Eintrag sieht so aus:

```typ
#set text(font: "Libertinus Serif", size: 11pt, lang: "de", region: "AT")
```

= Diskussion <diskussion>

Die Ergebnisse stützen die Annahme, dass Trennung von Inhalt und Form den Aufwand senkt. Einschränkend
ist zu sagen, dass alle Daten erfunden sind und keine Aussage über echte Praxen erlauben. Wörter mit
Umlauten und Sonderzeichen -- Größenänderung, Überweisungsschein, Öffnungszeiten, Füße, Maß -- sowie
lange Zusammensetzungen wie Sozialversicherungsträgerdokumentationspflicht dienen hier vor allem als
Prüfstein für die Silbentrennung.

Offen bleibt, wie sich das Verfahren auf längere Texte wie Fachbücher übertragen lässt. Dort kommen
Register, Querverweise über Kapitel und eine Druckvorstufe hinzu @grosseSatzUndSchrift2019[Kap. 9].

#bibliography("literatur.bib")
