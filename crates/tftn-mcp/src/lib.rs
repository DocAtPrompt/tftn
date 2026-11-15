//! MCP (Model Context Protocol) für TFTN.
//!
//! Ein KI-Assistent (etwa Claude) startet `tftn-app --mcp` als Brücke und spricht mit ihr JSON-RPC
//! über stdin/stdout ([`bruecke`]). Die Brücke reicht jeden Werkzeugaufruf über eine lokale
//! Verbindung (nur `127.0.0.1`, mit Zufallsschlüssel aus [`Verbindung`]) an die laufende App weiter.
//! Die App führt ihn aus und zeigt Änderungen sofort im Editor. Läuft die App nicht oder ist MCP
//! dort ausgeschaltet, gibt es die Verbindungsdatei nicht und der Aufruf endet mit einer Meldung.
//!
//! Ohne Netz und ohne weitere Abhängigkeiten: JSON-RPC ist hier von Hand geschrieben.

mod protokoll;
mod text;
mod verbindung;
mod werkzeuge;

pub use protokoll::bruecke;
pub use text::{ersetzen, kleinster_bereich, zeile_spalte};
pub use verbindung::{Auftrag, VERBINDUNGSDATEI, Verbindung, schluessel, weiterreichen};
pub use werkzeuge::werkzeuge;

use serde::{Deserialize, Serialize};

/// Ergebnis eines Werkzeugaufrufs, wie MCP es als `CallToolResult` erwartet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ergebnis {
    #[serde(rename = "content")]
    pub inhalt: Vec<Inhalt>,
    #[serde(rename = "isError", default)]
    pub fehler: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Inhalt {
    Text {
        text: String,
    },
    Image {
        /// PNG, Base64
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
}

impl Ergebnis {
    pub fn text(text: impl Into<String>) -> Ergebnis {
        Ergebnis {
            inhalt: vec![Inhalt::Text { text: text.into() }],
            fehler: false,
        }
    }

    pub fn fehler(text: impl Into<String>) -> Ergebnis {
        Ergebnis {
            inhalt: vec![Inhalt::Text { text: text.into() }],
            fehler: true,
        }
    }
}
