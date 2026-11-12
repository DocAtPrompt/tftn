// Start der Oberfläche: erst die Sprache laden, dann main.ts. So übersetzen auch Texte, die ein
// Modul schon beim Laden ausrechnet.

import { invoke } from "@tauri-apps/api/core";
import { type OberflaecheJs, katalogSetzen, seiteUebersetzen } from "./sprache";

try {
  const o = await invoke<OberflaecheJs>("oberflaeche", { system: navigator.language });
  katalogSetzen(o.texte, o.kennung);
  document.documentElement.lang = o.kennung === "de" ? "de-AT" : o.kennung;
  seiteUebersetzen(document.body);
} catch (e) {
  console.error("Sprache der Oberfläche:", e);
}
await import("./main");
