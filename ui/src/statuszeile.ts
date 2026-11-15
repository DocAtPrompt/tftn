// Statusleiste: Was der Benutzer über den Abgleich Editor ↔ Rust wissen muss. Die Messwerte
// (Revision, Prüfsummen, Latenz) sind für die Entwicklung und erscheinen nur in `tauri dev`.

import { t } from "./sprache";

export interface Abgleich {
  abgleiche: number;
  abweichungen: number;
  neusyncs: number;
  fehler: number;
}

export function syncText(
  z: Abgleich,
  rev: number,
  ungespeichert: boolean,
  entwicklung: boolean,
): { text: string; warnung: boolean } {
  const zustand = ungespeichert ? t("ungespeichert") : t("gespeichert");
  const gestoert = z.abweichungen + z.fehler;
  if (entwicklung)
    return {
      text: t("Rev {rev} · Prüfsummen gleich {gleich} · abweichend {abweichend} · Neusync {neusync} · Fehler {fehler} · {zustand}", {
        rev,
        gleich: z.abgleiche,
        abweichend: z.abweichungen,
        neusync: z.neusyncs,
        fehler: z.fehler,
        zustand,
      }),
      warnung: gestoert > 0,
    };
  if (gestoert > 0) return { text: t("Abgleich gestört ({anzahl}) · {zustand}", { anzahl: gestoert, zustand }), warnung: true };
  return { text: zustand, warnung: false };
}

/** Dauer eines Typst-Laufs für die Statusleiste: „87 ms“, ab einer Sekunde „1.2 s“. */
export function dauerText(ms: number, dezimal = "."): string {
  if (ms < 1) return "< 1 ms";
  if (ms < 999.5) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(1).replace(".", dezimal)} s`;
}
