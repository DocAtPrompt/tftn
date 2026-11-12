//! TFTN-Kern: Projekt, World der Vorschau, Kompilieren und Rastern. Kein UI, kein PDF (E12).

pub mod deutsch;
pub mod gegenprobe;
pub mod projekt;
pub mod schriften;
pub mod spiegel;
pub mod sprache;
pub mod vergleich;
pub mod vorschau;
pub mod world;

use tftn_sprachen::tr;
use typst::World;
use typst::WorldExt;
use typst::diag::{Severity, SourceDiagnostic};
use typst::utils::Scalar;
pub use typst_layout::PagedDocument;
use typst_render::RenderOptions;

pub use projekt::{Projekt, ProjektFehler, schriftvorrat_setzen};
pub use world::{ProjektWorld, WorldFehler};

/// Eine Meldung des Compilers, für Menschen aufbereitet.
#[derive(Debug, Clone)]
pub struct Meldung {
    pub fehler: bool,
    /// `datei:zeile:spalte` (1-basiert), falls bekannt.
    pub ort: Option<String>,
    pub text: String,
    /// Datei und Bytebereich der Meldung, falls bekannt (für Markierungen im Editor).
    pub stelle: Option<(typst::syntax::FileId, std::ops::Range<usize>)>,
}

impl std::fmt::Display for Meldung {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let art = if self.fehler {
            tr!("Fehler")
        } else {
            tr!("Warnung")
        };
        match &self.ort {
            Some(ort) => write!(f, "{art}: {ort}: {}", self.text),
            None => write!(f, "{art}: {}", self.text),
        }
    }
}

/// Ergebnis eines Laufs.
pub struct Ergebnis {
    pub dokument: Option<PagedDocument>,
    pub meldungen: Vec<Meldung>,
}

/// Kompiliert das Hauptdokument der World.
pub fn kompilieren(world: &ProjektWorld) -> Ergebnis {
    let warned = typst::compile::<PagedDocument>(world);
    let mut meldungen: Vec<Meldung> = warned.warnings.iter().map(|d| meldung(world, d)).collect();
    let dokument = match warned.output {
        Ok(dokument) => Some(dokument),
        Err(fehler) => {
            meldungen.extend(fehler.iter().map(|d| meldung(world, d)));
            None
        }
    };
    Ergebnis {
        dokument,
        meldungen,
    }
}

fn meldung(world: &ProjektWorld, d: &SourceDiagnostic) -> Meldung {
    let stelle = d.span.id().zip(world.range(d.span));
    let ort = d.span.id().and_then(|id| {
        let quelle = world.source(id).ok()?;
        let start = world.range(d.span)?.start;
        let zeile = quelle.lines().byte_to_line(start)? + 1;
        let spalte = quelle.lines().byte_to_column(start)? + 1;
        Some(format!(
            "{}:{zeile}:{spalte}",
            id.vpath().get_without_slash()
        ))
    });
    let hinweise: Vec<&str> = d.hints.iter().map(|h| h.v.as_str()).collect();
    let text = deutsch::meldungstext(&d.message, &hinweise);
    Meldung {
        fehler: d.severity == Severity::Error,
        ort,
        text,
        stelle,
    }
}

/// Rastert alle Seiten als PNG, genau wie `typst compile --format png --ppi <ppi>`.
pub fn seiten_png(dokument: &PagedDocument, ppi: f64) -> Vec<Vec<u8>> {
    let optionen = RenderOptions {
        pixel_per_pt: Scalar::new(ppi / 72.0),
        render_bleed: false,
    };
    dokument
        .pages()
        .iter()
        .map(|seite| {
            typst_render::render(seite, &optionen)
                .encode_png()
                .expect("PNG-Kodierung im Speicher")
        })
        .collect()
}

/// Text jeder Seite, nach Zeilen geordnet (für Tests und Textvergleiche, nicht für Barrierefreiheit).
/// Zeilen werden über die Grundlinie gebildet; Drehungen und Skalierungen bleiben unberücksichtigt.
pub fn seitenzeilen(dokument: &PagedDocument) -> Vec<Vec<String>> {
    dokument
        .pages()
        .iter()
        .map(|seite| {
            let mut stuecke = Vec::new();
            stuecke_sammeln(&seite.frame, 0.0, 0.0, &mut stuecke);
            // Grundlinien auf 1/10 pt gerundet zu Zeilen zusammenfassen.
            stuecke.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
            let mut zeilen: Vec<(i64, String)> = Vec::new();
            for (y, _, text) in stuecke {
                match zeilen.last_mut() {
                    Some((zy, zeile)) if *zy == y => zeile.push_str(&text),
                    _ => zeilen.push((y, text)),
                }
            }
            zeilen.into_iter().map(|(_, z)| z).collect()
        })
        .collect()
}

fn stuecke_sammeln(
    frame: &typst::layout::Frame,
    dx: f64,
    dy: f64,
    stuecke: &mut Vec<(i64, f64, String)>,
) {
    use typst::layout::FrameItem;
    for (pos, item) in frame.items() {
        let x = dx + pos.x.to_pt();
        let y = dy + pos.y.to_pt();
        match item {
            FrameItem::Group(g) => stuecke_sammeln(
                &g.frame,
                x + g.transform.tx.to_pt(),
                y + g.transform.ty.to_pt(),
                stuecke,
            ),
            FrameItem::Text(t) => stuecke.push(((y * 10.0).round() as i64, x, t.text.to_string())),
            _ => {}
        }
    }
}
