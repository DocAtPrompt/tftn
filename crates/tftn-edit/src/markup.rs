//! Eingebaute Markup-Helfer (Entwurf 4 „Was danach ändern umfasst“ 3, PLAN-M2 AP4): Syntax, die kein
//! Aufruf ist, bekommt Helfer statt Manifest-Einträgen. Überschriftenebene ±, Liste ↔ Aufzählung,
//! Formel Block ↔ inline. Jeder Patch wird nach (a)–(c) geprüft wie die Widgets.

use std::ops::Range;
use tftn_sprachen::tr;

use serde::{Deserialize, Serialize};
use typst_syntax::ast;
use typst_syntax::{LinkedNode, Side, Source, SyntaxKind};

use crate::{Patch, PatchFehler, anwenden, fehler};

/// Höchste Überschriftenebene, die der Helfer setzt.
pub const EBENE_MAX: usize = 6;

/// Eine Stelle, an der ein Markup-Helfer greift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "art", rename_all = "kebab-case")]
pub enum MarkupStelle {
    Ueberschrift {
        ebene: usize,
        marke: Range<usize>,
    },
    /// Alle Punkte der Liste bzw. Aufzählung (gleiche Art, nur durch Leerraum getrennt).
    Liste {
        aufzaehlung: bool,
        marken: Vec<Range<usize>>,
    },
    Formel {
        block: bool,
        bereich: Range<usize>,
    },
    /// `@schluessel` bzw. `@schluessel[Zusatz]` (PLAN-M2 AP11); `zusatz` ohne Klammern und Rand.
    Zitat {
        bereich: Range<usize>,
        schluessel: String,
        zusatz: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarkupAktion {
    /// ein `=` mehr (tiefer)
    EbenePlus,
    /// ein `=` weniger (höher)
    EbeneMinus,
    ListeUmschalten,
    FormelUmschalten,
}

/// Alle Stellen am Cursor (Byte), innerste zuerst.
pub fn markup_stellen(quelle: &Source, byte: usize) -> Vec<MarkupStelle> {
    let wurzel = LinkedNode::new(quelle.root());
    let mut knoten = wurzel
        .leaf_at(byte, Side::After)
        .or_else(|| wurzel.leaf_at(byte, Side::Before));
    let mut aus = Vec::new();
    while let Some(k) = knoten {
        match k.kind() {
            SyntaxKind::Heading => {
                if let Some(m) = k.children().find(|c| c.kind() == SyntaxKind::HeadingMarker) {
                    aus.push(MarkupStelle::Ueberschrift {
                        ebene: m.len(),
                        marke: m.range(),
                    });
                }
            }
            SyntaxKind::ListItem | SyntaxKind::EnumItem => aus.push(liste(&k)),
            // Eine offene Formel (`$` beim Tippen) ist noch keine Stelle.
            SyntaxKind::Equation
                if k.children()
                    .filter(|c| c.kind() == SyntaxKind::Dollar)
                    .count()
                    == 2 =>
            {
                if let Some(e) = k.cast::<ast::Equation>() {
                    aus.push(MarkupStelle::Formel {
                        block: e.block(),
                        bereich: k.range(),
                    });
                }
            }
            SyntaxKind::Ref => aus.extend(zitat(quelle, &k)),
            _ => {}
        }
        knoten = k.parent().cloned();
    }
    // Direkt hinter einem Zitat zählt es noch: Dort steht der Cursor nach dem Einfügen.
    if !aus.iter().any(|s| matches!(s, MarkupStelle::Zitat { .. })) {
        let mut davor = wurzel.leaf_at(byte, Side::Before);
        while let Some(k) = davor {
            if k.kind() == SyntaxKind::Ref && k.range().end == byte {
                if let Some(z) = zitat(quelle, &k) {
                    aus.insert(0, z);
                }
                break;
            }
            davor = k.parent().cloned();
        }
    }
    aus
}

/// Das Zitat im `Ref`-Knoten `k`.
fn zitat(quelle: &Source, k: &LinkedNode) -> Option<MarkupStelle> {
    let verweis = k.cast::<ast::Ref>()?;
    let block = k.children().find(|c| c.kind() == SyntaxKind::ContentBlock);
    // Beim Tippen steht `[` kurz offen; das ist noch kein Zitat, das man ändern kann.
    if block
        .as_ref()
        .is_some_and(|b| b.children().last().map(|c| c.kind()) != Some(SyntaxKind::RightBracket))
    {
        return None;
    }
    let zusatz = block
        .map(|c| quelle.text()[c.range().start + 1..c.range().end - 1].trim())
        .filter(|z| !z.is_empty())
        .map(str::to_string);
    Some(MarkupStelle::Zitat {
        bereich: k.range(),
        schluessel: verweis.target().to_string(),
        zusatz,
    })
}

/// Die Folge gleichartiger Punkte um `punkt`, getrennt nur durch Leerraum und Kommentare.
fn liste(punkt: &LinkedNode) -> MarkupStelle {
    let art = punkt.kind();
    let geschwister: Vec<LinkedNode> = match punkt.parent() {
        Some(p) => p.children().collect(),
        None => vec![punkt.clone()],
    };
    let i = geschwister
        .iter()
        .position(|g| g.range() == punkt.range())
        .unwrap_or(0);
    // Kommentare verwirft Typst beim Auswerten; sie trennen Punkte also nicht.
    let gehoert = |g: &LinkedNode| {
        g.kind() == art
            || matches!(
                g.kind(),
                SyntaxKind::Space
                    | SyntaxKind::Parbreak
                    | SyntaxKind::LineComment
                    | SyntaxKind::BlockComment
            )
    };
    let mut von = i;
    while von > 0 && gehoert(&geschwister[von - 1]) {
        von -= 1;
    }
    let mut bis = i;
    while bis + 1 < geschwister.len() && gehoert(&geschwister[bis + 1]) {
        bis += 1;
    }
    let marken = geschwister[von..=bis]
        .iter()
        .filter(|g| g.kind() == art)
        .filter_map(|g| {
            g.children()
                .find(|c| matches!(c.kind(), SyntaxKind::ListMarker | SyntaxKind::EnumMarker))
                .map(|m| m.range())
        })
        .collect();
    MarkupStelle::Liste {
        aufzaehlung: art == SyntaxKind::EnumItem,
        marken,
    }
}

/// Patch für `aktion` an `stelle`; geprüft nach (a)–(c).
pub fn markup_patch(
    quelle: &Source,
    stelle: &MarkupStelle,
    aktion: MarkupAktion,
) -> Result<Patch, PatchFehler> {
    let text = quelle.text();
    let patch = match (stelle, aktion) {
        (
            MarkupStelle::Ueberschrift { ebene, marke },
            MarkupAktion::EbenePlus | MarkupAktion::EbeneMinus,
        ) => {
            let neu = match aktion {
                MarkupAktion::EbenePlus => ebene + 1,
                _ => ebene.saturating_sub(1),
            };
            // Hinauf nur bis EBENE_MAX; zurückstufen geht auch von tieferen Ebenen aus dem Text.
            let zu_tief = matches!(aktion, MarkupAktion::EbenePlus) && neu > EBENE_MAX;
            if neu < 1 || zu_tief {
                return Err(PatchFehler::UngueltigerWert(tr!(
                    "Ebene {neu} außerhalb von 1..={max}",
                    neu,
                    max = EBENE_MAX
                )));
            }
            Patch {
                bereich: marke.clone(),
                text: "=".repeat(neu),
            }
        }
        (
            MarkupStelle::Liste {
                aufzaehlung,
                marken,
            },
            MarkupAktion::ListeUmschalten,
        ) => {
            let (Some(erste), Some(letzte)) = (marken.first(), marken.last()) else {
                return Err(PatchFehler::UngueltigerWert(tr!("Liste ohne Punkte")));
            };
            if *aufzaehlung {
                eigene_nummern_pruefen(text, marken)?;
            }
            let neu_marke = if *aufzaehlung { "-" } else { "+" };
            let mut aus = String::new();
            let mut pos = erste.start;
            for m in marken {
                aus.push_str(&text[pos..m.start]);
                aus.push_str(neu_marke);
                pos = m.end;
            }
            Patch {
                bereich: erste.start..letzte.end,
                text: aus,
            }
        }
        (MarkupStelle::Formel { block, bereich }, MarkupAktion::FormelUmschalten) => {
            let innen = &text[bereich.start + 1..bereich.end - 1];
            Patch {
                bereich: bereich.clone(),
                text: if *block {
                    formel_quelltext(innen.trim(), false)
                } else {
                    formel_quelltext(innen, true)
                },
            }
        }
        _ => {
            return Err(PatchFehler::UngueltigerWert(tr!(
                "{aktion} passt nicht zu dieser Stelle",
                aktion = format!("{aktion:?}")
            )));
        }
    };
    pruefen(quelle, stelle, &patch)?;
    Ok(patch)
}

/// Eine Aufzählung mit eigenen Nummern (`3.`, Sprünge wie `1.` → `10.`) verlöre sie als Liste, und
/// zurück kämen sie nicht wieder. Nummern, die Typst ohnehin vergäbe, sind kein Verlust.
fn eigene_nummern_pruefen(text: &str, marken: &[Range<usize>]) -> Result<(), PatchFehler> {
    let mut naechste = 1usize;
    for m in marken {
        let marke = &text[m.clone()];
        if let Some(n) = marke
            .strip_suffix('.')
            .and_then(|z| z.parse::<usize>().ok())
        {
            if n != naechste {
                return Err(PatchFehler::UngueltigerWert(tr!(
                    "Die Aufzählung hat eigene Nummern ({marke}); als Liste gingen sie verloren – bitte im Text umstellen",
                    marke
                )));
            }
            naechste = n;
        }
        naechste += 1;
    }
    Ok(())
}

/// Text der Formel ohne `$` und ohne Leerraum am Rand; `None`, wenn `stelle` keine Formel ist.
pub fn formel_text(quelle: &Source, stelle: &MarkupStelle) -> Option<String> {
    let MarkupStelle::Formel { bereich, .. } = stelle else {
        return None;
    };
    Some(
        quelle.text()[bereich.start + 1..bereich.end - 1]
            .trim()
            .to_string(),
    )
}

/// Neuer Text einer Formel (PLAN-M2 AP10). Ersetzt wird nur der Kern zwischen dem Leerraum am Rand,
/// Block bzw. Im Satz bleibt also. Geprüft: außerhalb kein Byte anders, kein neuer Fehlerknoten,
/// gleich viele Formeln, an der Stelle steht weiter eine Formel derselben Art mit dem neuen Text.
pub fn formel_patch(
    quelle: &Source,
    stelle: &MarkupStelle,
    neu: &str,
) -> Result<Patch, PatchFehler> {
    let MarkupStelle::Formel { block, bereich } = stelle else {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "An dieser Stelle steht keine Formel."
        )));
    };
    let neu = formel_leer_pruefen(neu)?;
    let innen_von = bereich.start + 1;
    let innen = &quelle.text()[innen_von..bereich.end - 1];
    // Endet die neue Formel in einem `//`-Kommentar, muss vor dem schließenden `$` ein Umbruch stehen.
    let kommentar = neu.lines().last().is_some_and(|z| z.contains("//"));
    let mit_umbruch = |hinten: &str| {
        if kommentar && !hinten.contains('\n') {
            format!("{neu}\n")
        } else {
            neu.to_string()
        }
    };
    let patch = if innen.trim().is_empty() {
        Patch {
            bereich: innen_von..bereich.end - 1,
            text: mit_umbruch(""),
        }
    } else {
        let vorne = innen.len() - innen.trim_start().len();
        let ende = vorne + innen.trim().len();
        Patch {
            bereich: innen_von + vorne..innen_von + ende,
            text: mit_umbruch(&innen[ende..]),
        }
    };
    let ergebnis = Source::detached(anwenden(quelle.text(), &patch));
    formel_ergebnis_pruefen(quelle, &ergebnis, 0)?;
    let danach = markup_stellen(&ergebnis, patch.bereich.start)
        .into_iter()
        .find(|s| matches!(s, MarkupStelle::Formel { .. }));
    match danach {
        Some(MarkupStelle::Formel {
            block: b,
            bereich: r,
        }) if b == *block && ergebnis.text()[r.start + 1..r.end - 1].trim() == neu => Ok(patch),
        _ => Err(formel_abgelehnt()),
    }
}

