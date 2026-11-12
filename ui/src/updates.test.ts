import { describe, expect, it } from "vitest";
import { type UpdatesJs, updatesText } from "./einstellungen";

const basis: UpdatesJs = {
  tftn_aktuell: "0.1.0",
  tftn_neu: null,
  tftn_link: "https://github.com/DocAtPrompt/tftn/releases/latest",
  typst_aktuell: "0.15.1",
  typst_neu: null,
  hinweise: [],
};

describe("updatesText", () => {
  it("alles_aktuell", () => {
    expect(updatesText(basis)).toEqual(["TFTN 0.1.0 ist aktuell.", "typst 0.15.1 ist aktuell."]);
  });
  it("neue_versionen", () => {
    const z = updatesText({ ...basis, tftn_neu: "0.2.0", typst_neu: "0.16.0" });
    expect(z[0]).toContain("TFTN 0.2.0 ist erschienen");
    expect(z[0]).toContain("releases/latest");
    expect(z[1]).toContain("mit einer neuen TFTN-Version");
  });
  it("tftn_nicht_veroeffentlicht", () => {
    const z = updatesText({ ...basis, hinweise: ["TFTN: keine öffentlich veröffentlichte Version gefunden"] });
    expect(z).toEqual(["typst 0.15.1 ist aktuell.", "TFTN: keine öffentlich veröffentlichte Version gefunden"]);
  });
});
