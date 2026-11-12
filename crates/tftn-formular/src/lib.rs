//! Fach „Dokument“ (PLAN-M2 AP7): Formular-Schema lesen und gegen das Profil „TFTN-Schema 1“ prüfen
//! (KONZEPT 6), Werte am Feld prüfen und `dokument.toml` verlustfrei schreiben (KONZEPT 4.7).

pub mod schema;

pub use schema::{Art, Feld, Formular, ProfilFehler, TextWidget, formular_lesen, sensibel};
pub mod pruefen;

pub use pruefen::{Kontext, Meldung, Pfad, Schritt, alles_pruefen, wert_pruefen};
pub mod daten;
pub mod historie;

pub use daten::{Daten, EintragAktion, atomar_schreiben, literatur_dateien, stand};
pub use historie::Historie;