/// Neue Quelle und Seitenangabe eines Zitats (PLAN-M2 AP11); `zusatz` leer oder `None` entfernt die
/// Angabe. Geprüft: kein neuer Fehlerknoten, gleich viele Verweise, an der Stelle steht danach genau
/// dieses Zitat. Was nicht als `@`-Verweis geht, wird abgelehnt.
pub fn zitat_patch(
    quelle: &Source,
    stelle: &MarkupStelle,
    schluessel: &str,
    zusatz: Option<&str>,
) -> Result<Patch, PatchFehler> {
    let MarkupStelle::Zitat { bereich, .. } = stelle else {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "An dieser Stelle steht kein Zitat."
        )));
    };
    let gelesen = |q: &Source, byte: usize| {
        markup_stellen(q, byte)
            .into_iter()
            .find(|s| matches!(s, MarkupStelle::Zitat { bereich, .. } if bereich.start == byte))
    };
    let allein = Source::detached(format!("@{schluessel}"));
    if !matches!(
        gelesen(&allein, 0),
        Some(MarkupStelle::Zitat { schluessel: s, .. }) if s == schluessel
    ) {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "Diese Quelle lässt sich hier nur im Code eintragen."
        )));
    }
    let zusatz = zusatz.map(str::trim).filter(|z| !z.is_empty());
    // `#` startet Code: `@a[#x]` wäre gültige Syntax, übersetzt aber nicht mehr.
    fn raute(n: &typst_syntax::SyntaxNode) -> bool {
        n.kind() == SyntaxKind::Hash || n.children().any(raute)
    }
    if zusatz.is_some_and(|z| raute(Source::detached(z).root())) {
        return Err(PatchFehler::UngueltigerWert(tr!(
            "Diese Seitenangabe enthält Zeichen, die hier nicht gehen."
        )));
    }
    let patch = Patch {
        bereich: bereich.clone(),
        text: match zusatz {
            Some(z) => format!("@{schluessel}[{z}]"),
            None => format!("@{schluessel}"),
        },
    };
    fn zahl(n: &typst_syntax::SyntaxNode) -> usize {
        usize::from(n.kind() == SyntaxKind::Ref) + n.children().map(zahl).sum::<usize>()
    }
    let ergebnis = Source::detached(anwenden(quelle.text(), &patch));
    let ende = patch.bereich.start + patch.text.len();
    let passt = fehler(ergebnis.root()) <= fehler(quelle.root())
        && zahl(ergebnis.root()) == zahl(quelle.root())
        && matches!(
            gelesen(&ergebnis, patch.bereich.start),
            Some(MarkupStelle::Zitat { bereich: r, schluessel: s, zusatz: z })
                if r.end == ende && s == schluessel && z.as_deref() == zusatz
        );
    if passt {
        Ok(patch)
    } else if zusatz.is_some() {
        Err(PatchFehler::UngueltigerWert(tr!(
            "Diese Seitenangabe enthält Zeichen, die hier nicht gehen."
        )))
    } else {
        Err(PatchFehler::UngueltigerWert(tr!(
            "Ohne Seitenangabe würde das Zitat mit dem Text dahinter verschmelzen."
        )))
    }
}

