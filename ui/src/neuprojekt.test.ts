import { describe, expect, it } from "vitest";
import { type TypJs, namePruefen, typenOrdnen, wechselNachSpeichern, zielPfad, entwurfHatText, brauchtListe, sperre } from "./neuprojekt";

describe("name_pruefen", () => {
  it("gleiche Fälle wie Rust name_pruefen", () => {
    for (const schlecht of ["", "a/b", "a\\b", "..", ".x", "  "]) {
      const m = namePruefen(schlecht);
      expect(m, schlecht).not.toBeNull();
      expect(m!.startsWith("Name")).toBe(true);
    }
    expect(namePruefen("Mein Artikel 2026")).toBeNull();
    // Abschlussprüfung: unter Windows unzulässig bzw. laufwerksrelativ (wie Rust).
    for (const schlecht of ["C:Artikel", "a<b", "a>b", 'a"b', "a|b", "a?b", "a*b", "CON", "nul", "Com1", "x.", "x\u0007"]) {
      expect(namePruefen(schlecht), schlecht).not.toBeNull();
    }
  });
});

describe("ziel_pfad", () => {
  it("mac und windows", () => {
    expect(zielPfad("/Users/x/Artikel", "Neu")).toBe("/Users/x/Artikel/Neu");
    expect(zielPfad("/Users/x/Artikel/", "Neu")).toBe("/Users/x/Artikel/Neu");
    expect(zielPfad("C:\\Tresor\\Büro", "Neu")).toBe("C:\\Tresor\\Büro\\Neu");
    expect(zielPfad("C:\\Tresor\\Büro\\", " Neu ")).toBe("C:\\Tresor\\Büro\\Neu");
  });
});

describe("typen_ordnen", () => {
  it("eigene zuerst, dann nach titel", () => {
    const t = (titel: string, herkunft: "eigen" | "tftn"): TypJs => ({ name: titel, version: "1.0.0", titel, beschreibung: null, herkunft });
    const o = typenOrdnen([t("Zeta", "tftn"), t("Brief", "tftn"), t("Mein", "eigen"), t("Anderer", "eigen")]);
    expect(o.map((x) => x.titel)).toEqual(["Anderer", "Mein", "Brief", "Zeta"]);
  });
});

describe("wechsel_speichert_zuerst", () => {
  it("ohne speichern kein wechsel", async () => {
    const reihe: string[] = [];
    const ok = await wechselNachSpeichern(
      async () => {
        reihe.push("speichern");
        return false;
      },
      async () => {
        reihe.push("wechseln");
      },
    );
    expect(ok).toBe(false);
    expect(reihe).toEqual(["speichern"]);
  });
  it("erst speichern, dann wechseln", async () => {
    const reihe: string[] = [];
    const ok = await wechselNachSpeichern(
      async () => {
        reihe.push("speichern");
        return true;
      },
      async () => {
        reihe.push("wechseln");
      },
    );
    expect(ok).toBe(true);
    expect(reihe).toEqual(["speichern", "wechseln"]);
  });
});

describe("entwurfHatText", () => {
  it("nur_die_sprachzeile_ist_leer", () => {
    expect(entwurfHatText('#set text(lang: "de", region: "AT")\n')).toBe(false);
    expect(entwurfHatText('#import "@local/tftn-leer:0.2.0": *\n#set text(lang: "de", region: "AT")\n')).toBe(false);
    expect(entwurfHatText('#import "@local/tftn-leer:0.2.0": *\n#set page(paper: "a5")\nText')).toBe(true);
    expect(entwurfHatText('#set text(lang: "de", region: "AT")\n\n  \n')).toBe(false);
    expect(entwurfHatText("")).toBe(false);
    expect(entwurfHatText('#set text(lang: "de", region: "AT")\nHallo')).toBe(true);
  });
});

describe("brauchtListe", () => {
  it("nennt_was_die_vorlage_braucht", () => {
    expect(brauchtListe(["Titel", "Literatur"])).toEqual(["Titel", "Literatur"]);
  });
  it("ohne_angaben_nichts", () => {
    expect(brauchtListe([])).toEqual(["Nichts weiter – einfach losschreiben."]);
    expect(brauchtListe(undefined)).toEqual(["Nichts weiter – einfach losschreiben."]);
  });
});

describe("einzeln", () => {
  it("ignoriert Aufrufe, solange einer läuft, und gibt danach wieder frei", async () => {
    const einzeln = sperre();
    let fertig!: () => void;
    let aufrufe = 0;
    const langsam = () => {
      aufrufe++;
      return new Promise<void>((r) => (fertig = r));
    };
    const erster = einzeln(langsam);
    expect(await einzeln(langsam)).toBe(false);
    expect(await einzeln(async () => void aufrufe++)).toBe(false);
    expect(aufrufe).toBe(1);
    fertig();
    expect(await erster).toBe(true);
    expect(await einzeln(async () => void aufrufe++)).toBe(true);
    expect(aufrufe).toBe(2);
  });

  it("meldet, wenn ein Aufruf wegen der Sperre nicht läuft", async () => {
    let besetzt = 0;
    const einzeln = sperre(() => besetzt++);
    let fertig!: () => void;
    const erster = einzeln(() => new Promise<void>((r) => (fertig = r)));
    expect(await einzeln(async () => {})).toBe(false);
    expect(besetzt).toBe(1);
    fertig();
    await erster;
    expect(await einzeln(async () => {})).toBe(true);
    expect(besetzt).toBe(1);
  });

  it("gibt auch nach einem Fehler frei", async () => {
    const einzeln = sperre();
    await expect(einzeln(async () => Promise.reject(new Error("x")))).rejects.toThrow("x");
    expect(await einzeln(async () => {})).toBe(true);
  });
});
