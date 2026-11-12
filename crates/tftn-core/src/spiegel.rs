//! Der Rust-Spiegel einer im Editor geöffneten Datei (KONZEPT 4.1, PLAN-M1 AP3).
//!
//! Der Editor (CodeMirror) ist der einzige Schreiber. Er schickt je Transaktion die Änderungen in
//! UTF-16-Positionen des alten Texts und die Revision, auf der sie beruhen. Der Spiegel rechnet in
//! UTF-8 um und wendet sie mit `Source::edit` an, das nur den betroffenen Bereich neu parst.
//! Passt die Revision nicht, wird nichts angewendet; der Editor schickt dann den ganzen Text.
//!
//! Zeilenenden sind immer `\n`: CodeMirror führt den Text intern so, und `typst-syntax` 0.15.1
//! verliert seine Zeilentabelle, wenn zwischen `\r` und `\n` eingefügt wird (danach Absturz in
//! `Lines::utf16_to_byte`; gefunden mit P7, KONZEPT Anhang B). Beim Laden wird daher vereinheitlicht,
//! und Änderungen mit `\r` werden abgelehnt.

use std::ops::Range;
use tftn_sprachen::tr;

use serde::{Deserialize, Serialize};
use typst::syntax::{FileId, LinkedNode, Source, Tag};

/// Eine Änderung in UTF-16-Positionen des Texts vor der Transaktion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aenderung {
    pub von: usize,
    pub bis: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncFehler {
    /// Der Editor rechnet mit einer anderen Revision als der Spiegel.
    Revision { erwartet: u64, bekommen: u64 },
    /// Eine Position liegt außerhalb des Texts oder Änderungen überlappen.
    Position(usize),
    /// Eine Änderung enthält `\r`; der Editor liefert nur `\n`.
    Wagenruecklauf,
}

impl std::fmt::Display for SyncFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Revision { erwartet, bekommen } => f.write_str(&tr!(
                "Revision {bekommen} statt {erwartet}, Neusynchronisation nötig",
                bekommen,
                erwartet
            )),
            Self::Position(p) => {
                f.write_str(&tr!("ungültige Position {p}, Neusynchronisation nötig", p))
            }
            Self::Wagenruecklauf => f.write_str(&tr!(
                "Zeilenende \\r in einer Änderung, Neusynchronisation nötig"
            )),
        }
    }
}

impl std::error::Error for SyncFehler {}

pub struct Spiegel {
    quelle: Source,
    rev: u64,
}

impl Spiegel {
    /// Neuer Spiegel; Zeilenenden werden auf `\n` vereinheitlicht, wie im Editor.
    pub fn neu(id: FileId, text: &str) -> Self {
        Self {
            quelle: Source::new(id, zeilenenden_lf(text)),
            rev: 0,
        }
    }

    pub fn quelle(&self) -> &Source {
        &self.quelle
    }

    pub fn text(&self) -> &str {
        self.quelle.text()
    }

    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// Wendet eine Transaktion an, die auf `basis` beruht. Ergibt die neue Revision.
    /// Die Änderungen müssen aufsteigend sortiert sein und dürfen sich nicht überlappen
    /// (so liefert sie CodeMirrors `ChangeSet.iterChanges`).
    pub fn anwenden(&mut self, basis: u64, aenderungen: &[Aenderung]) -> Result<u64, SyncFehler> {
        if basis != self.rev {
            return Err(SyncFehler::Revision {
                erwartet: self.rev,
                bekommen: basis,
            });
        }
        if aenderungen.iter().any(|a| a.text.contains('\r')) {
            return Err(SyncFehler::Wagenruecklauf);
        }
        // Erst alle Positionen im alten Text umrechnen und prüfen, dann von hinten anwenden,
        // damit frühere Positionen gültig bleiben.
        let zeilen = self.quelle.lines();
        let mut bytes: Vec<(Range<usize>, &str)> = Vec::with_capacity(aenderungen.len());
        let mut ende = 0;
        for a in aenderungen {
            if a.von > a.bis || a.von < ende {
                return Err(SyncFehler::Position(a.von));
            }
            let von = zeilen
                .utf16_to_byte(a.von)
                .ok_or(SyncFehler::Position(a.von))?;
            let bis = zeilen
                .utf16_to_byte(a.bis)
                .ok_or(SyncFehler::Position(a.bis))?;
            if zeilen.byte_to_utf16(von) != Some(a.von) || zeilen.byte_to_utf16(bis) != Some(a.bis)
            {
                // mitten in einem Ersatzzeichenpaar
                return Err(SyncFehler::Position(a.von));
            }
            bytes.push((von..bis, &a.text));
            ende = a.bis;
        }
        let fehlerhaft = self.quelle.root().diagnosis().either();
        for (bereich, text) in bytes.into_iter().rev() {
            self.quelle.edit(bereich, text);
        }
        self.nachpruefen(fehlerhaft);
        self.rev += 1;
        Ok(self.rev)
    }

