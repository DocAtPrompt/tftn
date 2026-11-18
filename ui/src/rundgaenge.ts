// Die Rundgänge als Daten (Engine: rundgang.ts). Ein neuer Rundgang ist ein weiterer Eintrag in
// `rundgaenge()`; er erscheint dann von selbst in „Über TFTN“ → Rundgänge. Regeln (rundgaenge.test.ts):
// - Texte über t("…"), damit das Video mit englischer Oberfläche englisch ist; getippte Texte ebenso.
// - Ziele sind CSS-Auswahlen mit einer ID aus ui/index.html oder aus `DYNAMISCHE_ZIELE`.
// - Wer schreibt (tippen, taste, bausteinWaehlen, feldSetzen, Pakete, Export), braucht `kopie`: Er
//   läuft dann in einer frischen Kopie mit erfundenen Daten, nie im Dokument des Benutzers.
// - Ein Schritt ohne Ziel zeigt nur den Untertitel.
// - Ein Schritt mit `kopie` wechselt mitten im Rundgang in eine Kopie eines anderen Dokumenttyps
//   (die große Vorführung: leeres Blatt → Rechnung → Befund).
// Was mehrere Rundgänge zeigen, steht einmal als Funktion (`tabelleSchritte`, `paketSchritte` …).

import type { Rundgang, Schritt } from "./rundgang";
import { t } from "./sprache";

/**
 * Ziele, die nicht in ui/index.html stehen, sondern zur Laufzeit entstehen: ID → Modul, das sie setzt
 * (der Test prüft, dass sie dort vergeben wird). Dazu Klassen von CodeMirror (`.cm-…`) und Klassen
 * aus den Modulen der Oberfläche unter einer dieser IDs.
 */
export const DYNAMISCHE_ZIELE: Record<string, string> = {
  seitenleiste: "seitenleiste.ts",
  "export-menue": "export.ts",
  "baustein-palette": "bausteinpalette.ts",
  reiter: "reiter.ts",
  palette: "zitat.ts",
  einstellungen: "einstellungen.ts",
  ueber: "ueber.ts",
  netzprotokoll: "netzprotokoll.ts",
  "inkognito-dialog": "inkognito.ts",
  projektsuche: "projektsuche.ts",
  "serie-dialog": "serie.ts",
  "literatur-dialog": "literaturdialog.ts",
  "paket-suche": "paketfach.ts",
};

/** Das Paket, das die Rundgänge aus Typst Universe holen: klein, stabil, im Bild gut zu sehen. */
export const PAKET = "showybox";

/** So lange darf ein Handgriff mit Netzzugriff dauern (Index bzw. Paket holen). */
const NETZ_FRIST = 120_000;

/** Was sich ein Durchgang zwischen seinen Schritten merkt; je Aufruf von `rundgaenge()` frisch (also
 *  auch nach dem Neuladen bei einem Wechsel der Kopie). */
interface Merker {
  /** Das Paket ist im Dokument (geholt, aus dem Zwischenspeicher oder schon da). */
  paket: boolean;
}

/** Der letzte Untertitel eines Themen-Rundgangs in einer Kopie. */
function zurueck(text: string): Schritt {
  return { vorher: (api) => api.schliessen(), vorherIn: "beide", text };
}

// --- Bausteine -----------------------------------------------------------------------------

/** „/“-Menü, Tabelle einsetzen und ihre Felder füllen. */
function tabelleSchritte(): Schritt[] {
  return [
    {
      ziel: ".cm-tooltip-autocomplete",
      vorher: async (api) => {
        await api.schreibmarke();
        await api.tippen("\n\n/");
        await api.warten(700);
        await api.tippen(t("Tabelle"), 110);
        await api.warten(600);
      },
      text: t("In einer leeren Zeile öffnet „/“ die Bausteine: Tabelle, Abbildung, Fußnote, Formel und mehr."),
    },
    {
      ziel: "#editor",
      vorher: async (api) => {
        await api.bausteinWaehlen("typst/tabelle");
        await api.warten(500);
        await api.tippen(t("Schicht"));
        await api.taste("Tab");
        await api.tippen(t("Schlaf"));
        await api.taste("Tab");
        await api.tippen(t("früh"));
        await api.taste("Tab");
        await api.tippen(t("Schlaf nach Schicht"));
      },
      text: t("Enter setzt die Tabelle ein. Die hellblauen Felder sind Platzhalter; Tab springt zum nächsten."),
    },
  ];
}

/** Fach „Hier“ mit der Schreibmarke in der Tabelle von `tabelleSchritte`. */
function hierSchritt(text: string): Schritt {
  return {
    ziel: "#seitenleiste",
    vorher: async (api) => {
      await api.schreibmarke(t("früh"));
      await api.fachOeffnen("hier");
      await api.warten(600);
    },
    text,
  };
}

