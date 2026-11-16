# TFTN – Handbuch

*English version: [manual.md](manual.md)*

TFTN („Typst for the Noobs“) ist ein Schreibprogramm für [Typst](https://typst.app) auf macOS und
Windows. Links schreiben Sie, rechts steht sofort die fertige Seite. TFTN arbeitet ganz ohne Netz;
nur wenn Sie ausdrücklich etwas holen (ein Paket, einen Literatureintrag, die Update-Abfrage), geht
eine Anfrage an eine feste Adresse hinaus.

Ihr Text bleibt gewöhnliches Typst. Jedes Dokument lässt sich auch ohne TFTN mit der typst-CLI
übersetzen und ergibt dasselbe PDF.

## Inhalt

1. [Erste Schritte](#1-erste-schritte)
2. [Die Oberfläche](#2-die-oberfläche)
3. [Schreiben](#3-schreiben)
4. [Werte ändern ohne Code](#4-werte-ändern-ohne-code)
5. [Seite, Titelei und Literatur](#5-seite-titelei-und-literatur)
6. [Pakete aus Typst Universe](#6-pakete-aus-typst-universe)
7. [Importieren und Exportieren](#7-importieren-und-exportieren)
8. [Sichern und Fassungen](#8-sichern-und-fassungen)
9. [Rechtschreibung, Zählen, Fehlermeldungen](#9-rechtschreibung-zählen-fehlermeldungen)
10. [Einstellungen und Tastenkürzel](#10-einstellungen-und-tastenkürzel)
11. [Netz, Updates, Datenschutz](#11-netz-updates-datenschutz)
12. [Kommandozeile](#12-kommandozeile)
13. [KI-Assistenten (MCP)](#13-ki-assistenten-mcp)

## 1. Erste Schritte

**Installieren (macOS):** `TFTN.app` aus der Release-Seite nach `/Programme` ziehen. Die App ist
nicht notarisiert: Beim ersten Start Rechtsklick → „Öffnen“. Die passende typst-CLI liegt bei.

**Windows:** `TFTN-<version>-windows-x64-setup.exe` von der Release-Seite ausführen; er installiert
für den aktuellen Benutzer, ohne Administratorrechte. Der Installer ist nicht signiert: Warnt
SmartScreen, „Weitere Informationen“ → „Trotzdem ausführen“.

**Ausprobieren:** Mit „Öffnen“ den Ordner `demo/` aus dem Quelltext wählen. Der Beispielartikel
zeigt Formeln, eine Abbildung, eine Tabelle, Fußnoten, Querverweise und Zitate. Ändern ist
ausdrücklich erwünscht.

**Ein neues Dokument:** TFTN startet mit einem leeren Entwurf. Einfach losschreiben; Cmd/Strg+S gibt
ihm einen Namen und legt es im Ordner „Dokumente“ ab. „Neu“ → „Aus Vorlage …“ legt stattdessen ein
Dokument eines Dokumenttyps an. Mitgeliefert sind: leeres Dokument, Brief, Rechnung, Befund/Arztbrief, Bericht,
Fachartikel, Fachbuch, Plakat und Lebenslauf, jeweils auf Deutsch und Englisch.

**Sprache eines neuen Dokuments:** Neue Dokumente und der leere Entwurf folgen der Sprache der
Oberfläche – mit englischer Oberfläche entsteht eine englische Vorlage (Silbentrennung,
Rechtschreibung und Beschriftungen wie „Abstract“, „Keywords“ und „References“). Bei jedem
Dokumenttyp mit Formular lässt sich das später umstellen: Fach „Dokument“ → „Sprache“.

**Befunde** enthalten Gesundheitsdaten. Solche Dokumente gehören in keinen Git-Ordner; liegt der
Dokumentordner in einem, warnt das Fach „Dokument“. Alles bleibt auf dem eigenen Rechner – TFTN
schickt nie Text irgendwohin.

**Rechnungen** rechnen Posten, Umsatzsteuer und Gesamtbetrag selbst (je Zeile auf Cent gerundet).
Die Rechnungsnummer vergibst du selbst, fortlaufend und ohne Lücken; TFTN zählt nicht mit, auch
nicht über mehrere Rechner. Ob die Rechnung alle Angaben enthält, die für dich vorgeschrieben sind,
prüfst du selbst.

**Was ist ein Projekt?** Ein Ordner mit `tftn.toml`, der Hauptdatei (meist `main.typ`), bei
Vorlagen dazu `dokument.toml` (Titelei), der Literaturdatei, den Schriften (`fonts/`) und den
benutzten Paketen (`packages/`). Alles, was das Dokument braucht, liegt im Ordner – er lässt sich
kopieren, synchronisieren und auf einem anderen Rechner ohne Netz weiterbearbeiten.

## 2. Die Oberfläche

- **Kopf:** links das Projekt (der volle Pfad erscheint beim Darüberfahren), rechts „Neu“ (leeres
  Dokument, aus Vorlage, Importieren), „Öffnen“, „Fassungen“, „Export“ und das Zahnrad
  (Einstellungen). In einem schmalen Fenster bleiben nur die Symbole.
- **Hinweise** erscheinen in einem Band unter dem Kopf. Kurze Meldungen verschwinden von selbst;
  Fehler und Fragen bleiben, bis man sie mit × schließt oder beantwortet.
- **Fenster wie Export, Neu oder Fassungen** schließen mit ×, Esc oder einem Klick daneben. Hat man
  in einem Dialog schon etwas eingetippt (etwa einen Projektnamen), schließt ihn nur × oder Esc.
- **Editor** (links): Typst-Quelltext mit Hervorhebung. Darüber die Knöpfe „Baustein“, „Zitat“,
  „Gliederung“, „Seite“ und als Symbole Seitenleiste und Schreibansicht.
- **Vorschau** (rechts): die fertigen Seiten. Ein Klick in die Vorschau springt an die Stelle im Text;
  die Vorschau folgt der Schreibmarke. Rechtsklick öffnet ein Menü mit „Ändern …“ und „Im Code zeigen“.
  Rechts in der Statuszeile: „Breite“ (Seiten so breit wie die Spalte), „Ganze Seite“, − und + sowie
  die Prozentzahl (ein Klick darauf zeigt die echte Größe). Cmd/Strg + Mausrad oder zwei Finger auf
  dem Trackpad zoomen um den Mauszeiger; ist die Seite breiter als die Spalte, gibt es eine waagrechte
  Laufleiste.
- **Seitenleiste** (Cmd/Strg+Umschalt+P): Fächer „Hier“, „Einfügen“, „Formel“, „Seite“, „Dokument“,
  „Pakete“. Ihre Breite lässt sich ziehen.
- **Gliederung** (Knopf „Gliederung“): die Überschriften neben dem Text, im zweiten Reiter die
  Kommentare.
- **Statuszeile:** Wörter und Zeichen, Dokumentsprache, Speicherstand, Netzzugriffe.

## 3. Schreiben

**Bausteine:** In einer leeren Zeile „/“ tippen öffnet die Bausteinliste (weitertippen filtert,
Enter fügt ein, Esc schließt). Cmd/Strg+Umschalt+B oder „Baustein“ öffnen sie mit Suchfeld; markierter
Text wandert in den Baustein. Danach springt Tab von Feld zu Feld, Umschalt+Tab zurück, Esc beendet.
Cmd/Strg+Z nimmt den ganzen Baustein in einem Schritt zurück.

**Vervollständigen:** Strg+Leertaste (am Mac auch ⌥`) schlägt Funktionen, Parameter, Marken und
Zitat-Schlüssel vor. Beim bloßen Tippen erscheint nichts.

**Auszeichnen:** Cmd/Strg+B fett, Cmd/Strg+I kursiv, Cmd/Strg+K Link. Cmd/Strg+Umschalt+. bzw. +,
setzt eine Überschrift eine Ebene tiefer bzw. höher. Fußnote, Abbildung und Formel:
Cmd/Strg+Umschalt+F, +A, +E.

**Formeln:** Das Fach „Formel“ (Cmd/Strg+Umschalt+X) hat ein Eingabefeld mit Vorschau und eine
Palette (Bruch, Wurzel, Summe, Matrix, Fälle, griechische Buchstaben, Symbole). Tab springt zum
nächsten Platzhalter. Steht die Schreibmarke in einer Formel, ändert „Übernehmen“ (Cmd/Strg+Eingabe)
sie in einem Schritt; sonst fügt „Einfügen“ eine neue ein.

**Bilder:** Ein Bild aus Finder bzw. Explorer in den Text ziehen oder aus der Zwischenablage einfügen
(etwa ein Bildschirmfoto). TFTN legt es unter `bilder/` ab und setzt eine Abbildung mit 80 % Breite
ein; die Schreibmarke steht in der Unterschrift. Formate: png, jpg, gif, svg, webp, pdf (bis 50 MB).

**Zitieren:** Cmd/Strg+Umschalt+L oder „Zitat“ durchsucht die Literaturdatei. Ein
vorhandenes Zitat ändern: Schreibmarke ins Zitat und die Seitenleiste öffnen (oder Rechtsklick in der
Vorschau → „Ändern …“). Dort lassen sich Seitenangabe und Quelle wechseln.

**Literatur aus einer DOI:** Im Zitat-Dialog eine DOI eingeben (`10.1038/nature14539`, `doi:…` oder
`https://doi.org/…`). „DOI nachschlagen und zitieren“ holt den Eintrag bei Crossref bzw. DataCite,
schreibt ihn mit lesbarem Schlüssel (`lecun2015`) in die Literaturdatei und fügt das Zitat ein. Fehlt
noch ein Literaturverzeichnis, kommt es ans Ende. Übertragen wird nur die DOI.

**Literatur verwalten:** „Literatur verwalten …“ im Zitat-Dialog oder im Fach „Dokument“ zeigt die
Einträge der Literaturdatei: links die Liste mit Suche (nicht zitierte Einträge sind vermerkt), rechts
das Formular mit den üblichen Feldern je Art (Zeitschriftenartikel, Buch, Sammelband, …). Personen
stehen eine je Zeile („Nachname, Vorname“). Bei einem neuen Eintrag schlägt TFTN den Schlüssel aus
erster Person und Jahr vor. „Speichern“ (Cmd/Strg+S) schreibt nur diesen Eintrag in die `.bib`; der
Rest der Datei bleibt, wie er ist. „Zitieren“ fügt `@schlüssel` an der Schreibmarke ein. Werte ohne
Klammern wie `2015` oder Abkürzungen aus `@string` stehen in Schreibmaschinenschrift und werden
unverändert geschrieben. YAML-Literatur (Hayagriva) lässt sich hier nicht bearbeiten.

**Gliederung:** Links stehen die Überschriften; ein Klick springt hin. Ziehen verschiebt einen
Abschnitt samt Unterabschnitten, ebenso Alt+↑/↓ auf einem Eintrag. Jede Verschiebung ist ein
Rückgängig-Schritt.

**Kommentare:** Was nach `//` steht (oder zwischen `/*` und `*/`), ist eine Notiz, die nicht gedruckt
wird. Der Reiter „Kommentare“ in der Gliederung listet sie nach Abschnitten; ein Klick springt hin.
Mehrere `//`-Zeilen untereinander gelten als ein Kommentar.

**Schreibansicht ohne Ablenkung** (Knopf „Schreibansicht“ oder Cmd/Strg+Umschalt+Eingabe): nur der
Text, mittig und etwas größer. Alles andere tritt zurück; die Vorschau rechnet im Hintergrund weiter.
Esc, dasselbe Kürzel oder der blasse Knopf oben rechts beenden sie. Für den ganzen Bildschirm die
Vollbildfunktion des Systems nutzen.

## 4. Werte ändern ohne Code

Was im Text als Wert steht – eine Breite, eine Farbe, ein Datum, eine Ausrichtung –, lässt sich ohne
Typst-Kenntnisse ändern. Jede Änderung ist geprüft und ein Rückgängig-Schritt.

- **Marken:** ⌥ bzw. Alt allein eine halbe Sekunde halten (oder Cmd/Strg+Umschalt+M). Hinter jedem
  einstellbaren Wert erscheint eine kleine Marke, „+ …“ dort, wo ein Wert fehlt. Ein Klick öffnet den
  passenden Editor (Farbwähler, Zahl mit Einheit, Auswahl …).
- **Seitenleiste, Fach „Hier“:** zeigt, worin die Schreibmarke steht, mit allen Werten als Formular,
  Knöpfen für Überschrift, Liste und Formel und – in einer Tabelle – dem **Tabellenraster**: Zeilen und
  Spalten einfügen, löschen, verschieben, Zellen verbinden und trennen; Doppelklick ändert den Text.
- **Rechtsklick in der Vorschau:** „Ändern …“, die Werte direkt, „Im Code zeigen“.

## 5. Seite, Titelei und Literatur

**Seite** (Knopf „Seite“): Format, Querformat, doppelseitig, Spalten, Ränder, Schrift, Größe,
Zeilenabstand, Blocksatz, nummerierte Überschriften, Sprache, Seitenzahl, Kopf- und Fußzeile (mit
`{seite}` und `{kapitel}`). Jede Einstellung steht als `#set`-Zeile oben im Text. Vorlagen wie der
Fachartikel legen das selbst fest. Mit „Schriften einbinden“ kommen die benutzten Schriften ins Projekt.

**Titelei** (Fach „Dokument“, Cmd/Strg+Umschalt+O): ein Formular aus dem Dokumenttyp – Titel,
Autorinnen und Autoren, Datum (`31.12.2026`), Zusammenfassung, Schlagwörter, Literaturdateien und
Zitierstil (eingebauter Name wie `ieee` oder eine `.csl`-Datei im Projekt). Jeder Wert wird geprüft und
erst dann in `dokument.toml` geschrieben; der Text in `main.typ` bleibt unberührt. Ein Klick auf die
Titelei in der Vorschau öffnet das Fach am passenden Feld. „Als Text bearbeiten“ zeigt
`dokument.toml` als Text, etwa für Felder, die das Formular nicht kennt, oder um eine fehlerhafte Datei
zu reparieren: „Übernehmen“ (Cmd/Strg+Eingabe) schreibt nur gültiges TOML, „Rückgängig“ nimmt es
zurück.

**Dokumenttyp aktualisieren:** Gibt es eine neuere Version des Dokumenttyps, zeigt sie das Fach
„Dokument“. „Vergleichen …“ zeigt die geänderten Seiten vorher und nachher, „Übernehmen“ stellt um.

**Eigene Dokumenttypen** liegen im Ordner `typen/` des Einstellungsordners und haben Vorrang vor den
mitgelieferten. Ein Dokumenttyp ist ein Typst-Paket mit einem Abschnitt `[tool.tftn]` in `typst.toml`.

**Eigenen Dokumenttyp anlegen**, etwa einen Brief mit dem eigenen Absender: Einstellungen → „Eigener
Dokumenttyp aus“ einen vorhandenen Typ wählen, einen Namen eingeben, „Anlegen“. TFTN kopiert ihn nach
`typen/<name>/` und zeigt den Ordner. Darin stehen die Vorgaben jedes neuen Dokuments in
`vorlage/dokument.toml` (englische Vorlage: `vorlage-en/`) und das Layout in `lib.typ`. Der neue Typ
erscheint unter „Neu“ → „Aus Vorlage …“; Dokumente, die schon bestehen, bleiben, wie sie sind. Die
Kopie bringt ihre Schriften mit und lässt sich mit dem Einstellungsordner auf andere Rechner
mitnehmen.

## 6. Pakete aus Typst Universe

Das Fach „Pakete“ (Cmd/Strg+Umschalt+Y) sucht im Paketindex nach Name, Beschreibung und Schlagwort.
„Hinzufügen“ holt das Paket samt seinen Abhängigkeiten, legt es unter `packages/preview/` ins Projekt,
hält seine Prüfsumme in `tftn.toml` fest und setzt auf Wunsch den `#import`. Danach baut das Dokument
ohne Netz, auch auf einem anderen Rechner. „Fehlt im Projekt“ listet Pakete, die ein abgetippter Import
verlangt. „Entfernen“ und „Neuere Version …“ (mit Vergleich) stehen am Paket.

**Pakete mit Helfern:** Für cetz (Zeichnungen), fletcher (Ablaufdiagramme) und cetz-plot (Diagramme
aus Daten) bringt TFTN eigene Bausteine mit; ihre Punkte, Farben und Datenreihen lassen sich über
Marken ändern. Eigene Paketbeschreibungen (TOML) im Ordner `beschreibungen/` des Einstellungsordners
wirken sofort.

## 7. Importieren und Exportieren

**Importieren** („Neu“ → „Importieren …“, braucht [Pandoc](https://pandoc.org) ab 3.1.2): Word (.docx),
OpenDocument (.odt), Markdown, HTML, EPUB, LaTeX und RTF werden zu einem neuen Dokument. Überschriften,
Listen, Tabellen, Fußnoten, Formeln und Bilder kommen mit; Layout, Kommentare und Änderungsverfolgung
nicht. Das Original bleibt unberührt. Pandoc läuft abgeschottet und holt nichts aus dem Netz.
Installieren: Mac `brew install pandoc`, Windows `winget install --id JohnMacFarlane.Pandoc`.

**Einfügen mit Umwandlung** (braucht ebenfalls Pandoc): Fügt man mit Cmd/Strg+V Markdown ein (etwa
`# Titel`, `**fett**`, `[Link](…)`, Tabellen) oder formatierten Text aus dem Browser oder aus Word,
steht er zuerst unverändert da, und das Hinweisband bietet „In Typst umwandeln“ an. Ein Klick ersetzt
das Eingefügte durch Typst; ein Rückgängig holt den eingefügten Text zurück. Tippt man weiter, gilt
das Angebot nicht mehr; schließt man das Band, zeigt die Vorschau die Fehler im eingefügten Text.
Später geht es auch so: Markdown im Editor markieren, Rechtsklick in die Auswahl, „In Typst
umwandeln“. Bilder aus dem Netz werden nicht geholt.

**PDF** („Export“): Die Profile kommen vom Dokumenttyp, beim Fachartikel etwa „vorschau“ und
„archiv“ (PDF/A-2b). Das PDF entsteht mit der beigelegten typst-CLI und ist reproduzierbar.

**Word (.docx) zum Gegenlesen** („Export“, braucht Pandoc): Titel, Überschriften, Listen, Tabellen,
Abbildungen, echte Fußnoten, Formeln, Zitate und Literaturverzeichnis kommen mit, das Seitenlayout
nicht. Die Datei liegt als `<Dokumentname>.docx` im Dokumentordner und wird beim nächsten Export
überschrieben.

## 8. Sichern und Fassungen

Jedes Dokument sichert sich eine Sekunde nach der letzten Änderung von selbst, auch beim Schließen und
Beenden. Cmd/Strg+S braucht man nur, um einem neuen Dokument einen Namen zu geben.

**Fassungen** zeigt frühere Stände: Höchstens alle zehn Minuten hält TFTN eine Fassung in
`.fassungen/` im Dokumentordner fest (vom letzten Tag alle, davor eine je Tag, 60 Tage lang).
Zurückholen ersetzt den Text in einem Schritt; Cmd/Strg+Z bringt den vorigen zurück.

## 9. Rechtschreibung, Zählen, Fehlermeldungen

**Rechtschreibung** nach Dokumentsprache: Unbekannte Wörter im Fließtext sind rot unterwellt, Befehle,
Formeln, Code und Verweise nie. Rechtsklick: Vorschläge, „Ins Wörterbuch aufnehmen“ (in `woerter.txt`
im Einstellungsordner) oder „Ignorieren“. Wörterbücher: Deutsch (Deutschland, Österreich, Schweiz),
Englisch (USA, Großbritannien), Französisch, Italienisch, Spanisch.

**Silbentrennung:** Typst trennt am Zeilenende selbst, nach den Regeln der Dokumentsprache. Trennt es
ein Wort falsch (Fachbegriffe, Namen, „Urin-stinkt“), trägt man es in `trennungen.txt` im
Projektordner ein – Fachartikel, Fachbuch und Bericht haben diese Datei. Ein Wort je Zeile, die
erlaubten Trennstellen mit `-` markiert; ein Wort ohne `-` wird nie getrennt. Zeilen mit `#` sind
Kommentare.

```
Ur-instinkt
Sozial-versicherungs-träger
Typst
```

Das Wort wird auch in längeren Wörtern gefunden (Zusammensetzungen, Beugungsformen);
Groß- und Kleinschreibung zählt. Die Änderung wirkt sofort in der Vorschau.

**Zählen:** Die Statuszeile zeigt Wörter und Zeichen, für eine Auswahl „Auswahl: … von …“, die
Gliederung je Abschnitt. Gezählt wird der geschriebene Text samt Unterschriften und Fußnoten.

**Fehlermeldungen** stehen am Fehler im Text und oben in der Vorschau, mit einem Tipp zur Behebung.
Solange ein Fehler besteht, zeigt die Vorschau den letzten fehlerfreien Stand, blass und mit roten
Schrägstreifen hinterlegt.

## 10. Einstellungen und Tastenkürzel

Das Zahnrad (Einstellungen, Cmd/Strg+,) zeigt alle Aktionen mit ihren Kürzeln: Zeile anklicken, neues Kürzel
drücken; Entf heißt „keine Belegung“, „Standard“ setzt zurück. Konflikte mit System- und
Programmkürzeln werden für Mac **und** Windows angezeigt.

| Aktion | Standard (Mod = Cmd bzw. Strg) |
|---|---|
| Baustein einfügen | Mod+Umschalt+B, in einer Leerzeile auch „/“ |
| Zitat einfügen | Mod+Umschalt+L |
| Vervollständigen | Strg+Leertaste, am Mac auch ⌥` |
| Einstellungen | Mod+, |
| Werte an dieser Stelle (Marken) | Mod+Umschalt+M oder ⌥ bzw. Alt halten |
| Seitenleiste | Mod+Umschalt+P |
| Seitenleiste: Dokument / Pakete / Formel | Mod+Umschalt+O / +Y / +X |
| Schreibansicht ohne Ablenkung | Mod+Umschalt+Eingabe |
| Speichern, Neu, Öffnen | Mod+S, Mod+N, Mod+O |
| Fett, Kursiv, Link | Mod+B, Mod+I, Mod+K |
| Überschrift tiefer / höher | Mod+Umschalt+. / Mod+Umschalt+, |
| Fußnote, Abbildung, Formel | Mod+Umschalt+F, +A, +E |

**Ordner für eigene Anpassungen:** Kürzel (`tasten.toml`), eigenes Wörterbuch, eigene Dokumenttypen
(`typen/`), Skins (`skins/`), Sprachdateien (`sprachen/`) und Paketbeschreibungen liegen in einem
Ordner, unten in den Einstellungen; „Im Ordner zeigen“ öffnet ihn. Ohne Wahl ist es ein Ordner dieses
Rechners. Wer mehrere Rechner nutzt, kann einen anderen Ordner wählen, etwa in einem Sync-Ordner;
dann haben alle Rechner dieselben Anpassungen. Änderungen von außen wirken sofort.

**Sprache der Oberfläche:** Deutsch und Englisch sind dabei; „Wie das System“ ist der Standard.
„Weitere Sprache …“ legt eine Vorlage zum Übersetzen an: links Deutsch, rechts die Übersetzung, als
`<kennung>.toml` (etwa `fr.toml`) im Ordner `sprachen/` speichern – ohne neues Programm.

**Aussehen (Skins):** Dabei sind Hell, Dunkel, Papier und Hoher Kontrast; „Wie das System“ wechselt
mit dem System zwischen hell und dunkel. Ein Skin legt die Farben der Oberfläche, die Farben der
Syntaxhervorhebung und Schrift, Größe und Zeilenabstand des Editors fest; die Vorschau bleibt weiß wie
das gedruckte Dokument. „Eigener Skin …“ legt im Ordner `skins/` eine Vorlage an; nach dem Ändern
wählt man sie unter „Aussehen“. Werte, die nicht passen, nennt der Dialog und überspringt sie. Die
Wahl gilt je Rechner.

**Größe der Oberfläche:** Klein, Normal, Groß oder Sehr groß vergrößert Leisten, Knöpfe und Dialoge
sofort. Editor und Vorschau bleiben, wie sie sind: Die Editorschrift legt der Skin fest, die
Vorschau passt sich ihrer Breite an. Die Wahl gilt je Rechner.

## 11. Netz, Updates, Datenschutz

TFTN liest und schreibt nur Ihre Dateien. Ins Netz geht nur das beigelegte Hilfsprogramm
`tftn-holer`, nur auf Klick und nur zu festen Adressen:

| Wofür | Adresse |
|---|---|
| Pakete | `packages.typst.org` |
| Literatur aus DOI | `api.crossref.org`, `api.datacite.org` (nur die DOI) |
| Nach Updates suchen | `api.github.com` (neueste Version von TFTN und typst) |

Solange eine Anfrage läuft, steht sie in der Statuszeile. **„Ganz offline“** in den Einstellungen
sperrt jeden Netzzugriff. TFTN installiert nie etwas von selbst: Eine neue Version laden Sie von der
Release-Seite.

Mit **MCP** (Abschnitt 13) liest ein KI-Assistent das offene Dokument. TFTN selbst geht dabei nicht ins
Netz. Wohin das Gelesene geht, hängt am Assistenten: Ein lokales Modell (etwa in LM Studio) behält alles
auf dem Rechner, ein Cloud-Assistent schickt es an seinen Anbieter. MCP bleibt aus, bis Sie es
einschalten.

## 12. Kommandozeile

Das Programm `tftn` (aus dem Quelltext gebaut) arbeitet ohne Oberfläche:

| Befehl | Wirkung |
|---|---|
| `tftn neu --typ tftn-fachartikel <ordner> [--sprache en]` | Projekt aus einem Dokumenttyp anlegen (`--sprache en`: englische Vorlage) |
| `tftn check <ordner>` | prüfen, Fehler und fehlende Pakete melden |
| `tftn build <ordner> [-o ziel]` | PDF über die typst-CLI (nach `_renders/`) |
| `tftn build <ordner> --png [--ppi 144]` | Seitenbilder statt PDF |
| `tftn export <ordner> [--profil archiv] [--abnehmen]` | Export-Profil des Dokumenttyps |
| `tftn gegenprobe <ordner>` | die gleichwertige `typst compile`-Zeile ausgeben |
| `tftn katalog <ordner>` | Bausteine des Projekts auflisten |
| `tftn typ-pruefen <ordner>` | alle Bausteine eines Dokumenttyps prüfen |

`tftn gegenprobe` zeigt, wie dasselbe PDF ohne TFTN entsteht – Ihr Dokument hängt nie an TFTN.

## 13. KI-Assistenten (MCP)

Über MCP (Model Context Protocol) kann ein KI-Assistent im offenen Dokument arbeiten – ein lokales Modell ebenso wie ein
Cloud-Dienst wie Claude: den
Quelltext lesen, Stellen ändern, das Formular im Fach „Dokument“ ausfüllen, Seiten der Vorschau ansehen, ein neues Dokument aus einem Typ anlegen
oder ein anderes öffnen. Jede Änderung erscheint sofort im Editor und in der Vorschau und lässt sich
mit Rückgängig zurücknehmen.

**Einschalten:** In den Einstellungen „MCP: KI-Assistenten … dürfen im offenen Dokument lesen und
schreiben“ anhaken. Solange MCP an ist, steht „MCP an“ in der Statusleiste; ein Klick darauf schaltet
es aus. Greift ein Assistent zu, pulsiert der Punkt davor, und für ein paar Sekunden steht da, was er
tut (etwa „MCP: Text geändert“); der Tooltip nennt den letzten Zugriff mit Uhrzeit. Auch in der
Schreibansicht bleibt der Hinweis sichtbar. Die Wahl gilt je Rechner und bleibt über einen Neustart erhalten.

**Im Assistenten eintragen:** Darunter steht die Zeile für Claude Code
(`claude mcp add tftn -- "<Pfad zu TFTN>" --mcp`); „Kopieren“ und im Terminal ausführen. Andere
Assistenten, auch solche mit lokalem Modell, tragen als Befehl das TFTN-Programm mit dem Argument
`--mcp` ein; die übliche `mcpServers`-Angabe (Claude Desktop, LM Studio u. a.) lautet etwa:

```json
{ "mcpServers": { "tftn": { "command": "/Applications/TFTN.app/Contents/MacOS/tftn-app", "args": ["--mcp"] } } }
```

Der Assistent spricht mit der laufenden App über eine Verbindung, die nur auf diesem Rechner besteht
und einen Zufallsschlüssel braucht. Ist TFTN geschlossen oder MCP aus, bekommt er eine Meldung.

**Beim Dokumentwechsel ausschalten:** Wer möchte, dass MCP nur für ein Dokument gilt, hakt „MCP
ausschalten, wenn ein anderes Dokument geöffnet wird“ an. Öffnen Sie dann selbst ein anderes Dokument,
geht MCP aus. Öffnet oder legt der Assistent eines an, pausiert MCP und TFTN fragt, ob es aktiv
bleiben soll („Aktiv lassen“ oder „MCP ausschalten“); bis dahin lehnt TFTN seine Aufrufe ab.

| Werkzeug | Wirkung |
|---|---|
| `status` | Projekt, Revision, gespeichert?, Ergebnis des letzten Typst-Laufs |
| `quelltext_lesen` | Quelltext der Hauptdatei, wie er im Editor steht |
| `text_ersetzen` | eine Stelle wörtlich ersetzen (oder jedes Vorkommen) |
| `quelltext_setzen` | den ganzen Quelltext ersetzen |
| `seite_ansehen` | eine Seite der Vorschau als Bild |
| `formular_lesen` | Felder und Werte des Formulars (Titel, Autoren, Datum …) |
| `formular_setzen` | mehrere Felder auf einmal setzen, geprüft wie im Fach; ein Rückgängig-Schritt |
| `typen_auflisten` | Dokumenttypen für ein neues Dokument |
| `dokument_anlegen` | neues Projekt aus einem Typ anlegen und öffnen |
| `dokument_oeffnen` | ein anderes Projekt öffnen (das offene wird vorher gespeichert) |
