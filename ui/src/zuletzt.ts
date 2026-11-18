// Menü „Öffnen“: Auswahl eines Projektordners und die zuletzt geöffneten Projekte (Liste in Rust,
// `zuletzt.rs`, je Rechner gemerkt).

import type { MenuePunkt } from "./knopfmenue";
import { t } from "./sprache";

export interface ZuletztJs {
  pfad: string;
  name: string;
  /** Name des Ordners darüber. */
  ort: string;
}

/**
 * Die Punkte nach „Projekt öffnen …“: je Projekt sein Name, bei gleichnamigen der Ordner darüber
 * als leiser Zusatz; der volle Pfad steht im Titel. Zuletzt „Liste leeren“. Leere Liste: nichts.
 */
export function zuletztPunkte(liste: ZuletztJs[], oeffnen: (pfad: string) => void, leeren: () => void): MenuePunkt[] {
  if (liste.length === 0) return [];
  const anzahl = new Map<string, number>();
  for (const z of liste) anzahl.set(z.name, (anzahl.get(z.name) ?? 0) + 1);
  const punkte: MenuePunkt[] = liste.map((z, i) => ({
    text: z.name,
    zusatz: (anzahl.get(z.name) ?? 0) > 1 && z.ort ? z.ort : undefined,
    titel: z.pfad,
    ikone: "dokument",
    trennerDavor: i === 0,
    aktion: () => oeffnen(z.pfad),
  }));
  punkte.push({
    text: t("Liste leeren"),
    titel: t("Vergisst die zuletzt geöffneten Projekte (nur auf diesem Rechner); die Projekte selbst bleiben"),
    trennerDavor: true,
    aktion: leeren,
  });
  return punkte;
}
