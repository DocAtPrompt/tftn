import { describe, expect, it } from "vitest";
import { doiAusEingabe } from "./doi";

describe("DOI erkennen", () => {
  it("in üblichen Schreibweisen", () => {
    for (const e of ["10.1038/nature14539", " https://doi.org/10.1038/nature14539", "doi:10.1038/nature14539", "DOI: 10.1038/nature14539", "http://dx.doi.org/10.1038/nature14539"])
      expect(doiAusEingabe(e)).toBe("10.1038/nature14539");
    expect(doiAusEingabe("10.5281/zenodo.3509134")).toBe("10.5281/zenodo.3509134");
  });

  it("nicht in gewöhnlicher Suche", () => {
    for (const e of ["", "LeCun 2015", "10.1038", "10.12/abc", "10.1038/a b", "nature14539"]) expect(doiAusEingabe(e)).toBeNull();
  });
});
