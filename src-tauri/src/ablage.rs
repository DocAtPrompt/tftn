//! Ergebnisse des Kompilierers (Vorschau, letzter Lauf), gebunden an die Sitzung, die sie erzeugt hat.
//!
//! Nach einem Dokumentwechsel kann der Kompilierer des alten Dokuments noch einen Lauf beenden. Jede
//! Sitzung bekommt deshalb eine Nummer; ein Kompilierer legt nur ab, solange seine Nummer die laufende
//! ist. Die Prüfung geschieht unter derselben Sperre wie das Schreiben, und `neue_sitzung` zählt weiter,
//! bevor `leeren` die Ablage räumt: Ein alter Lauf kann danach nichts mehr hineinschreiben.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// Beginnt eine neue Sitzung und gibt ihre Nummer.
pub fn neue_sitzung(laufend: &AtomicU64) -> u64 {
    laufend.fetch_add(1, Ordering::SeqCst) + 1
}

/// Ist `meine` noch die laufende Sitzung?
pub fn aktuell(laufend: &AtomicU64, meine: u64) -> bool {
    laufend.load(Ordering::SeqCst) == meine
}

pub fn leeren<T>(ziel: &Mutex<Option<T>>) {
    *ziel.lock().unwrap() = None;
}

/// Legt `wert` ab, wenn `meine` noch die laufende Sitzung ist.
pub fn ablegen<T>(
    ziel: &Mutex<Option<T>>,
    laufend: &AtomicU64,
    meine: u64,
    wert: impl FnOnce() -> T,
) -> bool {
    let mut z = ziel.lock().unwrap();
    if !aktuell(laufend, meine) {
        return false;
    }
    *z = Some(wert());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nach_dem_wechsel_legt_der_alte_kompilierer_nichts_mehr_ab() {
        let laufend = AtomicU64::new(0);
        let lauf = Mutex::new(None);
        let alt = neue_sitzung(&laufend);
        assert!(ablegen(&lauf, &laufend, alt, || "alt, Rev 7"));
        // Wechsel: neue Nummer, Ablage leer.
        let neu = neue_sitzung(&laufend);
        leeren(&lauf);
        assert_eq!(*lauf.lock().unwrap(), None);
        // Ein noch laufender Job des alten Dokuments endet jetzt.
        assert!(!ablegen(&lauf, &laufend, alt, || "alt, Rev 8"));
        assert_eq!(*lauf.lock().unwrap(), None);
        assert!(!aktuell(&laufend, alt));
        assert!(ablegen(&lauf, &laufend, neu, || "neu, Rev 0"));
        assert_eq!(*lauf.lock().unwrap(), Some("neu, Rev 0"));
    }
}