/// `$ innen $` (Block) bzw. `$innen$`. Endet die letzte Zeile in einem `//`-Kommentar, käme das
/// schließende `$` mit in den Kommentar; dann steht es in einer eigenen Zeile.
pub fn formel_quelltext(innen: &str, block: bool) -> String {
    let kommentar = innen.lines().last().is_some_and(|z| z.contains("//"));
    match (block, kommentar) {
        (true, false) => format!("$ {innen} $"),
        (true, true) => format!("$ {innen}\n$"),
        (false, false) => format!("${innen}$"),
        (false, true) => format!("${innen}\n$"),
    }
}

/// Neue Formel bei `byte`: `$ neu $` (Block) oder `$neu$` (im Satz). Nur im Markup.
pub fn formel_einfuegen(
    quelle: &Source,
    byte: usize,
    neu: &str,
    block: bool,
) -> Result<Patch, PatchFehler> {
    let neu = formel_leer_pruefen(neu)?;
    let patch = Patch {
        bereich: byte..byte,
        text: formel_quelltext(neu, block),
    };
    let ergebnis = Source::detached(anwenden(quelle.text(), &patch));
    formel_ergebnis_pruefen(quelle, &ergebnis, 1)?;
    let wurzel = LinkedNode::new(ergebnis.root());
    let mut knoten = wurzel.leaf_at(byte, Side::After);
    while let Some(k) = knoten {
        if k.kind() == SyntaxKind::Equation {
            return if k.offset() == byte
                && k.parent().is_some_and(|e| e.kind() == SyntaxKind::Markup)
            {
                Ok(patch)
            } else {
                Err(formel_nur_im_text())
            };
        }
        knoten = k.parent().cloned();
    }
    Err(formel_nur_im_text())
}

