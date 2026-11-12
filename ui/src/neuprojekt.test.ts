import { describe, expect, it } from "vitest";
import { type TypJs, namePruefen, oeffnenArt, typenOrdnen, wechselNachSpeichern, zielPfad, entwurfHatText, brauchtListe } from "./neuprojekt";

describe("name_pruefen", () => {
  it("gleiche Fälle wie Rust name_pruefen", () => {
    for (const schlecht of ["", "a/b", "a\\b", "..", ".x", "  "]) {
      const m = namePruefen(schlecht);
      expect(m, schlecht).not.toBeNull();
      expect(m!.startsWith("Projektname")).toBe(true);
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

describe("oeffnen_art", () => {
  it("systemdialog am mac, pfadfeld unter windows", () => {
    expect(oeffnenArt("mac")).toBe("dialog");
    expect(oeffnenArt("win")).toBe("pfad");
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
