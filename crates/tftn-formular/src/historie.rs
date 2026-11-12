//! Eigene Historie des Fachs „Dokument“ (Spec 6): ein Schritt je geschriebener Änderung.
//!
//! Gespeichert werden ganze Dateitexte. Zurückgeschrieben wird nur, solange die Datei noch genau
//! dem Stand nach dem letzten eigenen Schreiben entspricht; eine fremde Änderung (Sync,
//! Texteditor) leert die Historie, damit nie ein fremder Stand zurückgedreht wird.

/// Höchstens so viele Schritte.
const GRENZE: usize = 100;

#[derive(Debug, Clone, Default)]
pub struct Historie {
    rueck: Vec<String>,
    vor: Vec<String>,
    /// Dateitext nach dem letzten eigenen Schreiben
    zuletzt: Option<String>,
}

impl Historie {
    /// Ein Schritt: Die Datei hatte `vorher` und hat jetzt `nachher`.
    pub fn geschrieben(&mut self, vorher: String, nachher: &str) {
        self.rueck.push(vorher);
        if self.rueck.len() > GRENZE {
            self.rueck.remove(0);
        }
        self.vor.clear();
        self.zuletzt = Some(nachher.to_string());
    }

    /// Leert die Historie, wenn `aktuell` nicht mehr der eigene letzte Stand ist.
    pub fn abgleichen(&mut self, aktuell: &str) {
        if self.zuletzt.as_deref().is_some_and(|z| z != aktuell) {
            *self = Self::default();
        }
    }

    /// Der Text, der zurückgeschrieben werden soll; `None`, wenn nichts da ist oder die Datei
    /// fremd geändert wurde (dann ist die Historie danach leer).
    pub fn rueckgaengig(&mut self, aktuell: &str) -> Option<String> {
        self.abgleichen(aktuell);
        let alt = self.rueck.pop()?;
        self.vor.push(aktuell.to_string());
        self.zuletzt = Some(alt.clone());
        Some(alt)
    }

    pub fn wiederholen(&mut self, aktuell: &str) -> Option<String> {
        self.abgleichen(aktuell);
        let neu = self.vor.pop()?;
        self.rueck.push(aktuell.to_string());
        self.zuletzt = Some(neu.clone());
        Some(neu)
    }

    /// (Rückgängig möglich, Wiederholen möglich)
    pub fn kann(&self) -> (bool, bool) {
        (!self.rueck.is_empty(), !self.vor.is_empty())
    }
}
