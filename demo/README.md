# Demo: TFTN in fünf Minuten

Beispielprojekt (Dokumenttyp „Fachartikel“, @local/tftn-fachartikel:0.3.0). In TFTN über
„Öffnen …“ diesen Ordner wählen. Alle Personen, Daten und Quellen sind erfunden.

Example project. In TFTN, choose this folder via “Open …”. Everything in it is made up.

## Ohne TFTN bauen (Gegenprobe)

Mit typst 0.15.1 ergibt diese Zeile im Projektordner dasselbe PDF wie TFTN:

```bash
mkdir -p _renders
SOURCE_DATE_EPOCH=1794441600 typst compile --root . --package-path packages --package-cache-path packages --ignore-system-fonts --ignore-embedded-fonts --font-path fonts main.typ _renders/main.pdf
```

## Herkunft

- `packages/local/`: Dokumenttyp und seine Abhängigkeiten, kopiert aus TFTN `types/`.
- `fonts/`: frei lizenzierte Schriften, Lizenzen in `fonts/LIZENZEN.txt`.