    /// Ersetzt den ganzen Text (Neusynchronisation). Ergibt die neue Revision.
    pub fn ersetzen(&mut self, text: &str) -> u64 {
        let rev = self.rev + 1;
        self.neu_aufsetzen(text, rev);
        rev
    }

    /// Neusynchronisation mit einer vom Editor vorgegebenen Revision: Der Editor kann so weiter
    /// Änderungen auf `rev` aufbauen, bevor die Antwort da ist.
    pub fn neu_aufsetzen(&mut self, text: &str, rev: u64) {
        // `replace` parst ebenfalls inkrementell.
        let fehlerhaft = self.quelle.root().diagnosis().either();
        self.quelle.replace(&zeilenenden_lf(text));
        self.nachpruefen(fehlerhaft);
        self.rev = rev;
    }

    /// Umgehung für `typst-syntax` 0.15.1 (gefunden mit P2): Der inkrementelle Parser liefert
    /// manchmal einen anderen Baum als ein vollständiger Lauf, bisher nur bei Syntaxfehlern im alten
    /// oder neuen Baum (geprüft samt Warnungen). Beispiele: Ein offenes `*` schluckt die Leerzeile,
    /// ein `=` danach wird Text statt Überschrift; nach `# #` wird `a` ein Bezeichner statt Text.
    /// Dann zur Kontrolle vollständig parsen und nur bei Abweichung übernehmen, sonst bleiben die
    /// Spans stabil. Kosten nur bei Fehlern, 182 KB Text rund 6 ms (KONZEPT Anhang B).
    fn nachpruefen(&mut self, alt_fehlerhaft: bool) {
        if alt_fehlerhaft || self.quelle.root().diagnosis().either() {
            let frisch = Source::new(self.quelle.id(), self.quelle.text().to_string());
            if !self.quelle.root().spanless_eq(frisch.root()) {
                self.quelle = frisch;
            }
        }
    }

    /// Prüfsumme über den UTF-8-Text (FNV-1a, 64 bit), gleich gerechnet wie im Editor.
    pub fn pruefsumme(&self) -> u64 {
        pruefsumme(self.text())
    }

    /// Syntaxhervorhebung für den UTF-16-Bereich `sichtbar`: Bereiche in UTF-16 mit CSS-Klasse.
    /// Verschachtelte Bereiche kommen nach Anfang sortiert, äußere zuerst.
    pub fn hervorhebung(&self, sichtbar: Range<usize>) -> Vec<(usize, usize, &'static str)> {
        let zeilen = self.quelle.lines();
        let von = zeilen.utf16_to_byte(sichtbar.start).unwrap_or(0);
        let bis = zeilen
            .utf16_to_byte(sichtbar.end)
            .unwrap_or(zeilen.len_bytes());
        let mut aus = Vec::new();
        sammeln(
            &LinkedNode::new(self.quelle.root()),
            von..bis,
            &mut |b, tag| {
                if let (Some(a), Some(e)) =
                    (zeilen.byte_to_utf16(b.start), zeilen.byte_to_utf16(b.end))
                {
                    aus.push((a, e, tag.css_class()));
                }
            },
        );
        aus
    }
}

