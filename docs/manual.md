# TFTN – Manual

*Deutsche Fassung: [handbuch.md](handbuch.md)*

TFTN (“Typst for the Noobs”) is a writing app for [Typst](https://typst.app) on macOS and Windows.
You write on the left; the finished page appears on the right as you type. TFTN works fully
offline; only when you explicitly fetch something (a package, a bibliography entry, the update
check) does a request go out, and only to a fixed address.

Your text stays plain Typst. Every document can be compiled without TFTN using the typst CLI and
yields the same PDF.

The interface is available in English and German. The document types that ship with TFTN come in
both languages too: blank document, letter, invoice, medical report (doctor's letter), report,
journal article, book, poster and CV.

## Contents

1. [Getting started](#1-getting-started)
2. [The window](#2-the-window)
3. [Writing](#3-writing)
4. [Changing values without code](#4-changing-values-without-code)
5. [Page, front matter and references](#5-page-front-matter-and-references)
6. [Packages from Typst Universe](#6-packages-from-typst-universe)
7. [Import and export](#7-import-and-export)
8. [Saving and versions](#8-saving-and-versions)
9. [Spelling, counting, error messages](#9-spelling-counting-error-messages)
10. [Settings and keyboard shortcuts](#10-settings-and-keyboard-shortcuts)
11. [Network, updates, privacy](#11-network-updates-privacy)
12. [Command line](#12-command-line)
13. [AI assistants (MCP)](#13-ai-assistants-mcp)

## 1. Getting started

**Install (macOS):** drag `TFTN.app` from the release page into `/Applications`. The app is not
notarized: on first launch, right-click → “Open”. The matching typst CLI is bundled.

**Windows:** run `TFTN-<version>-windows-x64-setup.exe` from the releases page; it installs for the
current user without admin rights. The installer is not signed: if SmartScreen warns, choose
“More info” → “Run anyway”.

**Try it:** use “Open” → “Open project …” and choose the `demo/` folder from the source. The sample article shows
equations, a figure, a table, footnotes, cross-references and citations. Feel free to change
anything.

**A new document:** TFTN starts with an empty draft. Just start writing; Cmd/Ctrl+S gives it a
name and saves it in your Documents folder. “New” → “From template …” instead creates a document of a document
type, for example a journal article with front matter and bibliography. Both when creating from a
template and when saving for the first time with Cmd/Ctrl+S there is a checkbox “Embed fonts”: with
it the document carries the fonts it uses in `fonts/` (usually 1 to 3 MB) and looks the same on every
computer – recommended for sharing and archiving. Without it, it uses TFTN's fonts; you can embed them
later in the “Page” panel.

**Language of a new document:** new documents and the empty draft use the interface language –
with an English interface you get an English template (hyphenation, spell checking, and labels such
as “Abstract”, “Keywords” and “References”). In every document type with a form you can switch
later: “Document” tab → “Language”.

**Medical reports** contain health data. Keep such documents outside any Git repository; TFTN
shows a warning in the “Document” tab if the folder lies inside one. Everything stays on your
computer – TFTN never sends text anywhere.

**Invoices** compute items, VAT and the total themselves (each line rounded to the cent). You
assign the invoice number yourself, consecutive and without gaps; TFTN does not count, not even
across several computers. Whether the invoice contains every detail required for you is yours to
check.

**What is a project?** A folder with `tftn.toml`, the main file (usually `main.typ`), and for
templates also `dokument.toml` (front matter), the bibliography file, the fonts (`fonts/`) and the
packages in use (`packages/`). Everything the document needs lives in the folder – you can copy it,
sync it and keep working on another computer without a network.

## 2. The window

- **Header:** the project on the left (hover for the full path); on the right “New” (blank document,
  from template, import), “Open”, “Versions”, “Export” and the gear (settings). In a narrow window
  only the icons remain.
- **Open** is a menu: “Open project …” chooses a folder with `tftn.toml` (Cmd/Ctrl+O goes straight
  there); below it are the recently opened projects, up to ten, newest first. Hover for the full
  path; if two projects share a name, the folder above follows quietly. A project that is out of
  reach for the moment (say, a vault that is not mounted) is left out of the menu but stays
  remembered. “Clear list” forgets the list; the projects themselves stay. The list applies per
  computer.
- **Messages** appear in a band below the header. Short notices disappear by themselves; errors and
  questions stay until you close them with × or answer them.
- **Panels such as Export, New or Versions** close with ×, Esc or a click outside. Once you have
  typed something into a dialog (a project name, say), only × or Esc closes it.
- **Editor** (left): Typst source with highlighting. Above it: “Block”, “Citation”, “Outline”,
  “Page” and, as icons, sidebar and focus view.
- **Preview** (right): the finished pages. Clicking in the preview jumps to the matching place in
  the text; the preview follows the cursor. Right-click opens a menu with “Edit …” and “Show in
  code”.
  On the right of the status bar: “Width” (pages as wide as the column), “Whole page”, − and +, and
  the percentage (click it for actual size). Cmd/Ctrl + mouse wheel or a two-finger pinch zooms around
  the pointer; a page wider than the column gets a horizontal scrollbar.
- **Divider** between text and preview: drag it to share the width (both sides keep some room);
  double-click restores half and half. Reached with the Tab key, the arrow keys move it. The split
  applies per computer.
- **Sidebar** (Cmd/Ctrl+Shift+P): panels “Here”, “Insert”, “Equation”, “Page”, “Document”,
  “Packages”. The tabs at the top are icons; only the open panel shows its name (not even that in a
  narrow sidebar), the others show name and purpose when you hover over them. “Insert” is the block
  list with a search field: a click inserts the block at the cursor. Drag its edge to resize it.
- **Outline** (“Outline” button): the headings next to the text; the second tab lists the comments.
- **Status bar:** words and characters, document language, save state (“saved” or “unsaved”), the
  number of network accesses once there has been one (chapter 11), “MCP on” while MCP is switched on
  (chapter 13), and how long Typst took for its last run (“Preview: 87 ms”, from one second on e.g.
  “Preview: 1.2 s”).

## 3. Writing

**Blocks:** typing “/” on an empty line opens the block list (keep typing to filter, Enter inserts,
Esc closes). Cmd/Ctrl+Shift+B or “Block” open it with a search field; selected text moves into
the block. Afterwards Tab jumps from field to field, Shift+Tab back, Esc finishes. Cmd/Ctrl+Z undoes
the whole block in one step.

**Completion:** Ctrl+Space (on the Mac also ⌥`) suggests functions, parameters, labels and citation
keys. Nothing pops up while you just type.

**Formatting:** Cmd/Ctrl+B bold, Cmd/Ctrl+I italic, Cmd/Ctrl+K link. Cmd/Ctrl+Shift+. and +, move a
heading one level down or up. Footnote, figure and equation: Cmd/Ctrl+Shift+F, +A, +E.

**Equations:** the “Equation” panel (Cmd/Ctrl+Shift+X) has an input field with preview and a
palette (fraction, root, sum, matrix, cases, Greek letters, symbols). Tab jumps to the next
placeholder. With the cursor inside an equation, “Apply” (Cmd/Ctrl+Enter) changes it in one step;
otherwise “Insert” adds a new one.

**Images:** drag an image from Finder or Explorer into the text, or paste it (for example a
screenshot). TFTN stores it under `bilder/` and inserts a figure at 80 % width with the cursor in
the caption. Formats: png, jpg, gif, svg, webp, pdf (up to 50 MB).

**Citing:** Cmd/Ctrl+Shift+L or “Citation” searches the bibliography file. To change an
existing citation, put the cursor in it and open the sidebar (or right-click it in the preview →
“Edit …”); there you can change the page reference and the source.

**References from a DOI:** type a DOI in the citation dialog (`10.1038/nature14539`, `doi:…` or
`https://doi.org/…`). “Look up DOI and cite” fetches the entry from Crossref or DataCite, adds it to
the bibliography file with a readable key (`lecun2015`) and inserts the citation. If the document
has no bibliography yet, one is added at the end. Only the DOI is sent.

**Managing references:** “Manage bibliography …” in the citation dialog or in the “Document” pane shows
the entries of the bibliography file: on the left the list with search (entries not cited are
marked), on the right a form with the usual fields per type (journal article, book, collection, …).
People go one per line (“Last name, first name”). For a new entry TFTN suggests a key from the first
person and the year. “Save” (Cmd/Ctrl+S) writes only this entry to the `.bib`; the rest of the file
stays as it is. “Cite” inserts `@key` at the cursor. Values without braces such as `2015` or
abbreviations from `@string` are shown in monospace and written unchanged. YAML bibliographies
(Hayagriva) cannot be edited here.

**Outline:** the headings are listed on the left; a click jumps there. Dragging moves a section with
its subsections, as does Alt+↑/↓ on an entry. Every move is one undo step.

**Comments:** anything after `//` (or between `/*` and `*/`) is a note that is not printed. The
“Comments” tab in the outline lists them by section; a click jumps there. Several `//` lines in a row
count as one comment.

**Focus view** (“Focus view” button or Cmd/Ctrl+Shift+Enter): just the text, centered and a little
larger. Everything else steps back; the preview keeps updating in the background. Esc, the same
shortcut or the faint button at the top right leave it. For the whole screen, use the system’s full
screen mode.

## 4. Changing values without code

Values in the text – a width, a color, a date, an alignment – can be changed without knowing Typst.
Every change is checked and is one undo step.

- **Markers:** hold ⌥ or Alt alone for half a second (or press Cmd/Ctrl+Shift+M). A small marker
  appears after every adjustable value, “+ …” where a value is missing. Clicking it opens the
  matching editor (color picker, number with unit, choice …).
- **Sidebar, “Here” panel:** shows what the cursor is in, with all values as a form, buttons for
  heading, list and equation, and – inside a table – the **table grid**: insert, delete and move rows
  and columns, merge and split cells; double-click edits the text.
- **Right-click in the preview:** “Edit …”, the values directly, “Show in code”.

## 5. Page, front matter and references

**Page** (“Page” button): paper size, landscape, two-sided, columns, margins, font, size, line
spacing, justification, numbered headings, language, page numbers, header and footer (with `{seite}`
for the page and `{kapitel}` for the chapter). Each setting is a `#set` line at the top of the text.
Templates such as the journal article set this themselves. “Embed fonts” copies the fonts in use into
the project.

**More fonts:** “More fonts …” below the font choice lists the fonts installed on the computer, each
with a sample line, the number of styles and the size. “Font file …” (Mac) or a path (Windows) adds an
`.otf` or `.ttf` file; bold and italic from the same folder come along. If the font lacks characters of
the document language, such as “ ” or € (or ß and umlauts in German), this is shown right away
(“Missing: €”); such fonts are hidden at first. “Use font” copies the whole family into `fonts/` and
sets it as the document font. System fonts never enter typesetting directly: preview, export and the
cross-check line see the same files in `fonts/`, even on a computer where the font is not installed.
If a font’s license forbids embedding it in PDFs, TFTN refuses it. Use fonts that came with the system
(such as Apple’s or Microsoft’s) only for your own documents, and do not pass `fonts/` on.

**Missing characters:** If a font lacks a character, Typst silently takes it from another font or
draws an empty box. TFTN reports both as a warning that jumps to the spot: “The font … lacks the
character …; it is taken from …” or “… shows up as an empty box”.

**Front matter** (“Document” panel, Cmd/Ctrl+Shift+O): a form defined by the document type – title,
authors, date, abstract, keywords, bibliography files and citation style (a built-in name such as
`ieee` or a `.csl` file in the project). Each value is checked before it is written to
`dokument.toml`; the text in `main.typ` is never touched. Clicking the front matter in the preview
opens the panel at the matching field. “Edit as text” shows `dokument.toml` as text, for fields the
form doesn't know or to repair a broken file: “Apply” (Cmd/Ctrl+Enter) only writes valid TOML, and
“Undo” takes it back.

**Updating the document type:** when a newer version of the document type is available, the
“Document” panel shows it. “Compare …” shows the changed pages before and after, “Apply” switches.

**Your own document types** go into the `typen/` folder of the settings folder and take precedence
over the bundled ones. A document type is a Typst package with a `[tool.tftn]` section in
`typst.toml`.

**Creating your own document type**, such as a letter with your own sender: Settings → “Custom
document type from”, choose an existing type, enter a name, “Create”. TFTN copies it to
`typen/<name>/` and shows the folder. There, `vorlage/dokument.toml` holds the defaults of every new
document (English template: `vorlage-en/`) and `lib.typ` the layout. The new type appears under
“New” → “From template …”; existing documents stay as they are. The copy brings its own fonts and
travels with the settings folder to other computers.

## 6. Packages from Typst Universe

The “Packages” panel (Cmd/Ctrl+Shift+Y) searches the package index by name, description and
keyword. “Add” fetches the package and its dependencies, stores them under `packages/preview/` in
the project, records their checksums in `tftn.toml` and inserts the `#import` if you like. From then
on the document builds offline, also on another computer. “Missing in the project” lists packages that a
typed import needs. “Remove” and “Newer version …” (with comparison) are next to each package.

**Packages with helpers:** for cetz (drawings), fletcher (flow diagrams) and cetz-plot (charts from
data) TFTN ships its own blocks; their points, colors and data series can be changed through markers.
Your own package descriptions (TOML) in the `beschreibungen/` folder of the settings folder take
effect immediately.

## 7. Import and export

**Import** (“New” → “Import …”, requires [Pandoc](https://pandoc.org) 3.1.2 or later): Word (.docx),
OpenDocument (.odt), Markdown, HTML, EPUB, LaTeX and RTF become a new document. Headings, lists,
tables, footnotes, equations and images come along; layout, comments and tracked changes do not.
The original stays untouched. Pandoc runs sandboxed and fetches nothing from the network. Install:
Mac `brew install pandoc`, Windows `winget install --id JohnMacFarlane.Pandoc`.

**Paste with conversion** (also requires Pandoc): when you paste Markdown with Cmd/Ctrl+V (such as
`# Title`, `**bold**`, `[link](…)`, tables) or formatted text from a browser or Word, it first appears
unchanged, and the notification bar offers “Convert to Typst”. One click replaces the pasted text with
Typst; one undo brings the pasted text back. Once you keep typing, the offer goes away; if you close
the bar, the preview shows the errors in the pasted text. You can also convert later: select Markdown
in the editor, right-click the selection, “Convert to Typst”. Images from the internet are not
downloaded.

**PDF** (“Export”): the profiles come from the document type; the bundled ones have “PDF” and
“Archive PDF (long-term, PDF/A-2b)” (`vorschau` and `archiv` in the manifest). The archive PDF is
just the file; what it takes to typeset it again is in the archive package (below). The PDF is produced by the bundled typst CLI and is reproducible.
Each profile has two buttons:

- **Test build** saves the PDF in `_renders/`. That folder is not synced; use it to look at or print
  the document in between.
- **Finalize …** (a second click confirms) is the final version: the timestamp in `tftn.toml` jumps
  to now (it sets the date in the PDF), the PDF goes to `export/`, and next to it lies
  `<name>.gegenprobe.txt` with the typst command and the PDF's SHA-256.

**Archive package** (“Export” → “Archive package (zip)” → “Archive”): one zip for your records, for example for medical
reports you must keep for years. It is saved as `export/<document folder>-archive-<date>.zip` in the
document folder (a second one on the same day replaces the first) and contains:

- the PDF as PDF/A (the standard of the “archiv” profile, otherwise PDF/A-2b),
- the source with data, images and bibliography,
- all fonts used in `fonts/` and the packages in `packages/`,
- `README.txt` with the typst version, the command line for macOS/Linux and Windows, and the
  SHA-256 of the PDF.

`_lokal/`, `_renders/`, earlier exports in `export/` and hidden folders such as `.fassungen/` stay
out. The PDF is made in a copy of the document from nothing but what goes into the zip, without
TFTN's own fonts – so anything missing shows up now, not in ten years. With the same typst version
the unpacked folder gives the same PDF, byte for byte; the SHA-256 proves it. The document itself
is not changed.

**Word (.docx) for proofreading** (“Export”, requires Pandoc): title, headings, lists, tables, figures,
real footnotes, equations, citations and bibliography come along; the page layout does not. The file
is saved as `<document name>.docx` in the document folder and overwritten by the next export.

**Series** (“Export” → “Series …”): one PDF per record, e.g. for mail merge and invoices. The records
are either a folder of `.toml` files (one document per file, the PDF is named after the file) or one
`.toml` with `[[serie]]` sections. A record holds only what changes; everything else – sender, bank,
closing – comes from the Document pane of the project. Groups such as `[absender]` are merged, all other
values are replaced. The field `datei` names the PDF; without it the PDFs are numbered
(`invoices-001.pdf`).

```toml
[[serie]]
datei = "2026-014 Miller"
nummer = "2026-014"
empfaenger = ["Mr", "Dr Hans Miller", "5 High Street", "Graz 8010"]
anrede = "Dear Dr Miller,"
posten = [{ text = "Consulting", menge = 3, einheit = "h", preis = 120 }]
```

Letter and invoice have a “Salutation” field for this. To swap more of the text, write
`#daten.fieldname` there (new letters and invoices have `#let daten = …` at the top for this). The PDFs are written to the data folder and replace files of the same name.
Each one is made with the typst CLI like any export; the dialog shows the cross-check commands. For
this the main file reads its data with `toml(sys.inputs.at("daten", default: "dokument.toml"))`; for
older documents “Adjust main file” changes that one spot. A faulty record does not stop the others and
is reported at the end.

**Everything in one PDF:** “Output” offers “one PDF per record”, “all in one PDF (one print run)” and
“both”. The combined PDF is named after the data with `-all` (`november-all.pdf`) and is also written
to the data folder. With “double-sided printing” a blank page follows a document with an odd number of
pages, so that in duplex printing every letter starts on a front page. The combined PDF is made with
the typst CLI too: `_lokal/serie/alle.typ` embeds the finished PDFs page by page, and the text stays
text. With “all in one PDF” only, the single PDFs stay in `_lokal/serie/pdf/`. Faulty records are
left out of the combined PDF and reported.

On the command line: `tftn serie <project> --daten <folder or file> [--ausgabe einzeln|gesamt|beides] [--beidseitig]`.

## 8. Saving and versions

Every document saves itself one second after the last change, also when closing the window and
quitting. Cmd/Ctrl+S is only needed to name a new document.

**Versions** shows earlier states: at most every ten minutes TFTN keeps a version in `.fassungen/`
in the document folder (all from the last day, before that one per day, for 60 days). Restoring
replaces the text in one step; Cmd/Ctrl+Z brings the previous text back.

## 9. Spelling, counting, error messages

**Spelling** follows the document language: unknown words in running text are underlined in red;
commands, equations, code and references never are. Right-click: suggestions, “Add to dictionary”
(stored in `woerter.txt` in the settings folder) or “Ignore”. Dictionaries: German (Germany, Austria,
Switzerland), English (US, UK), French, Italian, Spanish.

**Hyphenation:** Typst hyphenates at the end of a line by itself, following the rules of the document
language. If it breaks a word wrongly (technical terms, names, “the-rapist”), add the word to
`trennungen.txt` in the project folder – journal article, book and report have this file. One word
per line, allowed break points marked with `-`; a word without `-` is never hyphenated. Lines starting
with `#` are comments.

```
the-ra-pist
Sozial-versicherungs-träger
Typst
```

The word is also found inside longer words (compounds, inflected forms); case matters. The change
shows in the preview right away.

**Counting:** the status bar shows words and characters, “Selection: … of …” for a selection, and
the outline shows the count per section. Captions and footnotes count; commands, equations, code and
comments do not.

**Error messages** appear at the error in the text and at the top of the preview, with a tip on how
to fix it. While an error persists, the preview keeps showing the last good state, faded and over
red diagonal stripes.

## 10. Settings and keyboard shortcuts

The gear (settings, Cmd/Ctrl+,) lists every action with its shortcut: click a row and press the new
shortcut; Delete means “no shortcut”, “Default” resets it. Conflicts with system and application
shortcuts are shown for Mac **and** Windows.

| Action | Default (Mod = Cmd or Ctrl) |
|---|---|
| Insert block | Mod+Shift+B, on an empty line also “/” |
| Insert citation | Mod+Shift+L |
| Complete | Ctrl+Space, on the Mac also ⌥` |
| Settings | Mod+, |
| Values at this position (markers) | Mod+Shift+M or hold ⌥/Alt |
| Sidebar | Mod+Shift+P |
| Sidebar: Document / Packages / Equation | Mod+Shift+O / +Y / +X |
| Distraction-free focus view | Mod+Shift+Enter |
| Save, New, Open | Mod+S, Mod+N, Mod+O |
| Bold, Italic, Link | Mod+B, Mod+I, Mod+K |
| Heading down / up | Mod+Shift+. / Mod+Shift+, |
| Footnote, Figure, Equation | Mod+Shift+F, +A, +E |

**Folder for your own customisations:** shortcuts (`tasten.toml`), your dictionary, your own
document types (`typen/`), skins (`skins/`), language files (`sprachen/`) and package descriptions
live in one folder, at the bottom of the settings; “Show in folder” opens it. By default it is a
folder on this computer. If you use several computers, you can choose another folder, for example
inside a sync folder, and all computers share the same customisations. Changes made outside TFTN
take effect immediately.

**Interface language:** English and German are included; “Same as system” is the default. “Another language …” creates a template for translating: German on the left, your translation on the right;
save it as `<code>.toml` (for example `fr.toml`) in the `sprachen/` folder – no new build needed.

**Appearance (skins):** Light, Dark, Paper and High contrast are included; “Same as system” follows
the system's light or dark mode. A skin sets the interface colours, the syntax highlighting colours
and the editor's font, size and line spacing; the preview stays white like the printed document.
“Custom skin …” creates a template in the `skins/` folder; after editing it, choose it under
“Appearance”. Values that don't fit are listed in the dialog and skipped. The choice applies per
computer.

**Interface size:** Small, Normal, Large or Extra large scales bars, buttons and dialogs right away.
The editor and preview stay as they are: the skin sets the editor font, and the preview fits its
width. The choice applies per computer.

## 11. Network, updates, privacy

TFTN only reads and writes your files. Only the bundled helper `tftn-holer` goes online, only when
you click, and only to fixed addresses:

| Purpose | Address |
|---|---|
| Packages | `packages.typst.org` |
| References from a DOI | `api.crossref.org`, `api.datacite.org` (only the DOI is sent) |
| Check for updates | `api.github.com` (latest version of TFTN and typst) |

While a request runs, the status bar shows it. **“Fully offline”** in the settings blocks all
network access. TFTN never installs anything by itself: you download a new version from the release
page.

With **MCP** (section 13) an AI assistant reads the open document. TFTN itself does not go online for
this. Where the content goes depends on the assistant: a local model (in LM Studio, for example) keeps
everything on your computer, a cloud assistant sends it to its provider. MCP stays off until you turn
it on.

**Network log – provably offline.** TFTN keeps track of every network access since it started.
Normally there is none. If TFTN did go online after a click, the status bar shows the count
(“1 network access”); clicking it, or **Settings → Network log …**, opens the window:

- at the top, one line saying whether and how often TFTN went online since the start;
- for each access the time, purpose (for example “Fetch package @preview/cetz:0.3.4 with its
  dependencies”), remote host, trigger (which click) and result, and below it every single connection
  with address, response and amount of data, as reported by the downloader itself;
- whether “Fully offline” is on, and how often AI programs accessed TFTN via MCP (counted per tool,
  plus rejected attempts). MCP is not a network access by TFTN – the connection stays on the computer
  (`127.0.0.1`) – but it passes text to another program, so it is listed as well.

The log lives in memory only and ends with the program; it is never stored.

**Data protection sheet.** “Data protection sheet (PDF) …” in the network log typesets one page of PDF
for the record of processing activities or the data protection officer: what TFTN does on the
computer, which network accesses are possible at all (purpose, remote host, trigger, what is
transmitted) and which happened in this session – normally “No network access since TFTN started”.
The sheet opens in the system's PDF viewer; print or save it from there. It is typeset with the
bundled typst CLI, without network.

## 12. Command line

The `tftn` program (built from source) works without the window:

| Command | Effect |
|---|---|
| `tftn neu --typ tftn-fachartikel <folder> [--sprache en]` | create a project from a document type (`--sprache en`: English template) |
| `tftn check <folder>` | check, report errors and missing packages |
| `tftn build <folder> [-o target]` | PDF via the typst CLI (into `_renders/`) |
| `tftn build <folder> --png [--ppi 144]` | page images instead of a PDF |
| `tftn export <folder> [--profil archiv] [--abnehmen]` | export profile of the document type |
| `tftn archiv <folder>` | archive package in `export/` (PDF/A, source, fonts, packages, instructions) |
| `tftn gegenprobe <folder>` | print the equivalent `typst compile` command |
| `tftn serie <folder> --daten <source> [--ausgabe einzeln\|gesamt\|beides] [--beidseitig] [--anpassen]` | one PDF per record or all in one (series) |
| `tftn katalog <folder>` | list the project’s blocks |
| `tftn typ-pruefen <folder>` | check all blocks of a document type |

`tftn gegenprobe` shows how to produce the same PDF without TFTN – your document never depends on
TFTN.

## 13. AI assistants (MCP)

With MCP (Model Context Protocol) an AI assistant can work in the open document – a local model just as well as a cloud
service such as Claude: read
the source, change passages, fill in the form of the Document pane, look at preview pages, create a new document from a type or open another
one. Every change appears in the editor and the preview right away and can be undone.

**Turning it on:** In the settings, tick “MCP: AI assistants … may read and write in the open
document”. While MCP is on, “MCP on” shows in the status bar; clicking it turns MCP off. While an assistant works, the dot
pulses and for a few seconds the badge says what it does (e.g. “MCP: text changed”); the tooltip
names the last access with its time. The badge stays visible in the focus view too. The choice is
per computer and survives a restart.

**Adding it to the assistant:** Below the checkbox is the line for Claude Code
(`claude mcp add tftn -- "<path to TFTN>" --mcp`); click “Copy” and run it in a terminal. Other
assistants, including ones running a local model, take the TFTN program with the argument `--mcp` as
the command; the usual `mcpServers` entry (Claude Desktop, LM Studio and others) looks like this:

```json
{ "mcpServers": { "tftn": { "command": "/Applications/TFTN.app/Contents/MacOS/tftn-app", "args": ["--mcp"] } } }
```

The assistant talks to the running app over a connection that exists only on this computer and needs
a random key. If TFTN is closed or MCP is off, it gets a message saying so.

**Turning it off when switching documents:** If MCP should apply to one document only, tick “Turn MCP
off when another document is opened”. When you open another document yourself, MCP turns off. When
the assistant opens or creates one, MCP pauses and TFTN asks whether it should stay active (“Keep
active” or “Turn MCP off”); until then TFTN refuses its calls.

| Tool | Effect |
|---|---|
| `status` | project, revision, saved?, result of the last Typst run |
| `quelltext_lesen` | source of the main file as it is in the editor |
| `text_ersetzen` | replace one passage literally (or every occurrence) |
| `quelltext_setzen` | replace the whole source |
| `auswahl_lesen` | what you have selected and where the cursor is |
| `gliederung_lesen` | headings and comments, including included chapter files |
| `dateien_auflisten` | files in the project folder |
| `datei_lesen` | a text file of the project (chapter, `.bib`, `trennungen.txt` …) |
| `datei_ersetzen` | replace a passage in such a file |
| `datei_schreiben` | write such a file as a whole or create it |
| `literatur_lesen` | entries of the bibliography files, cited or not |
| `literatur_eintrag_speichern` | create or change one entry in the `.bib` (only that one) |
| `fassung_sichern` | keep the current state as a version |
| `rechtschreibung_pruefen` | unknown words with line and suggestions |
| `seite_ansehen` | one preview page as an image |
| `formular_lesen` | fields and values of the form (title, authors, date …) |
| `formular_setzen` | set several fields at once, checked as in the pane; one undo step |
| `typen_auflisten` | document types for a new document |
| `dokument_anlegen` | create a new project from a type and open it |
| `dokument_oeffnen` | open another project (the open one is saved first) |

**Limits:** the assistant only reaches files inside the project folder, not hidden folders, `_lokal/`
or `packages/`. It may only write text files; it changes the main file through the editor (with
undo) and the form data through the form. When it changes another file, such as a chapter, undo in the
editor does not apply – so TFTN first keeps the old content in `.fassungen/dateien/` (bibliography too). The assistant
cannot delete files or bibliography entries.