fn formel_leer_pruefen(neu: &str) -> Result<&str, PatchFehler> {
    let neu = neu.trim();
    if neu.is_empty() {
        return Err(PatchFehler::UngueltigerWert(tr!("Die Formel ist leer.")));
    }
    Ok(neu)
}

fn formel_abgelehnt() -> PatchFehler {
    PatchFehler::UngueltigerWert(tr!("Die Formel ist so nicht gültig (Syntaxfehler)."))
}

fn formel_nur_im_text() -> PatchFehler {
    PatchFehler::UngueltigerWert(tr!("Eine neue Formel geht nur im Text."))
}

/// Kein neuer Fehlerknoten und genau `dazu` Formeln mehr als vorher.
fn formel_ergebnis_pruefen(alt: &Source, neu: &Source, dazu: usize) -> Result<(), PatchFehler> {
    fn zahl(n: &typst_syntax::SyntaxNode) -> usize {
        usize::from(n.kind() == SyntaxKind::Equation) + n.children().map(zahl).sum::<usize>()
    }
    if fehler(neu.root()) > fehler(alt.root()) || zahl(neu.root()) != zahl(alt.root()) + dazu {
        return Err(formel_abgelehnt());
    }
    Ok(())
}

/// (a) kein Byte außerhalb, (c) kein neuer Fehlerknoten, (b) die Stelle trägt den neuen Zustand
/// und dieselben Inhalte.
fn pruefen(quelle: &Source, stelle: &MarkupStelle, patch: &Patch) -> Result<(), PatchFehler> {
    let alt = quelle.text();
    let neu_text = anwenden(alt, patch);
    let ende_neu = patch.bereich.start + patch.text.len();
    if neu_text[..patch.bereich.start] != alt[..patch.bereich.start]
        || neu_text[ende_neu..] != alt[patch.bereich.end..]
    {
        return Err(PatchFehler::Invariante(
            "(a) Bytes außerhalb des Bereichs geändert",
        ));
    }
    let neu = Source::detached(neu_text);
    if fehler(neu.root()) > fehler(quelle.root()) {
        return Err(PatchFehler::Invariante("(c) neuer Fehlerknoten"));
    }
    let gleiche_art =
        |s: &MarkupStelle| std::mem::discriminant(s) == std::mem::discriminant(stelle);
    let danach = markup_stellen(&neu, patch.bereich.start)
        .into_iter()
        .find(gleiche_art)
        .ok_or(PatchFehler::Invariante(
            "(b) Stelle nach dem Patch nicht mehr da",
        ))?;
    let zustand_neu = match (stelle, &danach) {
        (MarkupStelle::Ueberschrift { .. }, MarkupStelle::Ueberschrift { ebene, .. }) => {
            *ebene == patch.text.len()
        }
        (
            MarkupStelle::Liste {
                aufzaehlung,
                marken,
            },
            MarkupStelle::Liste {
                aufzaehlung: a,
                marken: m,
            },
        ) => a != aufzaehlung && m.len() == marken.len(),
        (MarkupStelle::Formel { block, .. }, MarkupStelle::Formel { block: b, .. }) => b != block,
        _ => false,
    };
    if !zustand_neu {
        return Err(PatchFehler::Invariante(
            "(b) Stelle trägt nicht den neuen Zustand",
        ));
    }
    if inhalte(quelle, stelle) != inhalte(&neu, &danach) {
        return Err(PatchFehler::Invariante("(b) Inhalt der Stelle geändert"));
    }
    Ok(())
}