function bausteine(): Rundgang {
  return {
    id: "bausteine",
    titel: t("Bausteine und Textbausteine"),
    beschreibung: t("Tabelle und Formel über „/“ einsetzen, Werte im Fach „Hier“ und mit Marken ändern, eigene Textbausteine."),
    kopie: "tftn-fachartikel",
    schritte: [
      { text: t("Bausteine sind fertige Stücke: Überschrift, Abbildung, Tabelle, Fußnote, Formel. Den Typst-Code dafür musst du nicht kennen.") },
      ...tabelleSchritte(),
      {
        ziel: "#vorschau",
        vorher: (api) => api.vorschauAbwarten(),
        text: t("Die Tabelle steht gleich in der Vorschau, mit Nummer und Unterschrift."),
      },
      hierSchritt(t("Steht die Schreibmarke in der Tabelle, zeigt das Fach „Hier“ sie als Raster: Zeilen und Spalten einfügen, verschieben, Zellen verbinden.")),
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.schliessen();
          await api.taste("helfer/werte");
          await api.warten(800);
        },
        text: t("Cmd/Strg+Umschalt+M – oder ⌥ bzw. Alt halten – zeigt Marken hinter jedem einstellbaren Wert. Ein Klick öffnet den passenden Editor."),
      },
      {
        ziel: "#seitenleiste",
        vorher: async (api) => {
          await api.schreibmarke();
          await api.tippen("\n\n");
          await api.bausteinWaehlen("typst/formel");
          await api.warten(400);
          await api.tippen("a^2 + b^2 = c^2", 110);
          await api.fachOeffnen("formel");
          await api.warten(600);
        },
        text: t("Für Formeln gibt es das Fach „Formel“: Symbole, Brüche und Matrizen per Klick, mit Vorschau der Formel."),
      },
      {
        ziel: "#baustein-palette",
        vorher: async (api) => {
          await api.schliessen();
          await api.taste("helfer/baustein");
          await api.warten(500);
        },
        text: t("Cmd/Strg+Umschalt+B öffnet alle Bausteine als Liste mit Suche – auch die aus Paketen im Dokument."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.schliessen();
          await api.schreibmarke();
          await api.tippen("\n\n" + t("Alle Daten in diesem Beispiel sind erfunden."));
          await api.warten(300);
          await api.markieren(t("Alle Daten in diesem Beispiel sind erfunden."));
        },
        text: t("Eigene Textbausteine: einen Satz markieren, Rechtsklick, „Als Textbaustein speichern …“. Danach steht er im Menü nach „/“."),
      },
      {
        ziel: "#einstellungen-knopf",
        text: t("Gesammelt stehen sie in textbausteine.toml; die Einstellungen öffnen die Datei zum Bearbeiten."),
      },
      zurueck(t("Das waren die Bausteine. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Vorlagen und Formulare ------------------------------------------------------------------

/** Rechnung aus der Vorlage: Formular, Menge ändern, Summe, Steuerbefreiung, Logo. Wechselt in eine
 *  Kopie einer Rechnung (`anfang` ist der erste Untertitel dort). */
function rechnungSchritte(anfang: string): Schritt[] {
  return [
    { kopie: "tftn-rechnung", ziel: "#vorschau", text: anfang },
    {
      ziel: "#seitenleiste",
      vorher: (api) => api.fachOeffnen("dokument"),
      vorherIn: "beide",
      text: t("Das Fach „Dokument“ zeigt das Formular des Dokumenttyps: Absender, Empfänger, Posten, Bankverbindung."),
    },
    {
      ziel: '#seitenleiste [data-pfad="posten.0.menge"]',
      vorher: (api) => api.feldSetzen("posten.0.menge", "3"),
      text: t("Aus zwei Stunden werden drei. Netto, Umsatzsteuer und Summe rechnet die Vorlage selbst."),
    },
    {
      ziel: "#vorschau",
      vorher: (api) => api.vorschauAbwarten(),
      text: t("Die Vorschau zeigt sofort die neue Summe."),
    },
    {
      ziel: '#seitenleiste [data-pfad="befreiung"]',
      vorher: (api) => api.feldSetzen("befreiung", "heilbehandlung"),
      text: t("Für ärztliche Leistungen wählt man die Steuerbefreiung. Die Umsatzsteuer entfällt, unter der Summe steht der Hinweis darauf."),
    },
    {
      ziel: '#seitenleiste [data-pfad="logo"]',
      vorher: (api) => api.feldZeigen("logo"),
      vorherIn: "beide",
      text: t("Logo und Unterschrift sind Bilder: „Bild wählen …“ kopiert die Datei in den Dokumentordner."),
    },
  ];
}

function vorlagen(): Rundgang {
  return {
    id: "vorlagen",
    titel: t("Vorlagen und Formulare"),
    beschreibung: t("Brief und Rechnung aus der Vorlage: Formular im Fach „Dokument“, die Summe rechnet sich, Steuerbefreiung, Logo."),
    kopie: "tftn-brief",
    schritte: [
      {
        ziel: "#vorschau",
        text: t("Briefe, Rechnungen, Befunde: Wiederkehrendes beginnt mit einer Vorlage. Der Dokumenttyp bringt Layout und Formular mit."),
      },
      {
        ziel: "#seitenleiste",
        vorher: (api) => api.fachOeffnen("dokument"),
        vorherIn: "beide",
        text: t("Das Fach „Dokument“ zeigt das Formular: Absender, Empfänger, Ort, Datum, Betreff. Im Editor steht nur der Brieftext."),
      },
      {
        ziel: '#seitenleiste [data-pfad="betreff"]',
        vorher: (api) => api.feldSetzen("betreff", t("Ihre Anfrage vom 3. März")),
        text: t("Ein Feld ändert man wie in jedem Formular. Die Vorschau zieht gleich nach."),
      },
      {
        ziel: "#vorschau",
        vorher: (api) => api.vorschauAbwarten(),
        text: t("Die Werte stehen in dokument.toml im Dokumentordner – gewöhnlicher Text, kein verstecktes Format."),
      },
      ...rechnungSchritte(t("Jetzt eine Rechnung aus der Vorlage „Rechnung“, ebenfalls mit erfundenen Daten.")),
      {
        ziel: "#export-knopf",
        vorher: (api) => api.schliessen(),
        vorherIn: "beide",
        text: t("Mit Export → Serie entsteht aus einer CSV-Datei je Zeile eine eigene Rechnung."),
      },
      zurueck(t("Eigene Dokumenttypen legst du in den Einstellungen aus einem vorhandenen an. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Literatur -------------------------------------------------------------------------------

function literatur(): Rundgang {
  return {
    id: "literatur",
    titel: t("Literatur und Zitate"),
    beschreibung: t("Aus einer .bib-Datei zitieren, mit Seitenangabe; Literaturverzeichnis und Zitierstil; DOI nachschlagen (nur erklärt)."),
    kopie: "tftn-fachartikel",
    schritte: [
      {
        ziel: "#editor",
        text: t("Zitate kommen aus einer .bib-Datei im Dokumentordner, etwa aus Zotero mit Better BibTeX. Dieses Beispiel hat einen erfundenen Eintrag."),
      },
      {
        ziel: "#palette",
        vorher: async (api) => {
          await api.schreibmarke();
          await api.tippen("\n\n" + t("Schlaf wird seit Langem erforscht") + " ");
          await api.taste("helfer/zitat");
          await api.warten(400);
        },
        text: t("Cmd/Strg+Umschalt+L öffnet den Zitat-Dialog. Er sucht nach Autor, Jahr, Titel und Schlüssel."),
      },
      {
        ziel: "#palette",
        vorher: async (api) => {
          await api.eingeben("#palette .palette-suche", t("Muster"));
          await api.warten(500);
          await api.eingeben("#palette .palette-seite", "12");
        },
        text: t("Tippen filtert die Einträge; ins Feld „Seite“ kommt auf Wunsch die Seitenzahl."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.taste("Enter");
          await api.warten(400);
          await api.tippen(".");
        },
        text: t("Enter setzt das Zitat ein. Im Text ist es ein kurzer Schlüssel nach einem @."),
      },
      {
        ziel: "#vorschau",
        vorher: (api) => api.vorschauAbwarten(),
        text: t("Die Vorschau setzt es im gewählten Zitierstil; das Literaturverzeichnis am Ende entsteht von selbst."),
      },
      {
        ziel: "#palette .zitat-doi",
        vorher: async (api) => {
          await api.taste("helfer/zitat");
          await api.warten(400);
          await api.eingeben("#palette .palette-suche", "10.1000/182");
          await api.warten(400);
        },
        text: t("Steht eine DOI im Suchfeld, bietet der Dialog an, sie bei Crossref oder DataCite nachzuschlagen."),
      },
      {
        ziel: "#palette .zitat-doi",
        text: t("Das wäre ein Netzzugriff, nur auf Enter. Dieser Rundgang löst ihn nicht aus; sonst käme der Eintrag in die .bib-Datei."),
      },
      {
        ziel: "#literatur-dialog",
        vorher: async (api) => {
          await api.schliessen();
          await api.dialogOeffnen("literatur");
        },
        vorherIn: "beide",
        text: t("„Literatur verwalten …“ zeigt die Einträge der Datei und ändert sie in einem Formular."),
      },
      {
        ziel: '#seitenleiste [data-pfad="literatur.stil"]',
        vorher: async (api) => {
          await api.schliessen();
          await api.feldZeigen("literatur.stil");
        },
        vorherIn: "beide",
        text: t("Der Zitierstil ist ein Feld im Fach „Dokument“, etwa APA, IEEE oder eine eigene .csl-Datei."),
      },
      zurueck(t("Das war die Literatur. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Pakete ----------------------------------------------------------------------------------

/** Paket suchen, hinzufügen, verwenden. `ausfuehrlich`: mit der Zeile des Treffers (Themen-Rundgang). */
function paketSchritte(m: Merker, ausfuehrlich: boolean): Schritt[] {
  const ohne = t("Der Rundgang geht ohne Paket weiter.");
  const schritte: Schritt[] = [
    {
      ziel: "#seitenleiste",
      frist: NETZ_FRIST,
      vorher: async (api) => {
        await api.schliessen();
        const s = await api.paketSuchen(PAKET);
        if (s.index === "geholt") await api.sagen(t("Zuerst holt TFTN einmal den Paketindex von packages.typst.org. Danach sucht es darin ohne Netz."));
        else if (s.index === "fehlt") await api.sagen(t("Ohne Paketindex gibt es nichts zu suchen: „Ganz offline“ ist an, oder das Holen ging nicht.") + " " + ohne);
        else if (!s.gefunden) await api.sagen(t("Der Paketindex kennt {paket} nicht.", { paket: PAKET }) + " " + ohne);
      },
      text: t("Pakete aus Typst Universe gibt es im Fach „Pakete“. Gesucht wird im Index auf diesem Rechner, ohne Netz."),
    },
  ];
  if (ausfuehrlich) {
    schritte.push({
      ziel: `#seitenleiste [data-paket="${PAKET}"]`,
      text: t("{paket} zeichnet farbige Kästen. Die Zeile nennt die neueste Version und die Lizenz.", { paket: PAKET }),
    });
  }
  schritte.push(
    {
      ziel: "#seitenleiste",
      frist: NETZ_FRIST,
      vorher: async (api) => {
        const e = await api.paketHinzufuegen(PAKET);
        m.paket = e === "geholt" || e === "zwischenspeicher" || e === "schon-da";
        const anders: Record<string, string> = {
          zwischenspeicher: t("{paket} liegt schon im Zwischenspeicher dieses Rechners. TFTN kopiert es von dort, ganz ohne Netz.", { paket: PAKET }),
          "schon-da": t("{paket} ist schon in diesem Dokument.", { paket: PAKET }),
          offline: t("„Ganz offline“ ist an, deshalb holt TFTN nichts. In den Einstellungen lässt es sich ausschalten.") + " " + ohne,
          "nicht-gefunden": t("Ohne Treffer gibt es nichts hinzuzufügen.") + " " + ohne,
          fehler: t("Das Holen ging nicht; der Grund steht oben im Hinweis.") + " " + ohne,
        };
        const text = e === "geholt" ? undefined : anders[e];
        if (text) await api.sagen(text);
      },
      text: t("„Hinzufügen“ holt das Paket von packages.typst.org – ein Netzzugriff, nur auf diesen Klick."),
    },
    {
      ziel: "#editor",
      vorher: async (api) => {
        if (!m.paket) return api.sagen(t("Mit dem Paket stünde oben eine Importzeile; eine weitere Zeile genügte, es zu verwenden."));
        await api.schliessen();
        await api.markieren(`@preview/${PAKET}:`);
      },
      text: t("Oben steht jetzt die Importzeile, die TFTN eingefügt hat. Sie ist gewöhnliches Typst."),
    },
    {
      ziel: "#editor",
      vorher: async (api) => {
        if (!m.paket) return api.sagen(ohne);
        await api.schreibmarke();
        await api.tippen(`\n\n#${PAKET}.${PAKET}(title: [` + t("Aus Typst Universe") + "])[" + t("Dieser Kasten kommt aus einem Paket.") + "]");
      },
      text: t("Verwendet wird das Paket wie in jedem Typst-Dokument: eine Zeile genügt."),
    },
    {
      ziel: "#vorschau",
      vorher: async (api) => {
        if (!m.paket) return api.sagen(t("Gelingt das Holen, liegt das Paket danach im Dokumentordner und geht von da an offline."));
        await api.vorschauAbwarten();
      },
      text: t("Die Vorschau zeigt den Kasten. Das Paket liegt jetzt im Dokumentordner und geht von da an offline."),
    },
  );
  return schritte;
}

/** Netzprotokoll öffnen; ohne einen Zugriff seit dem Start sagt der Untertitel das. */
function netzprotokollSchritt(text: string): Schritt {
  return {
    ziel: "#netzprotokoll",
    vorher: async (api) => {
      await api.schliessen();
      await api.dialogOeffnen("netzprotokoll");
      if ((await api.netzZugriffe()) === 0) await api.sagen(t("Das Netzprotokoll ist leer: Seit dem Start ging TFTN nicht ins Netz."));
    },
    vorherIn: "beide",
    text,
  };
}

function pakete(m: Merker): Rundgang {
  return {
    id: "pakete",
    titel: t("Pakete aus Typst Universe"),
    beschreibung: t("Ein Paket suchen, hinzufügen (ein Netzzugriff) und verwenden; danach liegt es im Dokumentordner und geht offline."),
    kopie: "tftn-fachartikel",
    schritte: [
      { text: t("Typst Universe hat Hunderte Pakete: Kästen, Diagramme, Tabellen, Briefe. TFTN holt sie ins Dokument, auf Klick.") },
      ...paketSchritte(m, true),
      netzprotokollSchritt(t("Das Netzprotokoll verzeichnet jeden Zugriff seit dem Start: Zeit, Zweck, Gegenstelle und Ergebnis.")),
      {
        ziel: "#netz",
        vorher: (api) => api.schliessen(),
        vorherIn: "beide",
        text: t("Die Statuszeile zählt die Zugriffe; ein Klick darauf öffnet das Netzprotokoll."),
      },
      {
        ziel: "#seitenleiste",
        vorher: async (api) => {
          await api.fachOeffnen("pakete");
          if (!m.paket) await api.sagen(t("Unter „Im Dokument“ stünde das Paket, mit Version und vermerkter Prüfsumme."));
        },
        text: t("Unter „Im Dokument“ steht das Paket mit Version und vermerkter Prüfsumme. Entfernen geht ebenso mit einem Klick."),
      },
      {
        text: t("Es liegt im Dokumentordner unter packages/. Auf einem anderen Rechner setzt das Dokument genauso, auch ohne Netz."),
      },
      zurueck(t("Das waren die Pakete. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Lange Dokumente -------------------------------------------------------------------------

function langeDokumente(): Rundgang {
  const neu = t("kapitel/03-methoden.typ");
  return {
    id: "lange-dokumente",
    titel: t("Lange Dokumente"),
    beschreibung: t("Kapitel als eigene Dateien mit #include, Reiter, Gliederung und Suchen im ganzen Dokument."),
    kopie: "tftn-fachbuch",
    schritte: [
      { ziel: "#editor", text: t("Ein Buch muss nicht in einer Datei stehen. Hier bindet die Hauptdatei die Kapitel mit #include ein.") },
      { ziel: "#vorschau", text: t("Die Vorschau zeigt trotzdem das ganze Buch, über alle Kapitel.") },
      {
        ziel: "#reiter",
        vorher: (api) => api.reiterOeffnen(t("kapitel/01-einleitung.typ")),
        text: t("Eine Kapiteldatei öffnet sich als Reiter über dem Editor; „+“ öffnet weitere."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.schreibmarke();
          await api.tippen("\n" + t("Hier geht das erste Kapitel weiter."));
          await api.vorschauAbwarten();
        },
        text: t("Getippt wird im Kapitel; die Vorschau setzt das ganze Buch mit."),
      },
      {
        ziel: "#gliederung",
        vorher: (api) => api.gliederung(true),
        vorherIn: "beide",
        text: t("Die Gliederung neben dem Text zeigt die Überschriften der offenen Datei. Ein Klick springt hin, Ziehen verschiebt ganze Abschnitte."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.reiterOeffnen("main.typ");
          await api.schreibmarke();
          await api.tippen('\n#include "' + neu + '"');
        },
        text: t("Ein neues Kapitel ist eine Zeile #include in der Hauptdatei. Fehlt die Datei, legt TFTN sie an, mit einer Überschrift aus dem Namen."),
      },
      {
        ziel: "#reiter",
        vorher: async (api) => {
          await api.reiterOeffnen(neu);
          await api.schreibmarke();
          await api.tippen("\n" + t("Jedes Kapitel steht in einer eigenen Datei."));
        },
        text: t("Die neue Datei öffnet sich als Reiter, und das Kapitel steht schon im Buch."),
      },
      {
        ziel: "#projektsuche",
        vorher: async (api) => {
          await api.taste("helfer/projektsuche");
          await api.warten(400);
          await api.eingeben("#projektsuche .psuche-feld", t("Kapitel"));
          await api.warten(800);
        },
        text: t("Cmd/Strg+Umschalt+H sucht und ersetzt im ganzen Dokument, über alle Kapiteldateien."),
      },
      { ziel: "#projektsuche", text: t("Die Treffer stehen nach Datei geordnet; ein Klick öffnet die Datei an der Stelle.") },
      zurueck(t("Das waren die langen Dokumente. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Export ----------------------------------------------------------------------------------

/** Export-Menü öffnen und eine Probe als PDF setzen. */
function exportSchritte(): Schritt[] {
  return [
    {
      ziel: "#export-menue",
      vorher: async (api) => {
        await api.schliessen();
        await api.exportMenue(true);
      },
      vorherIn: "beide",
      text: t("Export: oben die PDF-Profile des Dokumenttyps, darunter Drucken, Archivpaket, Word, Webseite, E-Book und Serie."),
    },
    {
      ziel: "#export-menue",
      frist: 60_000,
      vorher: (api) => api.exportProbe(),
      text: t("„Probe“ setzt ein PDF in den Ordner _renders/. Darunter steht der Befehl, der dasselbe PDF ohne TFTN erzeugt."),
    },
  ];
}

function fassungenSchritt(): Schritt {
  return {
    ziel: "#fassungen-knopf",
    vorher: (api) => api.schliessen(),
    vorherIn: "beide",
    text: t("Frühere Fassungen hebt TFTN von selbst auf; jede lässt sich ansehen und zurückholen."),
  };
}

function exportUndSerie(): Rundgang {
  return {
    id: "export",
    titel: t("Export und Serie"),
    beschreibung: t("PDF-Profile, eine Probe als PDF, Drucken, Archivpaket, Word, Webseite, E-Book, Serie aus CSV und Fassungen."),
    kopie: "tftn-rechnung",
    schritte: [
      { ziel: "#export-knopf", text: t("Export macht aus dem Dokument Dateien. Der Text selbst bleibt dabei, wie er ist.") },
      ...exportSchritte(),
      {
        ziel: "#export-menue",
        text: t("„Abnehmen …“ schreibt die Endfassung nach export/, mit dem Zeitstempel von jetzt. Die Probe bleibt im nicht synchronisierten _renders/."),
      },
      {
        ziel: "#export-menue",
        text: t("Drucken setzt das PDF und öffnet es im PDF-Betrachter des Systems; dort wird gedruckt. Kürzel: Cmd/Strg+P."),
      },
      {
        ziel: "#export-menue",
        text: t("Das Archivpaket ist ein Zip mit PDF/A und allem, was es zum Neusetzen braucht – auch in Jahren und ohne TFTN, Byte für Byte gleich."),
      },
      {
        ziel: "#export-menue",
        text: t("Word, Webseite und E-Book entstehen über Pandoc, zum Gegenlesen oder für Lesegeräte; das Seitenlayout bleibt dabei weg."),
      },
      {
        ziel: "#serie-dialog",
        vorher: async (api) => {
          await api.schliessen();
          await api.dialogOeffnen("serie");
        },
        vorherIn: "beide",
        text: t("Serie: je Zeile einer CSV-Datei ein PDF, etwa für Serienbriefe und Rechnungen. Die Kopfzeile nennt die Felder."),
      },
      {
        ziel: "#serie-dialog",
        text: t("Die Datei wählt man im Systemdialog; das zeigt dieser Rundgang nicht. Die PDFs entstehen neben den Daten."),
      },
      fassungenSchritt(),
      zurueck(t("Das war der Export. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Datenschutz -----------------------------------------------------------------------------

/** „Ganz offline“ in den Einstellungen zeigen (nicht ändern). */
function offlineSchritt(text: string): Schritt {
  return {
    ziel: "#einstellungen .einst-offline",
    vorher: async (api) => {
      await api.schliessen();
      await api.dialogOeffnen("einstellungen");
    },
    vorherIn: "beide",
    text,
  };
}

/** Befund: Feld mit Personendaten, „Inkognito für KI“ und die Tabelle „Was die KI sieht“. */
function inkognitoSchritte(): Schritt[] {
  return [
    {
      ziel: '#seitenleiste [data-pfad="patient.name"]',
      vorher: async (api) => {
        await api.schliessen();
        await api.feldZeigen("patient.name");
      },
      vorherIn: "beide",
      text: t("Felder mit Personendaten sind „Inkognito für KI“: Ein KI-Assistent, über MCP verbunden, sieht nur einen Ersatztext."),
    },
    {
      ziel: "#inkognito-dialog",
      vorher: (api) => api.dialogOeffnen("inkognito"),
      vorherIn: "beide",
      text: t("„Was die KI sieht“ stellt Ersatztext und echten Wert gegenüber. Diese Tabelle bleibt auf dem Rechner."),
    },
  ];
}

function blattSchritt(): Schritt {
  return {
    ziel: "#netzprotokoll .netz-unten",
    text: t("Für Datenschutzbeauftragte gibt es eine Seite als PDF: was TFTN lokal tut und welche Zugriffe es gab."),
  };
}

function datenschutz(): Rundgang {
  return {
    id: "datenschutz",
    titel: t("Datenschutz"),
    beschreibung: t("Ganz offline, Netzprotokoll, Blatt für den Datenschutz, MCP und „Inkognito für KI“ an einem Befund mit erfundenen Daten."),
    kopie: "tftn-befund",
    schritte: [
      { text: t("TFTN läuft ganz auf diesem Rechner: ohne Konto, ohne Cloud, ohne Telemetrie.") },
      { ziel: "#editor", text: t("Dieser Befund enthält Personendaten – hier erfundene. Sie bleiben im Dokumentordner auf dem Rechner.") },
      netzprotokollSchritt(t("Das Netzprotokoll zeigt jeden Netzzugriff seit dem Start: Zeit, Zweck, Gegenstelle und Ergebnis.")),
      {
        ziel: "#netzprotokoll",
        text: t("Ins Netz geht nur ein kleines Hilfsprogramm, und nur auf Klick: Pakete holen, nach Updates suchen, eine DOI nachschlagen."),
      },
      blattSchritt(),
      offlineSchritt(t("„Ganz offline“ sperrt auch diese Zugriffe; Pakete kommen dann nur aus dem Zwischenspeicher.")),
      {
        ziel: "#einstellungen .einst-mcp",
        text: t("MCP lässt KI-Assistenten im offenen Dokument lesen und schreiben. Eingeschaltet wird es hier, von Hand."),
      },
      ...inkognitoSchritte(),
      zurueck(t("Das war der Datenschutz. Gleich ist wieder dein eigenes Dokument offen.")),
    ],
  };
}

// --- Aussehen ----------------------------------------------------------------------------------

function aussehen(): Rundgang {
  return {
    id: "aussehen",
    titel: t("Aussehen und Einstellungen"),
    beschreibung: t("Skins, Größe der Oberfläche, Editorschrift, Schreibansicht, Tastenkürzel und F1; danach ist alles wie vorher."),
    kopie: "tftn-fachartikel",
    schritte: [
      { text: t("TFTN lässt sich einstellen, wie du arbeitest. Was dieser Rundgang ändert, stellt er am Ende zurück.") },
      {
        ziel: "#einstellungen .einst-skin",
        vorher: (api) => api.dialogOeffnen("einstellungen"),
        vorherIn: "beide",
        text: t("Skins ändern Farben und Editorschrift. Einige sind dabei; eigene entstehen aus einer Vorlage."),
      },
      {
        ziel: "#einstellungen .einst-skin",
        vorher: async (api) => {
          await api.skinSetzen("dunkel");
          await api.warten(2000);
          await api.skinSetzen("papier");
        },
        text: t("Ein Skin wirkt sofort, ohne Neustart."),
      },
      {
        ziel: "#einstellungen .einst-groesse",
        vorher: (api) => api.oberflaeche(115),
        text: t("Die Größe der Oberfläche lässt sich eigens stellen: Leisten, Knöpfe und Dialoge wachsen, Editor und Vorschau bleiben."),
      },
      {
        ziel: "#einstellungen .einst-sprache",
        vorher: (api) => api.oberflaeche(100),
        text: t("Auch die Sprache der Oberfläche steht hier; mitgeliefert sind Deutsch und Englisch."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.schliessen();
          await api.taste("ansicht/schrift-groesser");
          await api.warten(800);
          await api.taste("ansicht/schrift-groesser");
        },
        text: t("Cmd/Strg und Plus bzw. Minus ändern die Editorschrift; Cmd/Strg+0 stellt sie zurück."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.taste("ansicht/schrift-normal");
          await api.warten(800);
          await api.taste("helfer/schreibansicht");
        },
        text: t("Die Schreibansicht – Cmd/Strg+Umschalt+Eingabe – zeigt nur den Text, ohne Ablenkung."),
      },
      {
        ziel: "#einstellungen .einst-tabelle",
        vorher: async (api) => {
          await api.taste("helfer/schreibansicht");
          await api.dialogOeffnen("einstellungen");
          await api.eingeben("#einstellungen .palette-suche", t("fett"));
        },
        text: t("Jedes Tastenkürzel lässt sich ändern: Zeile anklicken, neue Tasten drücken. Konflikte zeigt TFTN gleich an."),
      },
      {
        ziel: "#ueber",
        vorher: async (api) => {
          await api.schliessen();
          await api.taste("helfer/ueber");
        },
        text: t("F1 öffnet „Über TFTN“: alle Kürzel auf einen Blick, die Links zum Handbuch und diese Rundgänge."),
      },
      zurueck(t("Das waren Aussehen und Einstellungen. Skin, Größe und Schrift sind wieder wie vorher.")),
    ],
  };
}

// --- Die große Vorführung ------------------------------------------------------------------------

/** TFTN in wenigen Minuten: leeres Blatt → Rechnung → Befund (zwei Wechsel der Kopie). */
function inMinuten(m: Merker): Rundgang {
  const kapitel = t("kapitel/methoden.typ");
  return {
    id: "in-minuten",
    titel: t("TFTN in wenigen Minuten"),
    beschreibung: t("Die Vorführung: schreiben mit Vorschau, Bausteine, ein Paket aus Typst Universe, Kapitel, eine Rechnung aus der Vorlage, Export, Netzprotokoll und Inkognito."),
    kopie: "tftn-leer",
    schritte: [
      { text: t("Das ist TFTN, ein Schreibwerkzeug für Typst. Es läuft ganz auf diesem Rechner – ohne Konto, ohne Cloud.") },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.schreibmarke();
          await api.tippen("\n\n= " + t("Ein kurzer Bericht") + "\n\n" + t("Typst setzt Text sauber und schnell. Dieses Dokument ist eben erst entstanden."));
        },
        text: t("Links steht gewöhnlicher Typst-Text. Ein „=“ am Zeilenanfang macht eine Überschrift."),
      },
      {
        ziel: "#vorschau",
        vorher: (api) => api.vorschauAbwarten(),
        text: t("Rechts setzt die Vorschau die Seite nach jedem Tastendruck neu."),
      },
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.markieren(t("sauber und schnell"));
          await api.warten(500);
          await api.taste("typst/fett");
        },
        text: t("Kürzel helfen beim Auszeichnen: Cmd/Strg+B macht die Markierung fett."),
      },
      ...tabelleSchritte(),
      {
        ziel: "#vorschau",
        vorher: async (api) => {
          await api.vorschauAbwarten();
          await api.inVorschauKlicken(t("Ein kurzer Bericht"));
        },
        text: t("Ein Klick in die Vorschau setzt die Schreibmarke an die Stelle im Text – und die Vorschau zeigt, wo die Schreibmarke steht."),
      },
      hierSchritt(t("Werte ändern geht ohne Code: Steht die Schreibmarke in der Tabelle, zeigt das Fach „Hier“ sie als Raster.")),
      ...paketSchritte(m, false),
      {
        ziel: "#editor",
        vorher: async (api) => {
          await api.schliessen();
          await api.schreibmarke();
          await api.tippen('\n\n#include "' + kapitel + '"');
        },
        text: t("Lange Texte teilt man in Kapiteldateien; #include bindet eine ein. Fehlt die Datei, legt TFTN sie an."),
      },
      {
        ziel: "#reiter",
        vorher: async (api) => {
          await api.reiterOeffnen(kapitel);
          await api.schreibmarke();
          await api.tippen("\n" + t("Jedes Kapitel steht in einer eigenen Datei; die Vorschau zeigt das ganze Dokument."));
        },
        text: t("Jedes Kapitel öffnet sich als Reiter über dem Editor."),
      },
      { ziel: "#projektsuche-knopf", text: t("Suchen und Ersetzen geht über alle Kapiteldateien zugleich.") },
      ...rechnungSchritte(t("Wiederkehrendes beginnt mit einer Vorlage. Hier eine Rechnung, mit erfundenen Daten.")),
      ...exportSchritte(),
      fassungenSchritt(),
      { kopie: "tftn-befund", ziel: "#vorschau", text: t("Zum Schluss der Datenschutz, etwa in einer Arztpraxis. Ein Befund, wieder mit erfundenen Daten.") },
      netzprotokollSchritt(t("Das Netzprotokoll zeigt jeden Netzzugriff seit dem Start: Zeit, Zweck, Gegenstelle und Ergebnis – hier das Holen des Pakets.")),
      blattSchritt(),
      offlineSchritt(t("„Ganz offline“ in den Einstellungen sperrt auch diese Zugriffe.")),
      ...inkognitoSchritte(),
      {
        ziel: "#editor",
        vorher: (api) => api.schliessen(),
        vorherIn: "beide",
        text: t("Und der Text bleibt gewöhnliches Typst. Der Dokumentordner setzt auch mit dem typst-Befehl, ohne TFTN, genau gleich."),
      },
      {
        ziel: "#seite-knopf",
        text: t("Schriften lassen sich in den Dokumentordner einbinden. Dann setzt er auf jedem Rechner mit denselben Schriften."),
      },
      {
        ziel: "#ueber",
        vorher: (api) => api.taste("helfer/ueber"),
        text: t("F1 öffnet „Über TFTN“: alle Kürzel, das Handbuch und Rundgänge zu einzelnen Themen."),
      },
      {
        vorher: (api) => api.schliessen(),
        vorherIn: "beide",
        text: t("TFTN ist frei und quelloffen. Mehr unter github.com/DocAtPrompt/tftn"),
        dauer: 8000,
      },
    ],
  };
}

export function rundgaenge(): Rundgang[] {
  const m: Merker = { paket: false };
  return [
    inMinuten(m),

    {
      id: "ueberblick",
      titel: t("Überblick"),
      beschreibung: t("Wo was ist: Kopfleiste, Editor, Vorschau, Bausteine, Seitenleiste, Export, Statuszeile und Hilfe."),
      schritte: [
        { text: t("Willkommen bei TFTN. Dieser Rundgang zeigt in einer Minute, wo was ist.") },
        {
          ziel: "#kopf",
          text: t("Oben die Kopfleiste: links der Name des Dokuments, rechts Neu, Öffnen, Fassungen, Export und die Einstellungen."),
        },
        {
          ziel: "#editor",
          text: t("Links schreibst du – gewöhnlicher Typst-Text, mit Hervorhebung und Rechtschreibprüfung. Gespeichert wird von selbst."),
        },
        {
          ziel: "#vorschau",
          text: t("Rechts die Vorschau: Sie setzt die Seiten nach jedem Tastendruck neu. Ein Klick auf eine Stelle springt im Text dorthin."),
        },
        {
          ziel: "#baustein-knopf",
          text: t("In einer leeren Zeile öffnet „/“ das Menü der Bausteine: Überschrift, Abbildung, Tabelle, Fußnote und mehr. Dasselbe gibt es über diesen Knopf."),
        },
        {
          ziel: "#seitenleiste",
          vorher: (api) => api.fachOeffnen("einfuegen"),
          vorherIn: "beide",
          text: t("Die Seitenleiste zeigt, was an der Schreibmarke steht, und hat Fächer für Bausteine, Formeln, Seite, Dokument und Pakete."),
        },
        {
          ziel: "#export-menue",
          vorher: async (api) => {
            await api.schliessen();
            await api.exportMenue(true);
          },
          vorherIn: "beide",
          text: t("Export macht aus dem Dokument ein PDF nach den Profilen des Dokumenttyps, dazu Druck, Archivpaket, Word, Webseite und E-Book."),
        },
        {
          ziel: "#status",
          vorher: (api) => api.schliessen(),
          vorherIn: "beide",
          text: t("Unten die Statuszeile: Wörter, Sprache, Speicherstand, Zoom der Vorschau – und nach einem Netzzugriff dessen Zahl. Ein Klick darauf zeigt das Netzprotokoll."),
        },
        {
          ziel: "#ueber-knopf",
          text: t("Das „?“ öffnet „Über TFTN“: alle Tastenkürzel, die Links zum Handbuch und diese Rundgänge."),
        },
        { text: t("Das war der Überblick. Weitere Rundgänge stehen unter „?“ → Rundgänge.") },
      ],
    },
    {
      id: "schreiben",
      titel: t("Schreiben und Vorschau"),
      beschreibung: t("Text tippen, auszeichnen, einen Baustein über „/“ einsetzen und aus der Vorschau an die Stelle im Text springen."),
      kopie: "tftn-fachartikel",
      schritte: [
        { text: t("Dieser Rundgang läuft in einer Kopie mit erfundenen Daten; dein eigenes Dokument bleibt, wie es ist.") },
        {
          ziel: "#editor",
          vorher: async (api) => {
            await api.schreibmarke();
            await api.tippen("\n\n= " + t("Ergebnisse") + "\n\n" + t("Nach acht Wochen schliefen die Teilnehmenden im Mittel 40 Minuten länger."));
          },
          text: t("Geschrieben wird gewöhnlicher Text. Ein „=“ am Zeilenanfang macht eine Überschrift, eine Leerzeile einen neuen Absatz."),
        },
        {
          ziel: "#vorschau",
          vorher: (api) => api.warten(600),
          text: t("Die Vorschau setzt mit jedem Zeichen neu, meist in wenigen Millisekunden."),
        },
        {
          ziel: "#editor",
          vorher: async (api) => {
            await api.markieren(t("40 Minuten"));
            await api.warten(500);
            await api.taste("typst/fett");
          },
          text: t("Markieren und Cmd/Strg+B: Der Text wird fett. TFTN schreibt das Typst-Zeichen dafür selbst."),
        },
        {
          ziel: ".cm-tooltip-autocomplete",
          vorher: async (api) => {
            await api.schreibmarke();
            await api.tippen("\n\n/");
            await api.warten(700);
            await api.tippen(t("Fußnote"), 90);
            await api.warten(600);
          },
          text: t("In einer leeren Zeile öffnet „/“ das Menü der Bausteine. Weitertippen filtert die Liste."),
        },
        {
          ziel: "#editor",
          vorher: async (api) => {
            await api.bausteinWaehlen("typst/fussnote");
            await api.warten(400);
            await api.tippen(t("Alle Daten in diesem Beispiel sind erfunden."));
          },
          text: t("Enter setzt den Baustein ein; die Schreibmarke steht gleich dort, wo der Text hinkommt."),
        },
        {
          ziel: "#vorschau",
          vorher: async (api) => {
            await api.warten(1200);
            await api.inVorschauKlicken(t("Ergebnisse"));
          },
          text: t("Ein Klick in die Vorschau setzt die Schreibmarke an die passende Stelle im Text."),
        },
        {
          ziel: "#editor",
          text: t("So findet man auch in langen Dokumenten jede Stelle schnell wieder. Gleich ist wieder das eigene Dokument offen."),
        },
      ],
    },
    bausteine(),
    vorlagen(),
    literatur(),
    pakete(m),
    langeDokumente(),
    exportUndSerie(),
    datenschutz(),
    aussehen(),
  ];
}
