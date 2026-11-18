# TFTN – Manual

*Deutsche Fassung: [handbuch.md](handbuch.md)*

TFTN (“Typst for the Noobs”) is a writing app for [Typst](https://typst.app) on macOS and Windows;
on Linux you build it yourself (chapter 1). You write on the left; the finished page appears on the
right as you type. TFTN works fully offline; only when you explicitly fetch something (a package, a
bibliography entry, the update check) does a request go out, and only to a fixed address.

Your text stays plain Typst. Every document can be compiled without TFTN using the typst CLI and
yields the same PDF.

<!-- Ko-fi: switch on once the name exists (replace <name>, remove the comment markers; also the
     section “Supporting TFTN” at the end).
TFTN is free and will stay free. If it makes your work easier, you can support its development on
[Ko-fi](https://ko-fi.com/<name>) – more at the end.
-->

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

**Linux:** there is no ready-made download yet; you build TFTN yourself in four commands (see
“Building on Linux” in the README): install the system packages for WebKitGTK, Rust and Node.js,
then run `npm ci`, `cargo xtask typst-holen --nur-host` and
`npx tauri build --bundles deb --config src-tauri/tauri.buendel.json` in the source folder. The
result is a `.deb` with the typst CLI and the download helper, installed with
`sudo apt install ./target/release/bundle/deb/TFTN_*.deb` (or an AppImage with `--bundles appimage`).
The first build takes 15 to 30 minutes. As on Windows, the Ctrl shortcuts apply. The Linux build is
new and has hardly been tried by hand yet.

**Try it:** use “Open” → “Open document …” and choose the `demo/` folder from the source. The sample article shows
equations, a figure, a table, footnotes, cross-references and citations. Feel free to change
anything.

**Creating a new document** (a “new document” elsewhere): everything goes through the **“New”** button
at the top right.
- “From template …”: choose a document type (letter, invoice, medical report …) and a name – TFTN
  creates the document folder and opens it. The usual way.
- “Blank document” (Cmd/Ctrl+N): just start writing; the first Cmd/Ctrl+S asks for a name and
  creates the folder in your Documents folder. An empty draft like this is also open at start-up.
- “Import …”: turns a Word, Markdown, LaTeX or other file into a new document (chapter 7).

Open it again via “Open”: the recently opened documents or “Open document …”.

**Embed fonts:** both when creating from a template and when saving for the first time with Cmd/Ctrl+S there is a checkbox “Embed fonts”: with
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
check. Under “Tax exemption” in the “Document” tab you can choose “medical treatment (§ 6 (1) 19
Austrian VAT Act)” for medical services or “small business (§ 6 (1) 27)”: then there is no VAT, and
the matching note appears below the total (your own tax note takes precedence). This is not tax
advice.

**Logo and signature:** letters, medical reports and invoices put a logo in the header (letters
and invoices at the top right above the sender, medical reports to the left of the practice name);
letters and medical reports can also show an image of your signature above the name. In the
“Document” tab (group “Sender” or “Practice”), “Choose image …” brings the file into the document – if
it is not there yet, a copy goes to `bilder/` – and fills in the path; “Logo height in cm” (default
1.5) sets the size. For the signature, a scan with a transparent background (PNG) works best.
Without an image there is space to sign by hand, as before. The document type reaches the images
through the line `pfad: p => path(p)` in `main.typ` (new documents have it). A document that still
uses an older version of the type keeps typesetting unchanged; after updating it (“Document” tab,
see chapter 5) it shows a red note instead of the image until the line is added, for example
`#show: brief.with(daten: daten, pfad: p => path(p))`.

**What is a document?** In TFTN every document is a folder of its own, the document folder (other
programs call this a project; on the command line and in technical texts it is called that too).
It holds `tftn.toml`, the main file (usually `main.typ`), and for templates also `dokument.toml`
(front matter), chapter files, images, the bibliography file, the fonts (`fonts/`) and the packages
in use (`packages/`). Everything the document needs lives in the folder – you can copy it,
sync it and keep working on another computer without a network.

## 2. The window

- **Header:** the document on the left (hover for the full path); on the right “New” (blank document,
  from template, import), “Open”, “Versions”, “Export”, the gear (settings) and “?” (About TFTN,
  F1). In a narrow window only the icons remain.
- **About TFTN** (“?” or F1) shows the version and the typst version, links to the project page, the
  manual and “Report a bug”, all shortcuts currently in effect for reference (“Change shortcuts …”
  leads to the settings) and the licenses including the bundled components; “Show license texts”
  opens their folder. A link opens the page in the default browser only when you click it
  (section 11).
- **Open** is a menu: “Open document …” chooses a folder with `tftn.toml` (Cmd/Ctrl+O goes straight
  there); below it are the recently opened documents, up to ten, newest first. Hover for the full
  path; if two documents share a name, the folder above follows quietly. A document that is out of
  reach for the moment (say, a vault that is not mounted) is left out of the menu but stays
  remembered. “Clear list” forgets the list; the documents themselves stay. The list applies per
  computer.
- **Messages** appear in a band below the header. Short notices disappear by themselves; errors and
  questions stay until you close them with × or answer them.
- **Panels such as Export, New or Versions** close with ×, Esc or a click outside. Once you have
  typed something into a dialog (a name, say), only × or Esc closes it.
- **Editor** (left): Typst source with highlighting. Above it: “Block”, “Citation”, “Outline”,
  “Page” and, as icons, “Search the whole document” (magnifier), sidebar and focus view.
- **Tabs** above the editor: the main file and further chapter files (`.typ`, included for example
  with `#include "chapters/one.typ"`). “+” opens a chapter file of the document; clicking text from a
  chapter in the preview opens its tab by itself and jumps to the spot. If you write
  `#include "chapters/new.typ"` for a file that does not exist yet, TFTN creates it shortly after
  the last keystroke, with a heading from its name (missing folders are created too); “Open” in the
  message or “+” brings it up as a tab. Intermediate names while typing a path (say `metho.typ` on
  the way to `methode.typ`) are removed again as long as nobody has written into them. A chapter does not
  see the imports of the main file; if it needs something from the template, it gets it with its
  own `#import` line. Each tab has its own undo; a
  dot shows that it is not saved yet, × closes it (the file stays in the document, the main file is
  always open). The preview always shows the whole document, including unsaved chapters. All tabs
  are saved together (Cmd/Ctrl+S, or by themselves after a second). Outline, the Here panel,
  spelling and counting apply to the active tab; page settings, versions, document type and AI
  assistants (MCP) to the main file – changing something there takes you to its tab.
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
the block. Afterwards Tab jumps from field to field, Shift+Tab back, Esc or a mouse click finishes. The fields
still to be filled are tinted light blue; once you type in a field, it is no longer tinted. Cmd/Ctrl+Z undoes
the whole block in one step.

**Your own text blocks:** standard sentences and paragraphs (follow-up, treatment advice, findings,
closing formula …) live in `textbausteine.toml` in the folder for your own customisations
(chapter 10). They appear in every document type at the top of the “/” menu, in the palette and in
the “Insert” panel, under “Custom text blocks”. To add one: select text, right-click → “Save as text
block …”, enter a title and (optionally) a group. The block is appended to the end of the file;
everything already in it stays as it is. To edit: Settings → “Edit text blocks …” opens the file in
the text editor (if it is missing, it is created with a commented example). One block per entry:

```toml
[[baustein]]
titel = "Procedere Kontrolle"
gruppe = "Procedere"
kuerzel = "Mod-Alt-1"          # optional
einfuegen = '''
Kontrolle in ${feld:Wochen:6} Wochen, bei Beschwerden früher.
${cursor}'''
```

`titel` is the title, `einfuegen` the text, `gruppe` the group and `kuerzel` the shortcut.
`${feld:Name}` or `${feld:Name:Default}` are gaps to fill in (Tab jumps on), `${cursor}` places the
cursor, `${auswahl}` takes the selected text. Changes take effect as soon as the file is saved. If
the file is broken, a notice and the settings name line and reason; a single broken entry is
skipped, the others stay. Shortcuts can also be set in the settings, as for other blocks. The blocks
apply in text, not in code or equations (for those add `modus = ["code"]` or `["mathe"]`).

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

**Find and replace:** Cmd/Ctrl+F opens the search bar below the editor; it finds and replaces in the
active tab (with “match case”, “by word” and “regexp”).

**Search the whole document** (magnifier above the editor or Cmd/Ctrl+Shift+H) searches all `.typ` files of
the document: the main file and every chapter, including those not open as a tab (not in hidden
folders, `packages/`, `export/`, `_lokal/` and `_renders/`). For open tabs, the text in the editor
counts, even if unsaved. A selection in the text is already in the search field when the window opens.
The same switches as in the search bar: “Match case”, “by word”, “regexp” (as in JavaScript; a broken
expression is explained below the field). Matches are grouped by file with line number and excerpt; a
click opens the file as a tab and selects the match. At most 1,000 matches are listed; counting goes
on, and the line above the list says so. “Replace” (shown when hovering a match) replaces that one;
“Replace all …” first states the number of matches and files, a second click replaces. In the
replacement, `\n` stands for a line break, and with regular expressions `$1`, `$2` … for the groups and
`$&` for the whole match. Replacing happens in the editor, not directly in the files: files that are
not open yet open as tabs; Cmd/Ctrl+Z undoes the replacement per tab, and saving works as usual. The
list is a snapshot: after further typing, press Enter in the search field to search again; a match that
has moved in the meantime is not replaced blindly.

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
the document.

**More fonts:** “More fonts …” below the font choice lists the fonts installed on the computer, each
with a sample line, the number of styles and the size. “Font file …” adds an
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
`ieee` or a `.csl` file in the document). Each value is checked before it is written to
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
the document, records their checksums in `tftn.toml` and inserts the `#import` if you like. From then
on the document builds offline, also on another computer. “Missing in the document” lists packages that a
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
Mac `brew install pandoc`, Windows `winget install --id JohnMacFarlane.Pandoc`, Linux through the
package manager (Ubuntu 24.04: `sudo apt install pandoc`; older systems often ship a version that is
too old, then use the package from pandoc.org).

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

**Print** (“Export” → “Print …” or Cmd/Ctrl+P): saves, typesets the regular PDF like “Test build”
into `_renders/` and opens it in the system's PDF viewer, where you print with Cmd/Ctrl+P.

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

**Web page (HTML)** (“Export” → “Web page (HTML)” → “Export”, no Pandoc needed): typst sets the
document as HTML; headings, lists, tables, figures, footnotes, equations (as MathML), citations and
bibliography come along, the page layout (margins, headers, page numbers) does not. TFTN adds a plain
stylesheet for reading. The result is a single file, `<document name>.html` in the document folder:
images are inside it, fonts come from the browser, and it loads nothing from the internet – ready to
pass on, upload or read on a phone. “Open in browser” shows it right away; whatever typst leaves out
of the HTML is listed in the dialog (“Missing from the web page: …”). typst's HTML export is still
new; templates with an elaborate layout look plainer on the web than on paper.

**E-book (EPUB)** (“Export” → “E-book (EPUB)” → “Export”, requires Pandoc): the same route as Word –
typst sets the document as HTML, Pandoc turns it into an EPUB 3 for e-book readers, Apple Books or
Calibre. It has a title page, a table of contents over two levels and a section of its own for each
chapter (first-level heading); footnotes, tables, images, equations (as MathML), citations and
bibliography come along, the page layout does not – the reader chooses font and size. Title, authors
and language come from the document, the date from the date field in the “Document” panel (otherwise
the document's timestamp). TFTN adds a plain stylesheet (figures at most as wide as the page). The file
is saved as `<document name>.epub` in the document folder and overwritten by the next export; whatever
typst leaves out of the HTML is listed in the dialog (“Missing from the e-book: …”). Pandoc runs
sandboxed and loads nothing from the internet. There is no cover image.

**Series** (“Export” → “Series …”): one PDF per record, e.g. for mail merge and invoices. The records
are either a folder of `.toml` files (one document per file, the PDF is named after the file) or one
`.toml` with `[[serie]]` sections. A record holds only what changes; everything else – sender, bank,
closing – comes from the Document pane. Groups such as `[absender]` are merged, all other
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

**From a CSV** (address lists from Excel or practice software): instead of a `.toml` you can choose a
`.csv`; every row becomes a document, the first row names the columns. TFTN detects the separator
(semicolon as in German Excel, comma, tab) and the encoding: UTF-8 (including Excel's “CSV UTF-8”),
otherwise Windows-1252 as written by older Excel's “CSV (delimited)” – so umlauts come out right. Cells
in quotes may contain semicolons or line breaks; empty rows at the end are fine.

```
Datei;Rechnungsnummer;Empfänger 1;Empfänger 2;Empfänger 3;Empfänger 4;Anrede;Rechnungsdatum
2026-014 Miller;2026-014;Mr;Dr Hans Miller;5 High Street;Graz 8010;Dear Dr Miller,;31.01.2026
```

- **Columns and fields:** a column belongs to a field if it is named like the field's title (the
  German one from the template, e.g. “Rechnungsnummer”, or, with TFTN in English, its translation
  “Invoice number”) or like its key in `dokument.toml` (`nummer`); case, spaces, hyphens and “ae” for
  “ä” do not matter. Fields in groups take a dot or the group title in front: `absender.telefon` or
  “Absender Telefon”. An address (a list of lines such as `empfaenger`) comes from several columns
  `Empfänger 1`, `Empfänger 2` … or from one cell with line breaks. A column “Datei” (or “Dateiname”)
  names the PDF. Invoice items do not come from the CSV but from the Document pane.
- **Values:** numbers in German or English style (`1.234,50`, `1234.50`; a single dot counts as the
  decimal point), dates as `31.01.2026` or `2026-01-31` (four-digit year), yes/no fields as ja/nein,
  yes/no, x or 1/0; for a choice the value or its label counts. An empty cell leaves the field out, and
  the value from the Document pane applies.
- **In the dialog** you see the number of records, the assignment of every column (unassigned ones
  are highlighted, with a warning above), the values of the first record as they go into the document,
  and the errors with row and column (row numbers as in Excel, the header row is row 1). A row with an
  error produces no PDF; the others are made, and it is reported at the end.
- **Your own assignment:** the drop-down next to each column picks another field or “do not use”.
  TFTN remembers this in the document, in `tftn.toml` under `[serie-csv."<file name>.csv"]` – only column
  names and field names, no values. A fresh export with the same file name finds the assignment again,
  even in another folder. In the document folder the assignment travels with the document type whose fields it
  names and is backed up with it; next to a freshly exported CSV (often in Downloads) it would easily
  get lost.

Internally every row becomes the same record as a `[[serie]]` section, so the combined PDF,
double-sided printing, the cross-check (`_lokal/serie/<name>.toml`) and incognito work the same way.
If the CSV is inside the document folder, incognito knows its values, even from columns with an
ambiguous name such as “Name”; a column that matches no field (e.g. “Telefon” for a letter) it does not
know. Address lists are therefore best kept outside the document, where an assistant cannot see them at
all.

On the command line: `tftn serie <folder> --daten <folder, .toml or .csv> [--ausgabe einzeln|gesamt|beides] [--beidseitig]`.

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
`trennungen.txt` in the document folder – journal article, book and report have this file. One word
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
| About TFTN and keyboard shortcuts | F1 |
| Find and replace in the active tab (editor) | Mod+F |
| Search and replace in the whole document | Mod+Shift+H |
| Editor font larger / smaller / as in the skin | Mod++ (also Mod+=) / Mod+− / Mod+0 |
| Values at this position (markers) | Mod+Shift+M or hold ⌥/Alt |
| Save selection as text block … | none (assignable); right-click into the selection |
| Sidebar | Mod+Shift+P |
| Sidebar: Document / Packages / Equation | Mod+Shift+O / +Y / +X |
| Distraction-free focus view | Mod+Shift+Enter |
| Save, New, Open | Mod+S, Mod+N, Mod+O |
| Print … | Mod+P |
| Bold, Italic, Link | Mod+B, Mod+I, Mod+K |
| Heading down / up | Mod+Shift+. / Mod+Shift+, |
| Footnote, Figure, Equation | Mod+Shift+F, +A, +E |

**Folder for your own customisations:** shortcuts (`tasten.toml`), your own text blocks
(`textbausteine.toml`, chapter 3), your dictionary, your own
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

**Editor font:** Cmd/Ctrl++ and Cmd/Ctrl+− make the font in the editor larger or smaller in steps
(60 to 250 %), Cmd/Ctrl+0 restores the size of the skin. This applies to the editor only and per
computer; zoom the preview with Cmd/Ctrl + mouse wheel.

## 11. Network, updates, privacy

TFTN only reads and writes your files. Only the bundled helper `tftn-holer` goes online, only when
you click, and only to fixed addresses:

| Purpose | Address |
|---|---|
| Packages | `packages.typst.org` |
| References from a DOI | `api.crossref.org`, `api.datacite.org` (only the DOI is sent) |
| Check for updates | `api.github.com` (latest version of TFTN and typst) |

While a request runs, the status bar shows it. **“Fully offline”** in the settings blocks all
network access, the links included.

**Links in “About TFTN”** (project page, manual, report a bug on `github.com`) open the page in the
default browser when clicked. It is the browser that goes online, not TFTN; TFTN only hands it the
fixed address and transmits nothing. Every click is recorded in the network log. If there is a page
for supporting TFTN (Ko-fi, `ko-fi.com`), it appears quietly as the last link; a switch in the
settings hides it. With “Fully offline” the links do not open, and the addresses are shown for
copying. TFTN never installs anything by itself: you download a new version from the release
page.

With **MCP** (section 13) an AI assistant reads the open document. TFTN itself does not go online for
this. Where the content goes depends on the assistant: a local model (in LM Studio, for example) keeps
everything on your computer, a cloud assistant sends it to its provider. MCP stays off until you turn
it on. With **incognito** (section 13), personal data from the form only leaves as stand-in text, for
example “[Female patient]” instead of the name.

**Network log – provably offline.** TFTN keeps track of every network access since it started.
Normally there is none. If TFTN did go online after a click, the status bar shows the count
(“1 network access”); clicking it, or **Settings → Network log …**, opens the window:

- at the top, one line saying whether and how often TFTN went online since the start;
- for each access the time, purpose (for example “Fetch package @preview/cetz:0.3.4 with its
  dependencies”), remote host, trigger (which click) and result, and below it every single connection
  with address, response and amount of data, as reported by the downloader itself;
- clicks on a link in “About TFTN”, with the address and the trigger “Click: link in ‘About TFTN’”;
- whether “Fully offline” is on, and how often AI programs accessed TFTN via MCP (counted per tool,
  plus rejected attempts). MCP is not a network access by TFTN – the connection stays on the computer
  (`127.0.0.1`) – but it passes text to another program, so it is listed as well. If incognito was on,
  it also says how many accesses were pseudonymised and which fields were replaced (only the field
  names, never the values); the privacy sheet says the same.

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
| `tftn neu --typ tftn-fachartikel <folder> [--sprache en]` | create a document from a document type (`--sprache en`: English template) |
| `tftn check <folder>` | check, report errors and missing packages |
| `tftn build <folder> [-o target]` | PDF via the typst CLI (into `_renders/`) |
| `tftn build <folder> --png [--ppi 144]` | page images instead of a PDF |
| `tftn export <folder> [--profil archiv] [--abnehmen]` | export profile of the document type |
| `tftn archiv <folder>` | archive package in `export/` (PDF/A, source, fonts, packages, instructions) |
| `tftn gegenprobe <folder>` | print the equivalent `typst compile` command |
| `tftn serie <folder> --daten <source> [--ausgabe einzeln\|gesamt\|beides] [--beidseitig] [--anpassen]` | one PDF per record or all in one (series) |
| `tftn katalog <folder>` | list the document’s blocks |
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
| `status` | document, revision, saved?, result of the last Typst run |
| `quelltext_lesen` | source of the main file as it is in the editor |
| `text_ersetzen` | replace one passage literally (or every occurrence) |
| `quelltext_setzen` | replace the whole source |
| `auswahl_lesen` | what you have selected and where the cursor is |
| `gliederung_lesen` | headings and comments, including included chapter files |
| `dateien_auflisten` | files in the document folder |
| `datei_lesen` | a text file of the document (chapter, `.bib`, `trennungen.txt` …) |
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
| `dokument_anlegen` | create a new document from a type and open it |
| `dokument_oeffnen` | open another document (the open one is saved first) |

**Limits:** the assistant only reaches files inside the document folder, not hidden folders, `_lokal/`
or `packages/`. It may only write text files; it changes the main file through the editor (with
undo) and the form data through the form. When it changes another file, such as a chapter, undo in the
editor does not apply – so TFTN first keeps the old content in `.fassungen/dateien/` (bibliography too). The assistant
cannot delete files or bibliography entries.

### Incognito: personal data only as stand-in text

When an assistant is to check a medical report, back it up with studies or look for mistakes, it does
not need the patient's name. With **incognito**, TFTN replaces personal data from the form with
stand-in texts before anything reaches the assistant, and puts the real values back when it writes.

- **Which fields:** fields that the document type marks as personal data are incognito by default –
  for the medical report the name, date of birth and social insurance number. In the “Document” pane
  each field has the checkbox **“Incognito for AI”** below it: ticking it adds a field (such as the
  recipient of a letter), unticking removes it. This applies to this document and is stored in
  `tftn.toml` – only the field name and, if any, your stand-in text, never a value.
- **Stand-in texts:** the template suggests them; for the medical report “[Female patient]”, “[Male
  patient]” or “[Patient]” depending on the field **Sex**, and “[born 1970]” for the date of birth:
  a medical check needs sex and year of birth, so both are passed on. Next to the checkbox you can
  enter your own stand-in text for each field (without brackets; empty: the template's suggestion).
  With several people (a series in the document) they become “[Female patient]”, “[Female patient 2]”
  and so on.
- **What is replaced:** the values from the data file (`dokument.toml`) and from series records inside
  the document folder (TOML or CSV) – wherever the assistant would see them: in the source, in chapters and other
  files, in the form, in file names and the document path, in messages. TFTN recognises the whole value,
  the parts of a name (the surname alone, also with a genitive s), dates in the usual spellings
  (1970-02-01, 01.02.1970, 1 February 1970 …) and numbers with or without spaces, regardless of
  upper or lower case. Each spelling gets its own stand-in text (such as “[Female patient, part 2]”),
  so that TFTN puts back exactly what was there.
- **Writing back:** the assistant leaves the stand-in texts as they are or uses them in new text; TFTN
  puts the real values in before anything is written. A stand-in text that TFTN did not hand out (such
  as “[Female patient 7]”) is rejected – it never ends up in the document.
- **Page image and messages:** for `seite_ansehen` TFTN typesets the page separately from the stand-in
  texts, so the assistant sees no name in the image either. If that fails, it gets no image. Images
  from anywhere else are not given out while incognito is on. The Typst messages the assistant gets
  also come from such a run, not from the real one.
- **Across documents:** values TFTN has replaced in any document since it started are replaced in
  every other document too until you quit (in a folder name, for example); a stand-in text keeps its
  meaning that long.
- **Visible:** the status bar then says “MCP on · incognito”, the tooltip names the fields. **“What
  the AI sees …”** in the “Document” pane shows the table stand-in text → real value; it stays on your
  computer.
- **Blocked rather than half:** if the data file or a series record cannot be read, TFTN does not know
  the values for sure and gives out nothing over MCP until the error is fixed (“MCP on · incognito –
  blocked”).

**Limits of incognito:** only what TFTN knows as the value of a field is replaced. A name that only
appears in the running text, an inflection other than the genitive s, a nickname, a typo, or a name
glued to another word without a space is not recognised. TFTN cannot check embedded images and PDFs
(scans, signature); the answer to the page image names them. For sensitive documents, check “What the
AI sees …” and the source before letting a cloud service work on them.

<!-- Ko-fi: switch on once the name exists.
## Supporting TFTN

TFTN costs nothing: no subscription, no ads, no data. If you use it regularly and want to give something back, you can do so on
[Ko-fi](https://ko-fi.com/<name>). Bug reports and wishes on the
[project page](https://github.com/DocAtPrompt/tftn/issues) are just as welcome.

The link is also in “About TFTN” (F1); it opens the page in the browser only when clicked, and the
settings can hide it.
-->
