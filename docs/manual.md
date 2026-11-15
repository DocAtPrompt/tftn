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

**Try it:** use “Open” and choose the `demo/` folder from the source. The sample article shows
equations, a figure, a table, footnotes, cross-references and citations. Feel free to change
anything.

**A new document:** TFTN starts with an empty draft. Just start writing; Cmd/Ctrl+S gives it a
name and saves it in your Documents folder. “New” → “From template …” instead creates a document of a document
type, for example a journal article with front matter and bibliography.

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
  from template, import), “Open”, “Versions”, “Export” and, as icons, sidebar, focus view and
  settings (gear). In a narrow window only the icons remain.
- **Messages** appear in a band below the header. Short notices disappear by themselves; errors and
  questions stay until you close them with × or answer them.
- **Panels such as Export, New or Versions** close with ×, Esc or a click outside. Once you have
  typed something into a dialog (a project name, say), only × or Esc closes it.
- **Editor** (left): Typst source with highlighting. Above it: “Block”, “Citation”, “Outline” and
  “Page”.
- **Preview** (right): the finished pages. Clicking in the preview jumps to the matching place in
  the text; the preview follows the cursor. Right-click opens a menu with “Edit …” and “Show in
  code”.
- **Sidebar** (Cmd/Ctrl+Shift+P): panels “Here”, “Insert”, “Equation”, “Page”, “Document”,
  “Packages”. Drag its edge to resize it.
- **Outline** (“Outline” button): the headings next to the text.
- **Status bar:** words and characters, document language, save state, network activity.

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

**Outline:** the headings are listed on the left; a click jumps there. Dragging moves a section with
its subsections, as does Alt+↑/↓ on an entry. Every move is one undo step.

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

**PDF** (“Export”): the profiles come from the document type, for the journal article “vorschau”
(preview) and “archiv” (PDF/A-2b). The PDF is produced by the bundled typst CLI and is reproducible.

**Word (.docx) for proofreading** (“Export”, requires Pandoc): title, headings, lists, tables, figures,
real footnotes, equations, citations and bibliography come along; the page layout does not. The file
is saved as `<document name>.docx` in the document folder and overwritten by the next export.

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

**Counting:** the status bar shows words and characters, “Selection: … of …” for a selection, and
the outline shows the count per section. Captions and footnotes count; commands, equations, code and
comments do not.

**Error messages** appear at the error in the text and at the top of the preview, with a tip on how
to fix it. While an error persists, the preview keeps showing the last good state.

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

## 12. Command line

The `tftn` program (built from source) works without the window:

| Command | Effect |
|---|---|
| `tftn neu --typ tftn-fachartikel <folder> [--sprache en]` | create a project from a document type (`--sprache en`: English template) |
| `tftn check <folder>` | check, report errors and missing packages |
| `tftn build <folder> [-o target]` | PDF via the typst CLI (into `_renders/`) |
| `tftn build <folder> --png [--ppi 144]` | page images instead of a PDF |
| `tftn export <folder> [--profil archiv] [--abnehmen]` | export profile of the document type |
| `tftn gegenprobe <folder>` | print the equivalent `typst compile` command |
| `tftn katalog <folder>` | list the project’s blocks |
| `tftn typ-pruefen <folder>` | check all blocks of a document type |

`tftn gegenprobe` shows how to produce the same PDF without TFTN – your document never depends on
TFTN.

## 13. AI assistants (MCP)

With MCP (Model Context Protocol) an AI assistant can work in the open document – a local model just as well as a cloud
service such as Claude: read
the source, change passages, look at preview pages, create a new document from a type or open another
one. Every change appears in the editor and the preview right away and can be undone.

**Turning it on:** In the settings, tick “MCP: AI assistants … may read and write in the open
document”. While MCP is on, “MCP on” shows in the status bar; clicking it turns MCP off. The choice is
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
| `seite_ansehen` | one preview page as an image |
| `typen_auflisten` | document types for a new document |
| `dokument_anlegen` | create a new project from a type and open it |
| `dokument_oeffnen` | open another project (the open one is saved first) |
