//! Seitenvergleich zweier Fassungen eines Projekts (PLAN-M2 AP8; AP9 nutzt ihn für Pakete).
//!
//! Beide Fassungen werden aus den Dateien auf der Platte übersetzt, jede Seite wird gerastert, und
//! die Bilder werden Byte für Byte verglichen. Ein Hash des Layouts taugt dafür nicht: Er enthält die
//! Quellstellen, und die wechseln mit der Paketversion, auch wenn die Seite gleich aussieht.

use crate::vorschau::rastern;
use crate::{Projekt, ProjektWorld, kompilieren};
use tftn_sprachen::tr;

/// Eine geänderte Seite: Nummer (0-basiert), PNG der alten und der neuen Fassung (fehlt, wenn es
/// die Seite dort nicht gibt).
pub type Seitenbild = (usize, Option<Vec<u8>>, Option<Vec<u8>>);

#[derive(Debug)]
pub struct Vergleich {
    pub seiten_alt: usize,
    pub seiten_neu: usize,
    /// 0-basierte Nummern der geänderten Seiten (Bild weicht ab oder fehlt in einer Fassung)
    pub geaendert: Vec<usize>,
    /// PNG je geänderter Seite (Seite, alt, neu), höchstens `bilder_max`
    pub bilder: Vec<Seitenbild>,
    /// Fehler beim Übersetzen der neuen Fassung (dann ist `geaendert` leer)
    pub fehler: Vec<String>,
}

/// Übersetzt und rastert alle Seiten; `Err` mit den Fehlermeldungen, wenn es nicht übersetzt.
fn seiten(projekt: &Projekt, ppi: f64) -> Result<Vec<Vec<u8>>, Vec<String>> {
    let world = ProjektWorld::neu(projekt).map_err(|e| vec![e.to_string()])?;
    let ergebnis = kompilieren(&world);
    let fehler: Vec<String> = ergebnis
        .meldungen
        .iter()
        .filter(|m| m.fehler)
        .map(|m| m.to_string())
        .collect();
    match ergebnis.dokument {
        Some(d) if fehler.is_empty() => Ok(d.pages().iter().map(|s| rastern(s, ppi)).collect()),
        _ => Err(if fehler.is_empty() {
            vec![tr!("kein Dokument erzeugt")]
        } else {
            fehler
        }),
    }
}

/// Vergleicht `alt` mit `neu`. Übersetzt `alt` nicht, ist das ein `Err`; übersetzt `neu` nicht,
/// stehen die Fehler in `fehler`.
pub fn seiten_vergleichen(
    alt: &Projekt,
    neu: &Projekt,
    ppi: f64,
    bilder_max: usize,
) -> Result<Vergleich, String> {
    let a = seiten(alt, ppi).map_err(|f| {
        tr!(
            "Die jetzige Fassung übersetzt nicht: {fehler}",
            fehler = f.join("; ")
        )
    })?;
    let n = match seiten(neu, ppi) {
        Ok(n) => n,
        Err(fehler) => {
            return Ok(Vergleich {
                seiten_alt: a.len(),
                seiten_neu: 0,
                geaendert: Vec::new(),
                bilder: Vec::new(),
                fehler,
            });
        }
    };
    let mut geaendert = Vec::new();
    let mut bilder = Vec::new();
    for i in 0..a.len().max(n.len()) {
        let (x, y) = (a.get(i), n.get(i));
        if x != y {
            geaendert.push(i);
            if bilder.len() < bilder_max {
                bilder.push((i, x.cloned(), y.cloned()));
            }
        }
    }
    Ok(Vergleich {
        seiten_alt: a.len(),
        seiten_neu: n.len(),
        geaendert,
        bilder,
        fehler: Vec::new(),
    })
}
