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

## 1. Erste Schritte

**Installieren (macOS):** `TFTN.app` aus der Release-Seite nach `/Programme` ziehen. Die App ist
nicht notarisiert: Beim ersten Start Rechtsklick → „Öffnen“. Die passende typst-CLI liegt bei.

**Windows:** `TFTN-<version>-windows-x64-setup.exe` von der Release-Seite ausführen; er installiert
für den aktuellen Benutzer, ohne Administratorrechte. Der Installer ist nicht signiert: Warnt
SmartScreen, „Weitere Informationen“ → „Trotzdem ausführen“.

**Ausprobieren:** Mit „Öffnen …“ den Ordner `demo/` aus dem Quelltext wählen. Der Beispielartikel
zeigt Formeln, eine Abbildung, eine Tabelle, Fußnoten, Querverweise und Zitate. Ändern ist
ausdrücklich erwünscht.

**Ein neues Dokument:** TFTN startet mit einem leeren Entwurf. Einfach losschreiben; Cmd/Strg+S gibt
ihm einen Namen und legt es im Ordner „Dokumente“ ab. „Vorlage …“ legt stattdessen ein Dokument eines
Dokumenttyps an. Mitgeliefert sind: leeres Dokument, Brief, Rechnung, Befund/Arztbrief, Bericht,
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

- **Kopf:** Vorlage, Importieren, Öffnen, Fassungen, Export, Seitenleiste, Schreibansicht,
  Einstellungen. Hinweise erscheinen hier.
- **Editor** (links): Typst-Quelltext mit Hervorhebung. Darüber die Knöpfe „Baustein …“, „Zitat
  einfügen“, „Gliederung“ und „Seite …“.
- **Vorschau** (rechts): die fertigen Seiten. Ein Klick in die Vorschau springt an die Stelle im Text;
  die Vorschau folgt der Schreibmarke. Rechtsklick öffnet ein Menü mit „Ändern …“ und „Im Code zeigen“.
- **Seitenleiste** (Cmd/Strg+Umschalt+P): Fächer „Hier“, „Einfügen“, „Formel“, „Seite“, „Dokument“,
  „Pakete“. Ihre Breite lässt sich ziehen.
- **Gliederung** (Knopf „Gliederung“): die Überschriften neben dem Text.
- **Statuszeile:** Wörter und Zeichen, Dokumentsprache, Speicherstand, Netzzugriffe.

## 3. Schreiben

**Bausteine:** In einer leeren Zeile „/“ tippen öffnet die Bausteinliste (weitertippen filtert,
Enter fügt ein, Esc schließt). Cmd/Strg+Umschalt+B oder „Baustein …“ öffnen sie mit Suchfeld; markierter
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

**Zitieren:** Cmd/Strg+Umschalt+L oder „Zitat einfügen“ durchsucht die Literaturdatei. Ein
vorhandenes Zitat ändern: Schreibmarke ins Zitat und die Seitenleiste öffnen (oder Rechtsklick in der
Vorschau → „Ändern …“). Dort lassen sich Seitenangabe und Quelle wechseln.

**Literatur aus einer DOI:** Im Zitat-Dialog eine DOI eingeben (`10.1038/nature14539`, `doi:…` oder
`https://doi.org/…`). „DOI nachschlagen und zitieren“ holt den Eintrag bei Crossref bzw. DataCite,
schreibt ihn mit lesbarem Schlüssel (`lecun2015`) in die Literaturdatei und fügt das Zitat ein. Fehlt
noch ein Literaturverzeichnis, kommt es ans Ende. Übertragen wird nur die DOI.

**Gliederung:** Links stehen die Überschriften; ein Klick springt hin. Ziehen verschiebt einen
Abschnitt samt Unterabschnitten, ebenso Alt+↑/↓ auf einem Eintrag. Jede Verschiebung ist ein
Rückgängig-Schritt.

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

**Seite** (Knopf „Seite …“): Format, Querformat, doppelseitig, Spalten, Ränder, Schrift, Größe,
Zeilenabstand, Blocksatz, nummerierte Überschriften, Sprache, Seitenzahl, Kopf- und Fußzeile (mit
`{seite}` und `{kapitel}`). Jede Einstellung steht als `#set`-Zeile oben im Text. Vorlagen wie der
Fachartikel legen das selbst fest. Mit „Schriften einbinden“ kommen die benutzten Schriften ins Projekt.

**Titelei** (Fach „Dokument“, Cmd/Strg+Umschalt+O): ein Formular aus dem Dokumenttyp – Titel,
Autorinnen und Autoren, Datum (`31.12.2026`), Zusammenfassung, Schlagwörter, Literaturdateien und
Zitierstil (eingebauter Name wie `ieee` oder eine `.csl`-Datei im Projekt). Jeder Wert wird geprüft und
erst dann in `dokument.toml` geschrieben; der Text in `main.typ` bleibt unberührt. Ein Klick auf die
Titelei in der Vorschau öffnet das Fach am passenden Feld.

