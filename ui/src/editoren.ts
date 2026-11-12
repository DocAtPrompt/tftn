// Editoren der Widgets (PLAN-M2 AP6) für Seitenleiste, Marken und Vorschau-Menü. Die Umwandlung
// steht in `werte.ts` (geprüft); hier nur DOM. Gesetzt wird beim Bestätigen (Enter, Verlassen,
// Auswahl), nie bei jedem Tastendruck; nur der Farbwähler meldet Zwischenwerte beim Ziehen.

import { type FeldJs, type Wert, eingabeZuWert, wertZuEingabe } from "./werte";
import { t } from "./sprache";

export type Setzen = (w: Wert, ende: boolean) => void;

function el<K extends keyof HTMLElementTagNameMap>(tag: K, klasse = "", text = ""): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (klasse) e.className = klasse;
  if (text) e.textContent = text;
  return e;
}

/** Editor für ein Feld; `setzen(wert, ende)` – `ende=false` nur beim Ziehen im Farbwähler. */
export function editor(feld: FeldJs, setzen: Setzen): HTMLElement {
  const huelle = el("div", "feld-editor");
  if (feld.gesperrt) {
    const g = el("div", "feld-gesperrt");
    if (feld.quelltext) g.append(el("code", "", feld.quelltext), " – ");
    g.append(feld.gesperrt);
    huelle.append(g);
    return huelle;
  }
  const meldung = el("div", "feld-fehler");
  meldung.hidden = true;
  const versuchen = (text: string, ende = true) => {
    const e = eingabeZuWert(feld.art, text);
    if ("fehler" in e) {
      meldung.textContent = e.fehler;
      meldung.hidden = false;
      return;
    }
    meldung.hidden = true;
    setzen(e.wert, ende);
  };
  const anfang = wertZuEingabe(feld.art, feld.wert);
  const mit = feld.art.mit ?? {};

  switch (feld.art.art) {
    case "schalter": {
      const box = el("input");
      box.type = "checkbox";
      box.checked = feld.wert?.wert === true;
      box.addEventListener("change", () => versuchen(String(box.checked)));
      huelle.append(box);
      break;
    }
    case "wahl": {
      const werte: string[] = mit.werte ?? [];
      if (werte.length) {
        const auswahl = el("select");
        if (feld.fehlt) auswahl.append(new Option("—", "", true, true));
        for (const w of werte) auswahl.append(new Option(w, w, false, feld.wert?.wert === w));
        auswahl.addEventListener("change", () => auswahl.value && versuchen(auswahl.value));
        huelle.append(auswahl);
        break;
      }
      huelle.append(textfeld(anfang, false, versuchen));
      break;
    }
    case "farbe": {
      if (mit.raum !== "cmyk") {
        const wahl = el("input");
        wahl.type = "color";
        wahl.value = anfang || "#e6e6e6";
        let zeitgeber: number | undefined;
        wahl.addEventListener("input", () => {
          clearTimeout(zeitgeber);
          zeitgeber = window.setTimeout(() => versuchen(wahl.value, false), 120);
        });
        wahl.addEventListener("change", () => {
          clearTimeout(zeitgeber);
          versuchen(wahl.value, true);
        });
        huelle.append(wahl);
        break;
      }
      huelle.append(textfeld(anfang, false, versuchen, "C; M; Y; K in %"));
      break;
    }
    case "datum": {
      const d = el("input");
      d.type = "date";
      d.value = anfang;
      d.addEventListener("change", () => versuchen(d.value));
      huelle.append(d);
      break;
    }
    case "inhalt":
    case "liste":
      huelle.append(textfeld(anfang, true, versuchen));
      break;
    case "text":
      huelle.append(textfeld(anfang, Boolean(mit.mehrzeilig), versuchen));
      break;
    case "tabelle":
      huelle.append(el("div", "feld-gesperrt", t("im Tabellenraster bearbeiten")));
      break;
    default: {
      const hinweise: Record<string, string> = {
        laenge: t("etwa 12 pt"),
        prozent: t("etwa 70 %"),
        punkt: t("zwei Werte; getrennt durch ;"),
      };
      huelle.append(textfeld(anfang, false, versuchen, hinweise[feld.art.art] ?? ""));
    }
  }
  huelle.append(meldung);
  return huelle;
}

/** Eingabe, die beim Bestätigen setzt: Enter (mehrzeilig: Cmd/Strg+Enter) oder Verlassen. */
function textfeld(
  anfang: string,
  mehrzeilig: boolean,
  versuchen: (text: string) => void,
  platzhalter = "",
): HTMLElement {
  const e = mehrzeilig ? el("textarea") : el("input");
  e.value = anfang;
  e.placeholder = platzhalter;
  e.spellcheck = false;
  let zuletzt = anfang;
  const bestaetigen = () => {
    if (e.value === zuletzt) return;
    zuletzt = e.value;
    versuchen(e.value);
  };
  e.addEventListener("keydown", (ev) => {
    const ke = ev as KeyboardEvent;
    if (ke.key === "Enter" && (!mehrzeilig || ke.metaKey || ke.ctrlKey)) {
      ke.preventDefault();
      bestaetigen();
    } else if (ke.key === "Escape") {
      e.value = zuletzt;
    }
  });
  e.addEventListener("blur", bestaetigen);
  return e;
}
