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
use typst::syntax::{FileId, LinkedNode, Side, Source, Span, VirtualRoot};
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
    /// Datei, zu der `quelle_rev` gehört (die aktive des letzten Laufs).
    quelle_datei: Option<FileId>,
}

impl Vorschau {
    pub fn neu(projekt: &Projekt) -> Result<Self, WorldFehler> {
        Ok(Self {
            world: ProjektWorld::neu(projekt)?,
            gut: None,
            quelle_rev: 0,
            quelle_datei: None,
        })
    }

    pub fn world(&self) -> &ProjektWorld {
        &self.world
    }

    /// Liest die Schriften neu (nach `schriften::familie_einbinden`); der nächste Lauf setzt mit
    /// ihnen.
    pub fn schriften_neu_lesen(&mut self, projekt: &Projekt) {
        self.world.schriften_neu_lesen(projekt);
    }

    pub fn gut(&self) -> Option<&GuterStand> {
        self.gut.as_ref()
    }

    /// Kompiliert Revision `rev`; `haupt` ist die Quelle aus dem Editor (sonst die Platte).
    pub fn lauf(&mut self, rev: u64, haupt: Option<Source>) -> Lauf {
        self.world.zuruecksetzen();
        if let Some(q) = haupt {
            self.quelle_datei = Some(q.id());
            self.world.quelle_setzen(q);
        }
        self.setzen(rev)
    }

    /// Wie `lauf`, mit mehreren offenen Dateien: `aktiv` (Revision `rev`) und die übrigen offenen,
    /// alle ungespeichert aus dem Editor. Was nicht darunter ist, kommt von der Platte.
    pub fn lauf_offen(&mut self, rev: u64, aktiv: Source, weitere: Vec<Source>) -> Lauf {
        self.world.zuruecksetzen();
        self.quelle_datei = Some(aktiv.id());
        self.world
            .quellen_setzen(std::iter::once(aktiv).chain(weitere));
        self.setzen(rev)
    }