fn sammeln(knoten: &LinkedNode, bereich: Range<usize>, f: &mut impl FnMut(Range<usize>, Tag)) {
    let r = knoten.range();
    if r.end < bereich.start || r.start > bereich.end {
        return;
    }
    if let Some(tag) = typst::syntax::highlight(knoten)
        && tag != Tag::Error
        && !r.is_empty()
    {
        f(r, tag);
    }
    for kind in knoten.children() {
        sammeln(&kind, bereich.clone(), f);
    }
}

/// `\r\n` und einzelnes `\r` → `\n`.
pub fn zeilenenden_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// FNV-1a 64 über die UTF-8-Bytes.
pub fn pruefsumme(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};

    fn id() -> FileId {
        RootedPath::new(VirtualRoot::Project, VirtualPath::new("main.typ").unwrap()).intern()
    }

    fn ae(von: usize, bis: usize, text: &str) -> Aenderung {
        Aenderung {
            von,
            bis,
            text: text.into(),
        }
    }

    #[test]
    fn umlaute_und_emoji_in_utf16() {
        // „ä“ ist 1 UTF-16-Einheit, 2 Bytes; „😀“ 2 Einheiten, 4 Bytes.
        let mut s = Spiegel::neu(id(), "ä😀b");
        assert_eq!(s.anwenden(0, &[ae(3, 4, "c")]).unwrap(), 1);
        assert_eq!(s.text(), "ä😀c");
        assert_eq!(s.anwenden(1, &[ae(0, 1, "Ö"), ae(1, 3, "")]).unwrap(), 2);
        assert_eq!(s.text(), "Öc");
    }

    #[test]
    fn mitten_im_ersatzzeichenpaar_ist_fehler() {
        let mut s = Spiegel::neu(id(), "😀");
        assert_eq!(
            s.anwenden(0, &[ae(1, 1, "x")]),
            Err(SyncFehler::Position(1))
        );
        assert_eq!(s.text(), "😀");
    }

    #[test]
    fn falsche_revision_aendert_nichts() {
        let mut s = Spiegel::neu(id(), "abc");
        let f = s.anwenden(5, &[ae(0, 0, "x")]);
        assert_eq!(
            f,
            Err(SyncFehler::Revision {
                erwartet: 0,
                bekommen: 5
            })
        );
        assert_eq!(s.text(), "abc");
    }

    #[test]
    fn zeilenenden_werden_vereinheitlicht_und_cr_abgelehnt() {
        let mut s = Spiegel::neu(id(), "a\r\nb\rc");
        assert_eq!(s.text(), "a\nb\nc");
        assert_eq!(
            s.anwenden(0, &[ae(1, 1, "\r\n")]),
            Err(SyncFehler::Wagenruecklauf)
        );
        assert_eq!(s.text(), "a\nb\nc");
        s.ersetzen("x\r\ny");
        assert_eq!(s.text(), "x\ny");
    }

    #[test]
    fn pruefsumme_bekannter_wert() {
        // FNV-1a 64 von "a" (Referenzwert der Spezifikation)
        assert_eq!(pruefsumme("a"), 0xaf63_dc4c_8601_ec8c);
    }

    /// typst-syntax 0.15.1 allein parst hier inkrementell falsch (siehe `nachpruefen`).
    #[test]
    fn inkrementell_falsch_geparstes_wird_korrigiert() {
        let gleich = |s: &Spiegel| {
            s.quelle()
                .root()
                .spanless_eq(Source::detached(s.text()).root())
        };
        let mut s = Spiegel::neu(id(), "*\n\n=\n\nbx");
        s.anwenden(0, &[ae(7, 8, "y")]).unwrap();
        assert!(gleich(&s));
        s.ersetzen("*\n\n=\n\nbz");
        assert!(gleich(&s));
        let mut s = Spiegel::neu(id(), " #$)u");
        s.anwenden(0, &[ae(0, 0, "#"), ae(2, 2, "a")]).unwrap();
        assert!(gleich(&s));
    }

    #[test]
    fn hervorhebung_ueberschrift() {
        let s = Spiegel::neu(id(), "= Jänner\n*fett*");
        let h = s.hervorhebung(0..100);
        assert!(h.contains(&(0, 8, "typ-heading")), "{h:?}");
        assert!(h.contains(&(9, 15, "typ-strong")), "{h:?}");
    }
}
