// Seitenvergleich alt/neu mit „Übernehmen“ und „Verwerfen“, für Dokumenttyp (AP8) und Pakete (AP10).

import type { VergleichJs } from "./formular";
import { t } from "./sprache";

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

export interface VergleichAktionen {
  uebernehmen(knopf: HTMLButtonElement): void;
  verwerfen(): void;
  /** Satz, wenn keine Importstelle gefunden wurde. */
  ohneImport: string;
}

export function vergleichTeil(v: VergleichJs, a: VergleichAktionen): DocumentFragment {
  const d = document.createDocumentFragment();
  const nummern = v.geaendert.map((i) => i + 1);
  const wo = { nummern: nummern.join(", "), alt: v.seiten_alt, neu: v.seiten_neu };
  d.append(
    el(
      "p",
      "",
      v.fehler.length
        ? t("Die neue Version übersetzt nicht:")
        : nummern.length
          ? nummern.length === 1
            ? t("Seite {nummern} ändert sich (vorher {alt}, nachher {neu} Seiten).", wo)
            : t("Seiten {nummern} ändern sich (vorher {alt}, nachher {neu} Seiten).", wo)
          : t("Keine sichtbare Änderung."),
    ),
  );
  for (const f of v.fehler) d.append(el("p", "dok-fehler", f));
  for (const b of v.bilder) {
    const paar = el("div", "dok-seitenpaar");
    paar.append(el("div", "leiste-hilfe", t("Seite {nr}", { nr: b.seite + 1 })));
    for (const [titel, url] of [
      [t("vorher"), b.alt],
      [t("nachher"), b.neu],
    ] as const) {
      const fig = el("figure");
      if (url) {
        const img = el("img");
        img.src = url;
        img.alt = t("Seite {nr} {titel}", { nr: b.seite + 1, titel });
        fig.append(img);
      } else fig.append(el("div", "leiste-leer", "–"));
      fig.append(el("figcaption", "", titel));
      paar.append(fig);
    }
    d.append(paar);
  }
  if (v.geaendert.length > v.bilder.length) d.append(el("p", "leiste-hilfe", t("Bilder nur für die ersten {anzahl} Seiten.", { anzahl: v.bilder.length })));
  if (v.stellen.length) d.append(el("p", "leiste-hilfe", t("Importzeile wird geändert in: {stellen}", { stellen: v.stellen.join(", ") })));
  else d.append(el("p", "leiste-hilfe", a.ohneImport));
  const uebernehmen = el("button", "", t("Übernehmen"));
  uebernehmen.disabled = v.fehler.length > 0;
  uebernehmen.addEventListener("click", () => a.uebernehmen(uebernehmen));
  const verwerfen = el("button", "", t("Verwerfen"));
  verwerfen.addEventListener("click", () => a.verwerfen());
  d.append(uebernehmen, verwerfen);
  return d;
}
