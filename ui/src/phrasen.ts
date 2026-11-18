// Texte, die CodeMirror selbst zeigt (Suchleiste, Gehe-zu-Zeile, Fehlerliste, Vorschläge), in der
// Sprache der Oberfläche. Schlüssel sind CodeMirrors englische Phrasen; `$` setzt CodeMirror ein.

import { t } from "./sprache";

export function phrasen(): Record<string, string> {
  return {
    Find: t("Suchen"),
    Replace: t("Ersetzen"),
    next: t("weiter"),
    previous: t("zurück"),
    all: t("alle"),
    "match case": t("Groß/klein beachten"),
    "by word": t("ganzes Wort"),
    regexp: t("regulärer Ausdruck"),
    replace: t("ersetzen"),
    "replace all": t("alle ersetzen"),
    close: t("schließen"),
    "current match": t("aktueller Treffer"),
    "on line": t("in Zeile"),
    "replaced $ matches": t("$ Treffer ersetzt"),
    "replaced match on line $": t("Treffer in Zeile $ ersetzt"),
    "Go to line": t("Gehe zu Zeile"),
    go: t("los"),
    Diagnostics: t("Meldungen"),
    "No diagnostics": t("Keine Meldungen"),
    Completions: t("Vorschläge"),
    "Control character": t("Steuerzeichen"),
    "Selection deleted": t("Auswahl gelöscht"),
    "folded code": t("eingeklappter Code"),
    unfold: t("aufklappen"),
    to: t("bis"),
  };
}
