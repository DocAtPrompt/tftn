// Schreibansicht ohne Ablenkung: nur der Text, mittig. Kopf, Knöpfe, Gliederung, Seitenleiste,
// Vorschau und Statuszeile treten zurück; das Aussehen steht in stil.css unter `body.schreibansicht`.

/** Esc verlässt die Schreibansicht nur, wenn nichts anderes die Taste braucht. */
export function escVerlaesst(an: boolean, schonBehandelt: boolean, dialogOffen: boolean): boolean {
  return an && !schonBehandelt && !dialogOffen;
}

export function istAn(): boolean {
  return document.body.classList.contains("schreibansicht");
}

export function setzen(an: boolean) {
  document.body.classList.toggle("schreibansicht", an);
}