**Dokumenttyp aktualisieren:** Gibt es eine neuere Version des Dokumenttyps, zeigt sie das Fach
„Dokument“. „Vergleichen …“ zeigt die geänderten Seiten vorher und nachher, „Übernehmen“ stellt um.

**Eigene Dokumenttypen** liegen im Ordner `typen/` des Einstellungsordners und haben Vorrang vor den
mitgelieferten. Ein Dokumenttyp ist ein Typst-Paket mit einem Abschnitt `[tool.tftn]` in `typst.toml`.

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

**Importieren** (Knopf „Importieren …“, braucht [Pandoc](https://pandoc.org) ab 3.1.2): Word (.docx),
OpenDocument (.odt), Markdown, HTML, EPUB, LaTeX und RTF werden zu einem neuen Dokument. Überschriften,
Listen, Tabellen, Fußnoten, Formeln und Bilder kommen mit; Layout, Kommentare und Änderungsverfolgung
nicht. Das Original bleibt unberührt. Pandoc läuft abgeschottet und holt nichts aus dem Netz.
Installieren: Mac `brew install pandoc`, Windows `winget install --id JohnMacFarlane.Pandoc`.

**PDF** („Export ▾“): Die Profile kommen vom Dokumenttyp, beim Fachartikel etwa „vorschau“ und
„archiv“ (PDF/A-2b). Das PDF entsteht mit der beigelegten typst-CLI und ist reproduzierbar.

**Word (.docx) zum Gegenlesen** („Export ▾“, braucht Pandoc): Titel, Überschriften, Listen, Tabellen,
Abbildungen, echte Fußnoten, Formeln, Zitate und Literaturverzeichnis kommen mit, das Seitenlayout
nicht. Die Datei liegt als `<Dokumentname>.docx` im Dokumentordner und wird beim nächsten Export
überschrieben.

## 8. Sichern und Fassungen

Jedes Dokument sichert sich eine Sekunde nach der letzten Änderung von selbst, auch beim Schließen und
Beenden. Cmd/Strg+S braucht man nur, um einem neuen Dokument einen Namen zu geben.

**Fassungen …** zeigt frühere Stände: Höchstens alle zehn Minuten hält TFTN eine Fassung in
`.fassungen/` im Dokumentordner fest (vom letzten Tag alle, davor eine je Tag, 60 Tage lang).
Zurückholen ersetzt den Text in einem Schritt; Cmd/Strg+Z bringt den vorigen zurück.

## 9. Rechtschreibung, Zählen, Fehlermeldungen

**Rechtschreibung** nach Dokumentsprache: Unbekannte Wörter im Fließtext sind rot unterwellt, Befehle,
Formeln, Code und Verweise nie. Rechtsklick: Vorschläge, „Ins Wörterbuch aufnehmen“ (in `woerter.txt`
im Einstellungsordner) oder „Ignorieren“. Wörterbücher: Deutsch (Deutschland, Österreich, Schweiz),
Englisch (USA, Großbritannien), Französisch, Italienisch, Spanisch.

**Zählen:** Die Statuszeile zeigt Wörter und Zeichen, für eine Auswahl „Auswahl: … von …“, die
Gliederung je Abschnitt. Gezählt wird der geschriebene Text samt Unterschriften und Fußnoten.

**Fehlermeldungen** stehen am Fehler im Text und oben in der Vorschau, mit einem Tipp zur Behebung.
Solange ein Fehler besteht, zeigt die Vorschau den letzten fehlerfreien Stand.

## 10. Einstellungen und Tastenkürzel

„Einstellungen …“ (Cmd/Strg+,) zeigt alle Aktionen mit ihren Kürzeln: Zeile anklicken, neues Kürzel
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

**Einstellungsordner:** Kürzel (`tasten.toml`), Optionen, eigenes Wörterbuch, eigene Dokumenttypen,
Paketbeschreibungen und Sprachdateien liegen in einem Ordner, den Sie in den Einstellungen wählen –
etwa in einem synchronisierten Ordner, damit alle Rechner dieselbe Belegung haben. Änderungen von
außen wirken sofort.

**Sprache der Oberfläche:** Deutsch und Englisch sind dabei; „Wie das System“ ist der Standard.
„Weitere Sprache …“ legt eine Vorlage zum Übersetzen an: links Deutsch, rechts die Übersetzung, als
`<kennung>.toml` (etwa `fr.toml`) im Ordner `sprachen/` speichern – ohne neues Programm.

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