/// Was der Helfer nicht ändern darf: Text der Überschrift, Text jedes Punkts, Formel ohne Rand.
fn inhalte(quelle: &Source, stelle: &MarkupStelle) -> Vec<String> {
    let rumpf = |pos: usize, art: &[SyntaxKind]| -> String {
        let wurzel = LinkedNode::new(quelle.root());
        let mut k = wurzel.leaf_at(pos, Side::After);
        while let Some(n) = k {
            if art.contains(&n.kind()) {
                return n
                    .children()
                    .find(|c| c.kind() == SyntaxKind::Markup)
                    .map(|m| m.get().full_text().to_string())
                    .unwrap_or_default();
            }
            k = n.parent().cloned();
        }
        String::new()
    };
    match stelle {
        MarkupStelle::Ueberschrift { marke, .. } => {
            vec![rumpf(marke.start, &[SyntaxKind::Heading])]
        }
        MarkupStelle::Liste { marken, .. } => marken
            .iter()
            .map(|m| rumpf(m.start, &[SyntaxKind::ListItem, SyntaxKind::EnumItem]))
            .collect(),
        MarkupStelle::Formel { bereich, .. } | MarkupStelle::Zitat { bereich, .. } => {
            vec![
                quelle.text()[bereich.start + 1..bereich.end - 1]
                    .trim()
                    .to_string(),
            ]
        }
    }
}
