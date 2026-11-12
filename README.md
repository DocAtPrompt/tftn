# TFTN – Typst for the Noobs

A native, offline writing app for [Typst](https://typst.app) on macOS and Windows: write on the
left, see the finished page on the right as you type. Built with Rust, Tauri 2 and CodeMirror 6.

- **Live preview with click-to-source** in both directions.
- **Blocks and completion** for figures, tables, equations, citations and more – type “/” on an empty
  line.
- **Change values without code:** markers, a context sidebar, a table grid and an equation palette.
- **Document types** as Typst packages (a journal article ships with TFTN), with a form for the front
  matter.
- **Citations** from your BibTeX/BibLaTeX file, or straight from a DOI (Crossref, DataCite).
- **Typst Universe packages** vendored into the project with checksums – documents build offline,
  on any machine.
- **Import** from Word, OpenDocument, Markdown, HTML, EPUB and LaTeX, **export** to PDF (incl.
  PDF/A) and to Word for proofreading – both via Pandoc.
- **Autosave with versions**, outline with drag-and-drop sections, spell checking, word count,
  distraction-free focus view.
- **Offline by design:** no network access except on explicit request, through one small helper
  with a fixed list of addresses. Your text stays plain Typst; `tftn gegenprobe` prints the
  `typst compile` command that produces the identical PDF without TFTN.

Interface and bundled document types in English and German; more interface languages can be added
as plain files. New documents follow the interface language.

**Status:** early but usable day to day on macOS (Apple silicon). Windows builds and passes the test
suite, but has not been tried by hand yet.

## Documentation

- [Manual (English)](docs/manual.md)
- [Handbuch (Deutsch)](docs/handbuch.md)
- [`demo/`](demo/) – a sample article to open in TFTN

## Install

Download `TFTN.app` from the [releases](https://github.com/DocAtPrompt/tftn/releases) and drag it to
`/Applications`. The app is not notarized: on first launch, right-click → “Open”. The matching typst
CLI is bundled. For Windows, build from source for now.

Optional: [Pandoc](https://pandoc.org) 3.1.2 or later for import and Word export
(`brew install pandoc`, `winget install --id JohnMacFarlane.Pandoc`).

## Build from source

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.92.0 | pinned in `rust-toolchain.toml`, rustup installs it |
| Node.js | 26.12 LTS | pinned in `.nvmrc` |
| typst CLI | exactly 0.15.1 | `cargo xtask typst-holen` downloads and verifies it into `_lokal/`; or put it on `PATH` / in `TFTN_TYPST` |
| Pandoc | ≥ 3.1.2 | optional: import, Word export |
| veraPDF + Java | ≥ 1.30 / ≥ 11 | optional: PDF/A checks in the tests |
| Windows | MSVC Build Tools, WebView2 | “Desktop development with C++” |

```bash
npm ci
```

```bash
cargo xtask typst-holen
```

```bash
npx tauri dev
```

```bash
npm run buendel
```

`npx tauri dev` starts the app in development mode (`TFTN_PROJEKT=<folder>` opens a specific
project). `npm run buendel` builds `target/release/bundle/macos/TFTN.app` (release, ad-hoc signed).

## Repository layout

| Path | Content |
|---|---|
| `crates/tftn-core` | preview world, incremental compile, click-to-source |
| `crates/tftn-edit` | value editing via the Typst syntax tree (patches) |
| `crates/tftn-katalog` | blocks, snippets, completion |
| `crates/tftn-literatur` | citations, DOI lookup |
| `crates/tftn-export`, `tftn-manifest`, `tftn-pakete` | export via the typst CLI, document types, package vendoring |
| `crates/tftn-holer` | the only program with network access (fixed addresses) |
| `crates/tftn-sprachen` | interface translations (`tr!`) |
| `crates/tftn-cli` | the `tftn` command line tool |
| `src-tauri/`, `ui/` | the app: Tauri shell and interface (TypeScript, CodeMirror 6) |
| `typen/` | bundled document types (Typst packages) |
| `pakete/` | descriptions of Universe packages with helpers (cetz, fletcher, cetz-plot) |
| `sprachen/` | interface language files (`en.toml`) |
| `woerterbuecher/` | Hunspell dictionaries, each with its license |
| `tests/korpus/` | test documents (synthetic), reference images, checksums |
| `demo/` | sample project |
| `xtask/` | build and check tasks |

Code, comments and identifiers are in German.

## Development

| Command | Purpose |
|---|---|
| `cargo xtask ci` | everything CI checks, locally and offline: UI build and tests, fmt, clippy, Rust tests, network isolation, checksums, visual regression (macOS) |
| `cargo test --workspace` | Rust tests (needs typst 0.15.1 on `PATH` or in `TFTN_TYPST`) |
| `npm test` | UI tests (vitest) |
| `cargo xtask referenzen` | visual regression against `tests/referenz/` |
| `cargo xtask sprachen` | sync `sprachen/en.toml` with the texts in the code |
| `cargo xtask typstyle [--check]` | format the bundled Typst sources |
| `cargo xtask messen` | measure keystroke-to-preview latency |

Every visible text is written in German in the code (`tr!("…")` in Rust, `t("…")` in TypeScript);
`cargo xtask sprachen` collects them and a test requires `sprachen/en.toml` to be complete.

## Command line

`tftn neu`, `check`, `build`, `export`, `gegenprobe`, `katalog`, `typ-pruefen` – see the
[manual](docs/manual.md#12-command-line).

## License

TFTN is licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Bundled third-party material keeps its own license, stored next to it: fonts (`LIZENZEN.txt` in the
`fonts`/`schriften` folders), dictionaries (`woerterbuecher/*/license`), the Typst packages in the
test corpus, and the typst CLI (Apache-2.0).