    fn setzen(&mut self, rev: u64) -> Lauf {
        let start = Instant::now();
        self.quelle_rev = rev;
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

    /// Die Datei, zu der `quelle_rev` gehört (die aktive des letzten Laufs).
    pub fn quelle_datei(&self) -> Option<FileId> {
        self.quelle_datei
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
                // Im Paket: steht die Stelle in einer Feldmarke, führt sie zum Formularfeld. Das
                // Literaturverzeichnis setzt zwar die Vorlage, ist aber kein Formularinhalt: Ein
                // Klick darauf tut nichts, statt das offene Fach zu wechseln.
                _ => match typst::World::source(&self.world, id) {
                    Ok(q) => match feld_an(&q, byte) {
                        Some(feld) => Sprung::Feld(feld),
                        None if aufruf_an(&q, byte, "bibliography") => Sprung::Nichts,
                        None => Sprung::Vorlage,
                    },
                    Err(_) => Sprung::Vorlage,
                },
            },
            Some(Jump::Url(url)) => Sprung::Url(url.as_str().to_string()),
            Some(Jump::Position(p)) => {
                Sprung::Position(p.page.get() - 1, p.point.x.to_pt(), p.point.y.to_pt())
            }
            // Kein Span: Liegt dort Text oder ein Bild, kommt es typischerweise aus Daten (toml)
            // oder vom Paket. Sonst war es eine leere Stelle (Rand, Zwischenraum in einer Tabelle):
            // dann nichts tun, statt das Formular zu öffnen.
            None if inhalt_an(frame, punkt) => Sprung::Vorlage,
            None => Sprung::Nichts,
        }
    }

    /// Quelle → Vorschau: Punkte (Seite 0-basiert, x, y in pt) für das Byte `byte` in `quelle`.
    /// `quelle` muss die Spans der kompilierten Fassung tragen (Spiegel-Quelle); bei veraltetem
    /// gutem Stand werden nur unveränderte Bereiche gefunden.
    ///
    /// Ist an `byte` nichts gesetzt (Kommentar, Leerzeile, Code ohne Ausgabe), gilt der nächste
    /// gesetzte Text danach, am Ende der Datei der davor.
    pub fn stellen(&self, quelle: &Source, byte: usize) -> Vec<(usize, f64, f64)> {
        let genau = self.stellen_genau(quelle, byte);
        if !genau.is_empty() || self.gut.is_none() {
            return genau;
        }
        let wurzel = LinkedNode::new(quelle.root());
        let Some(start) = wurzel
            .leaf_at(byte, Side::After)
            .or_else(|| wurzel.leaf_at(byte, Side::Before))
        else {
            return genau;
        };
        // Höchstens so viele Blätter in jede Richtung (jede Probe durchsucht das Dokument).
        const NACHBARN: usize = 40;
        for vorwaerts in [true, false] {
            let mut k = start.clone();
            for _ in 0..NACHBARN {
                let Some(n) = (if vorwaerts {
                    k.next_leaf()
                } else {
                    k.prev_leaf()
                }) else {
                    break;
                };
                if !n.kind().is_trivia() {
                    let s = self.stellen_genau(quelle, n.offset());
                    if !s.is_empty() {
                        return s;
                    }
                }
                k = n;
            }
        }
        genau
    }

    fn stellen_genau(&self, quelle: &Source, byte: usize) -> Vec<(usize, f64, f64)> {
        let Some(gut) = self.gut.as_ref() else {
            return Vec::new();
        };
        let mut gefunden: Vec<_> = typst_ide::jump_from_cursor(&gut.dokument, quelle, byte)
            .into_iter()
            .map(|p| (p.page.get() - 1, p.point.x.to_pt(), p.point.y.to_pt()))
            .collect();
        // Der Text einer Überschrift steht auch im Inhaltsverzeichnis und in Kolumnentiteln, die
        // davor liegen können: die Überschrift selbst zuerst.
        if let Some(u) = ueberschrift_stelle(&gut.dokument, quelle, byte) {
            gefunden.insert(0, u);
        }
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

/// Wo die Überschrift gesetzt ist, in der `byte` steht (Seite 0-basiert, x, y in pt); `None`
/// außerhalb von Überschriften.
fn ueberschrift_stelle(
    dokument: &PagedDocument,
    quelle: &Source,
    byte: usize,
) -> Option<(usize, f64, f64)> {
    use typst::foundations::NativeElement;
    use typst::introspection::Introspector;
    use typst::model::HeadingElem;
    use typst::syntax::SyntaxKind;
    let wurzel = LinkedNode::new(quelle.root());
    let mut knoten = wurzel
        .leaf_at(byte, Side::After)
        .or_else(|| wurzel.leaf_at(byte, Side::Before))?;
    let span = loop {
        if knoten.kind() == SyntaxKind::Heading {
            break knoten.span();
        }
        knoten = knoten.parent()?.clone();
    };
    let intro = dokument.introspector();
    intro
        .query(&HeadingElem::ELEM.select())
        .iter()
        .filter(|u| u.span() == span)
        .find_map(|u| {
            let p = intro.position(u.location()?)?;
            Some((p.page.get() - 1, p.point.x.to_pt(), p.point.y.to_pt()))
        })
}

/// Liegt am Punkt (relativ zu `frame`) ein Text oder ein Bild? Text zählt in seiner Breite und von
/// einer Schriftgröße über bis 0,3 Schriftgrößen unter der Grundlinie.
fn inhalt_an(frame: &Frame, punkt: Point) -> bool {
    frame.items().any(|(pos, item)| {
        let p = punkt - *pos;
        match item {
            FrameItem::Group(g) => g
                .transform
                .invert()
                .is_some_and(|inv| inhalt_an(&g.frame, p.transform(inv))),
            FrameItem::Text(t) => {
                p.x >= Abs::zero() && p.x <= t.width() && p.y >= -t.size && p.y <= t.size * 0.3
            }
            FrameItem::Image(_, groesse, _) => {
                p.x >= Abs::zero() && p.y >= Abs::zero() && p.x <= groesse.x && p.y <= groesse.y
            }
            _ => false,
        }
    })
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

/// Liegt `byte` in einem Aufruf der Funktion `name` (etwa `bibliography(…)`)?
fn aufruf_an(quelle: &Source, byte: usize, name: &str) -> bool {
    use typst::syntax::ast;
    let wurzel = LinkedNode::new(quelle.root());
    let mut knoten = wurzel
        .leaf_at(byte, Side::After)
        .or_else(|| wurzel.leaf_at(byte, Side::Before));
    while let Some(k) = knoten {
        if let Some(aufruf) = k.cast::<ast::FuncCall>()
            && let ast::Expr::Ident(n) = aufruf.callee()
            && n.as_str() == name
        {
            return true;
        }
        knoten = k.parent().cloned();
    }
    false
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
