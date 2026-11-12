//! Kompilierlauf der Vorschau mit „letztem gutem Stand“ (KONZEPT 4.2, 4.3; PLAN-M1 AP4).
//!
//! Ein Lauf kompiliert die aktuelle Editor-Quelle. Gelingt er, wird das Dokument der neue gute
//! Stand; jede Seite bekommt einen Hash, damit nur geänderte Seiten neu gerastert werden. Schlägt
//! er fehl, bleibt der alte gute Stand stehen und die Meldungen gehen an den Editor.
//! Nach jedem Lauf werden alte Einträge aus dem Cache von comemo entfernt.
//!
//! Klicksprung (KONZEPT 4.4, PLAN-M1 AP5): `klick` bildet einen Punkt auf einer Seite auf eine
//! Stelle in der Quelle ab, `stellen` eine Quellstelle auf Punkte in der Vorschau. Text aus
//! `dokument.toml` oder aus dem Paket hat keine Stelle in `main.typ`; solche Klicks ergeben
//! `Sprung::Vorlage` statt eines falschen Sprungs.

use std::time::Instant;

use typst::introspection::PagedPosition;
use typst::layout::{Abs, Frame, FrameItem, Point};
use typst::syntax::{LinkedNode, Side, Source, Span, VirtualRoot};
use typst::utils::{Scalar, hash128};
use typst_ide::Jump;
use typst_layout::PagedDocument;
use typst_render::RenderOptions;

use crate::{Meldung, Projekt, ProjektWorld, WorldFehler, kompilieren};

/// Wie viele Läufe ein ungenutzter Cache-Eintrag überlebt (KONZEPT 4.2).
pub const CACHE_ALTER: usize = 10;

/// Der zuletzt fehlerfrei kompilierte Stand.
pub struct GuterStand {
    pub rev: u64,
    pub dokument: PagedDocument,
    /// Hash je Seite (Inhalt, Größe, Füllung).
    pub hashes: Vec<u128>,
}

/// Seitengröße in Punkt und Hash, für die Oberfläche.
#[derive(Debug, Clone, PartialEq)]
pub struct Seite {
    pub hash: u128,
    pub breite: f64,
    pub hoehe: f64,
}

/// Ergebnis eines Laufs.
pub struct Lauf {
    pub rev: u64,
    /// `true`: Dieser Lauf ist der neue gute Stand.
    pub gelungen: bool,
    pub meldungen: Vec<Meldung>,
    /// Revision des angezeigten (guten) Stands, falls es einen gibt.
    pub stand_rev: Option<u64>,
    /// Seiten des guten Stands.
    pub seiten: Vec<Seite>,
    /// Seiten (0-basiert), deren Hash sich gegenüber dem vorigen guten Stand geändert hat.
    pub geaendert: Vec<usize>,
    pub dauer_ms: f64,
}

/// Wohin ein Klick in der Vorschau führt.
#[derive(Debug, Clone, PartialEq)]
pub enum Sprung {
    /// Byte in der Hauptdatei, bezogen auf die Quelle des letzten Laufs (`Vorschau::quelle_rev`).
    Haupt(usize),
    /// Eine andere Datei des Projekts (Pfad relativ zum Projekt) mit Byte.
    Datei(String, usize),
    /// Text der Titelei in `tftn-feld("<name>", …)`: das Formularfeld `<name>` (PLAN-M2 AP7).
    Feld(String),
    /// Text aus dem Dokumenttyp-Paket oder aus Formulardaten (KONZEPT 4.4).
    Vorlage,
    Url(String),
    /// Ziel auf einer Seite (interner Verweis): Seite 0-basiert, Punkt in pt.
    Position(usize, f64, f64),
    Nichts,
}

pub struct Vorschau {
    world: ProjektWorld,
    gut: Option<GuterStand>,
    quelle_rev: u64,
}

impl Vorschau {
    pub fn neu(projekt: &Projekt) -> Result<Self, WorldFehler> {
        Ok(Self {
            world: ProjektWorld::neu(projekt)?,
            gut: None,
            quelle_rev: 0,
        })
    }

    pub fn world(&self) -> &ProjektWorld {
        &self.world
    }

    pub fn gut(&self) -> Option<&GuterStand> {
        self.gut.as_ref()
    }

