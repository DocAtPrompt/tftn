# Testkorpus Diagramme (PLAN-M2 AP10)

Echte Universe-Pakete, gegen die die mitgelieferten Paketbeschreibungen (`pakete/*.toml`) geprüft werden.
Die Tests kopieren `packages/preview/` in ein neu angelegtes Fachartikel-Projekt; es gibt hier also weder
`tftn.toml` noch `main.typ`. Die Pakete liegen im Repo, damit Tests und CI ohne Netz laufen.

Geholt am 2026-11-09 mit `tftn-holer` von `packages.typst.org`, je die höchste Version, deren `compiler`
höchstens 0.15.1 ist:

| Paket | Version | Lizenz | braucht |
|---|---|---|---|
| cetz | 0.5.3 | LGPL-3.0-or-later | oxifmt 1.0.0 |
| fletcher | 0.5.8 | MIT | cetz 0.3.4, oxifmt 0.2.1 |
| cetz-plot | 0.1.4 | LGPL-3.0-or-later | cetz 0.5.2, oxifmt 1.0.0 |

Die Lizenztexte stehen in den Paketordnern. Beim Aktualisieren: Paket mit dem Holer neu holen, Ordner
austauschen, `empfohlen` in `pakete/*.toml` anpassen, Tests laufen lassen.
