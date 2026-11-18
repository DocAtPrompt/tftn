// Die Rundgänge als Daten (Engine: rundgang.ts). Ein neuer Rundgang ist ein weiterer Eintrag in
// `rundgaenge()`; er erscheint dann von selbst in „Über TFTN“ → Rundgänge. Regeln (rundgaenge.test.ts):
// - Texte über t("…"), damit das Video mit englischer Oberfläche englisch ist; getippte Texte ebenso.
// - Ziele sind CSS-Auswahlen mit einer ID aus ui/index.html oder aus `DYNAMISCHE_ZIELE`.
// - Wer schreibt (tippen, taste, bausteinWaehlen), braucht `kopie`: Er läuft dann in einer frischen
//   Kopie mit erfundenen Daten, nie im Dokument des Benutzers.
// - Ein Schritt ohne Ziel zeigt nur den Untertitel.

import type { Rundgang } from "./rundgang";
import { t } from "./sprache";

/**
 * Ziele, die nicht in ui/index.html stehen, sondern zur Laufzeit entstehen: ID → Modul, das sie setzt
 * (der Test prüft, dass sie dort vergeben wird). Dazu Klassen von CodeMirror (`.cm-…`).
 */
export const DYNAMISCHE_ZIELE: Record<string, string> = {
  seitenleiste: "seitenleiste.ts",
  "export-menue": "export.ts",
  "baustein-palette": "bausteinpalette.ts",
};

export function rundgaenge(): Rundgang[] {
  return [
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
  ];
}