    /// Kompiliert Revision `rev`; `haupt` ist die Quelle aus dem Editor (sonst die Platte).
    pub fn lauf(&mut self, rev: u64, haupt: Option<Source>) -> Lauf {
        let start = Instant::now();
        self.quelle_rev = rev;
        self.world.zuruecksetzen();
        if let Some(q) = haupt {
            self.world.quelle_setzen(q);
        }
        let ergebnis = kompilieren(&self.world);
        let gelungen = ergebnis.dokument.is_some();
        let mut geaendert = Vec::new();
        if let Some(dokument) = ergebnis.dokument {
            let hashes: Vec<u128> = dokument.pages().iter().map(hash128).collect();
            let alt = self
                .gut
                .as_ref()
                .map(|g| g.hashes.as_slice())
                .unwrap_or(&[]);
            geaendert = (0..hashes.len())
                .filter(|&i| alt.get(i) != Some(&hashes[i]))
                .collect();
            self.gut = Some(GuterStand {
                rev,
                dokument,
                hashes,
            });
        }
        comemo::evict(CACHE_ALTER);
        let seiten = self
            .gut
            .as_ref()
            .map(|g| {
                g.dokument
                    .pages()
                    .iter()
                    .zip(&g.hashes)
                    .map(|(p, &hash)| Seite {
                        hash,
                        breite: p.frame.width().to_pt(),
                        hoehe: p.frame.height().to_pt(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Lauf {
            rev,
            gelungen,
            meldungen: ergebnis.meldungen,
            stand_rev: self.gut.as_ref().map(|g| g.rev),
            seiten,
            geaendert,
            dauer_ms: start.elapsed().as_secs_f64() * 1000.0,
        }
    }

    /// Revision der Hauptquelle, die die World gerade kennt (Bezug für `Sprung::Haupt`).
    pub fn quelle_rev(&self) -> u64 {
        self.quelle_rev
    }

    /// Vorschau → Quelle: Klick auf Seite `seite` (0-basiert) am Punkt (`x`, `y`) in pt.
    pub fn klick(&self, seite: usize, x: f64, y: f64) -> Sprung {
        let Some(gut) = self.gut.as_ref() else {
            return Sprung::Nichts;
        };
        let Some(frame) = gut.dokument.pages().get(seite).map(|p| &p.frame) else {
            return Sprung::Nichts;
        };
        let punkt = Point::new(Abs::pt(x), Abs::pt(y));
        // Erst ohne Verweise: Zitate und Querverweise sind Links aufs Ziel; im Editor soll ein
        // Klick darauf aber zur Stelle im Text führen. Nur wenn dort nichts liegt, dem Link folgen.
        let ohne = typst_ide::jump_from_click_in_frame(
            &self.world,
            &gut.dokument,
            &ohne_verweise(frame),
            punkt,
        );
        let sprung = match ohne {
            Some(Jump::File(..)) => ohne,
            _ => {
                let seite = std::num::NonZeroUsize::new(seite + 1).expect("seite + 1 > 0");
                typst_ide::jump_from_click(
                    &self.world,
                    &gut.dokument,
                    &PagedPosition {
                        page: seite,
                        point: punkt,
                    },
                )
            }
        };
        match sprung {
            Some(Jump::File(id, byte)) if id == self.world.haupt() => Sprung::Haupt(byte),
            Some(Jump::File(id, byte)) => match id.root() {
                VirtualRoot::Project => {
                    Sprung::Datei(id.vpath().get_without_slash().to_string(), byte)
                }
                // Im Paket: steht die Stelle in einer Feldmarke, führt sie zum Formularfeld.
                _ => typst::World::source(&self.world, id)
                    .ok()
                    .and_then(|q| feld_an(&q, byte))
                    .map_or(Sprung::Vorlage, Sprung::Feld),
            },
            Some(Jump::Url(url)) => Sprung::Url(url.as_str().to_string()),
            Some(Jump::Position(p)) => {
                Sprung::Position(p.page.get() - 1, p.point.x.to_pt(), p.point.y.to_pt())
            }
            // Kein Span: typischerweise Text aus Daten (toml) oder vom Paket erzeugt.
            None => Sprung::Vorlage,
        }
    }

    /// Quelle → Vorschau: Punkte (Seite 0-basiert, x, y in pt) für das Byte `byte` in `quelle`.
    /// `quelle` muss die Spans der kompilierten Fassung tragen (Spiegel-Quelle); bei veraltetem
    /// gutem Stand werden nur unveränderte Bereiche gefunden.
    pub fn stellen(&self, quelle: &Source, byte: usize) -> Vec<(usize, f64, f64)> {
        let Some(gut) = self.gut.as_ref() else {
            return Vec::new();
        };
        let gefunden: Vec<_> = typst_ide::jump_from_cursor(&gut.dokument, quelle, byte)
            .into_iter()
            .map(|p| (p.page.get() - 1, p.point.x.to_pt(), p.point.y.to_pt()))
            .collect();
        if !gefunden.is_empty() {
            return gefunden;
        }
        // jump_from_cursor kennt nur Textknoten; für Formelzeichen, Code usw. die Spans des Knotens
        // und seiner Eltern direkt in den Glyphen suchen.
        let wurzel = LinkedNode::new(quelle.root());
        let mut knoten = wurzel
            .leaf_at(byte, Side::After)
            .or_else(|| wurzel.leaf_at(byte, Side::Before));
        for _ in 0..3 {
            let Some(k) = knoten else { break };
            let mut aus = Vec::new();
            for (nr, seite) in gut.dokument.pages().iter().enumerate() {
                span_suchen(&seite.frame, k.span(), nr, 0.0, 0.0, &mut aus);
            }
            if !aus.is_empty() {
                return aus;
            }
            knoten = k.parent().cloned();
        }
        Vec::new()
    }

    /// Rastert Seite `nr` (0-basiert) des guten Stands als PNG, falls ihr Hash noch `hash` ist.
    pub fn seite_png(&self, nr: usize, hash: u128, ppi: f64) -> Option<Vec<u8>> {
        let gut = self.gut.as_ref()?;
        if gut.hashes.get(nr) != Some(&hash) {
            return None;
        }
        Some(rastern(gut.dokument.pages().get(nr)?, ppi))
    }
}

/// Kopie eines Frames ohne Link-Flächen (rekursiv).
fn ohne_verweise(frame: &Frame) -> Frame {
    let mut neu = Frame::new(frame.size(), frame.kind());
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Link(..) => {}
            FrameItem::Group(g) => {
                let mut g = g.clone();
                g.frame = ohne_verweise(&g.frame);
                neu.push(*pos, FrameItem::Group(g));
            }
            anders => neu.push(*pos, anders.clone()),
        }
    }
    neu
}

/// Sammelt die Positionen aller Glyphen mit `span` (Grundlinie, Glyph-Anfang).
fn span_suchen(
    frame: &Frame,
    span: Span,
    seite: usize,
    dx: f64,
    dy: f64,
    aus: &mut Vec<(usize, f64, f64)>,
) {
    for (pos, item) in frame.items() {
        let (x, y) = (dx + pos.x.to_pt(), dy + pos.y.to_pt());
        match item {
            FrameItem::Group(g) => span_suchen(
                &g.frame,
                span,
                seite,
                x + g.transform.tx.to_pt(),
                y + g.transform.ty.to_pt(),
                aus,
            ),
            FrameItem::Text(t) => {
                let mut gx = x;
                for g in &t.glyphs {
                    if g.span.0 == span {
                        aus.push((seite, gx, y));
                        break;
                    }
                    gx += g.x_advance.at(t.size).to_pt();
                }
            }
            _ => {}
        }
    }
}

/// Der innerste Aufruf `tftn-feld("<name>", …)` um `byte`; `<name>` ist das Formularfeld der Titelei
/// (PLAN-M2 AP7). Rein syntaktisch: Das erste Argument muss ein String-Literal sein.
pub fn feld_an(quelle: &Source, byte: usize) -> Option<String> {
    use typst::syntax::ast;
    let wurzel = LinkedNode::new(quelle.root());
    let mut knoten = wurzel
        .leaf_at(byte, Side::After)
        .or_else(|| wurzel.leaf_at(byte, Side::Before))?;
    loop {
        if let Some(aufruf) = knoten.cast::<ast::FuncCall>()
            && let ast::Expr::Ident(name) = aufruf.callee()
            && name.as_str() == "tftn-feld"
            && let Some(ast::Arg::Pos(ast::Expr::Str(s))) = aufruf.args().items().next()
        {
            return Some(s.get().to_string());
        }
        knoten = knoten.parent()?.clone();
    }
}

/// Rastert eine Seite als PNG, wie `typst compile --format png --ppi <ppi>`.
pub fn rastern(seite: &typst_layout::Page, ppi: f64) -> Vec<u8> {
    let optionen = RenderOptions {
        pixel_per_pt: Scalar::new(ppi / 72.0),
        render_bleed: false,
    };
    typst_render::render(seite, &optionen)
        .encode_png()
        .expect("PNG-Kodierung im Speicher")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn feld_an_syntaktisch() {
        let text = r#"#tftn-feld("titel", text(daten.titel)) #tftn-feld("aussen", [a #tftn-feld("innen", b)]) x"#;
        let quelle = Source::detached(text);
        let in_titel = text.find("daten.titel").unwrap() + 2;
        assert_eq!(feld_an(&quelle, in_titel).as_deref(), Some("titel"));
        let innen = text.find(" b)").unwrap() + 1;
        assert_eq!(feld_an(&quelle, innen).as_deref(), Some("innen"));
        let aussen = text.find("[a").unwrap() + 1;
        assert_eq!(feld_an(&quelle, aussen).as_deref(), Some("aussen"));
        assert_eq!(feld_an(&quelle, text.len() - 1), None);
        // Erstes Argument muss ein String-Literal sein.
        let q = Source::detached(r#"#tftn-feld(name, x)"#);
        assert_eq!(feld_an(&q, 17), None);
    }

    fn testartikel() -> Projekt {
        Projekt::oeffnen(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel"),
        )
        .unwrap()
    }

    #[test]
    fn letzter_guter_stand_bleibt_bei_fehler() {
        let projekt = testartikel();
        let mut v = Vorschau::neu(&projekt).unwrap();
        let text = std::fs::read_to_string(projekt.haupt()).unwrap();
        let id = v.world().haupt();

        let eins = v.lauf(1, Some(Source::new(id, text.clone())));
        assert!(eins.gelungen);
        assert_eq!(eins.geaendert, (0..eins.seiten.len()).collect::<Vec<_>>());

        // Absichtlicher Syntaxfehler: Vorschau bleibt auf Rev 1, Meldung mit Stelle.
        let kaputt = text.replace("= Einleitung", "#let x = (");
        let zwei = v.lauf(2, Some(Source::new(id, kaputt)));
        assert!(!zwei.gelungen);
        assert_eq!(zwei.stand_rev, Some(1));
        assert_eq!(zwei.seiten, eins.seiten);
        assert!(
            zwei.meldungen
                .iter()
                .any(|m| m.fehler && m.stelle.is_some())
        );

        // Korrektur: wieder aktuell, keine Seite geändert gegenüber Rev 1.
        let drei = v.lauf(3, Some(Source::new(id, text)));
        assert!(drei.gelungen);
        assert_eq!(drei.stand_rev, Some(3));
        assert!(drei.geaendert.is_empty());
    }

    #[test]
    fn nur_betroffene_seiten_aendern_sich_und_png_nur_fuer_aktuellen_hash() {
        let projekt = testartikel();
        let mut v = Vorschau::neu(&projekt).unwrap();
        let text = std::fs::read_to_string(projekt.haupt()).unwrap();
        let id = v.world().haupt();
        let eins = v.lauf(1, Some(Source::new(id, text.clone())));
        // Ein Wort im letzten Absatz ändern: die erste Seite bleibt gleich.
        let neu = text.replace("Offen bleibt", "Unklar bleibt");
        let zwei = v.lauf(2, Some(Source::new(id, neu)));
        assert!(zwei.gelungen);
        assert!(!zwei.geaendert.is_empty());
        assert!(!zwei.geaendert.contains(&0), "{:?}", zwei.geaendert);
        let alt = eins.seiten[zwei.geaendert[0]].hash;
        assert!(v.seite_png(zwei.geaendert[0], alt, 72.0).is_none());
        let png = v.seite_png(0, zwei.seiten[0].hash, 72.0).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }
}
