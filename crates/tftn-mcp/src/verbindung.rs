//! Lokale Verbindung zwischen Brücke und App: TCP nur auf `127.0.0.1`, je Aufruf eine Zeile hin
//! (Auftrag mit Schlüssel) und eine zurück (Ergebnis). Port und Schlüssel stehen in der
//! Verbindungsdatei im Konfigurationsordner der App; es gibt sie nur, solange MCP dort an ist.

use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Ergebnis;

/// Dateiname im Konfigurationsordner der App.
pub const VERBINDUNGSDATEI: &str = "mcp-verbindung.json";

/// Längster Aufruf: Öffnen wartet, bis der Benutzer ein Dokument ohne Namen benannt hat.
const GEDULD: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verbindung {
    pub port: u16,
    pub schluessel: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Auftrag {
    pub schluessel: String,
    pub werkzeug: String,
    pub argumente: Value,
}

/// 128 Bit Zufall aus dem Betriebssystem (über `RandomState`), hexadezimal.
pub fn schluessel() -> String {
    let teil = || {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u32(std::process::id());
        h.finish()
    };
    format!("{:016x}{:016x}", teil(), teil())
}

impl Verbindung {
    pub fn lesen(ordner: &Path) -> Option<Verbindung> {
        let text = std::fs::read_to_string(ordner.join(VERBINDUNGSDATEI)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Schreibt die Datei; unter Unix nur für den Benutzer lesbar.
    pub fn schreiben(&self, ordner: &Path) -> std::io::Result<()> {
        let pfad = ordner.join(VERBINDUNGSDATEI);
        let mut optionen = std::fs::OpenOptions::new();
        optionen.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            optionen.mode(0o600);
        }
        let mut f = optionen.open(&pfad)?;
        f.write_all(serde_json::to_string(self)?.as_bytes())
    }

    pub fn entfernen(ordner: &Path) {
        let _ = std::fs::remove_file(ordner.join(VERBINDUNGSDATEI));
    }
}

/// Reicht einen Werkzeugaufruf an die App weiter (Brückenseite). Jeder Fehlschlag wird zu einem
/// Ergebnis mit `isError`, damit der Assistent dem Benutzer sagen kann, was fehlt.
pub fn weiterreichen(ordner: &Path, werkzeug: &str, argumente: Value) -> Ergebnis {
    const AUS: &str = "TFTN läuft nicht, oder MCP ist in TFTN ausgeschaltet. Der Benutzer kann es in \
        TFTN unter Einstellungen → MCP einschalten.";
    let Some(v) = Verbindung::lesen(ordner) else {
        return Ergebnis::fehler(AUS);
    };
    let adresse = SocketAddr::from((Ipv4Addr::LOCALHOST, v.port));
    let Ok(mut strom) = TcpStream::connect_timeout(&adresse, Duration::from_secs(2)) else {
        return Ergebnis::fehler(AUS);
    };
    let _ = strom.set_read_timeout(Some(GEDULD));
    let auftrag = Auftrag {
        schluessel: v.schluessel,
        werkzeug: werkzeug.to_string(),
        argumente,
    };
    let gesendet = serde_json::to_string(&auftrag)
        .map_err(std::io::Error::other)
        .and_then(|t| writeln!(strom, "{t}"));
    if let Err(e) = gesendet {
        return Ergebnis::fehler(format!("Verbindung zu TFTN: {e}"));
    }
    let mut zeile = String::new();
    match BufReader::new(strom).read_line(&mut zeile) {
        Ok(0) => Ergebnis::fehler("TFTN hat die Verbindung ohne Antwort beendet."),
        Ok(_) => serde_json::from_str(&zeile)
            .unwrap_or_else(|e| Ergebnis::fehler(format!("unlesbare Antwort von TFTN: {e}"))),
        Err(e) => Ergebnis::fehler(format!("keine Antwort von TFTN: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn ordner(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("tftn-mcp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn schluessel_sind_lang_und_verschieden() {
        let a = schluessel();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, schluessel());
    }

    #[test]
    fn ohne_verbindungsdatei_eine_meldung() {
        let d = ordner("ohne");
        let e = weiterreichen(&d, "status", Value::Null);
        assert!(e.fehler);
        assert!(serde_json::to_string(&e).unwrap().contains("Einstellungen"));
    }

    #[test]
    fn auftrag_hin_ergebnis_zurueck() {
        let d = ordner("hin");
        let horcher = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let v = Verbindung {
            port: horcher.local_addr().unwrap().port(),
            schluessel: schluessel(),
        };
        v.schreiben(&d).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let modus = std::fs::metadata(d.join(VERBINDUNGSDATEI))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(modus & 0o777, 0o600);
        }
        assert_eq!(Verbindung::lesen(&d), Some(v.clone()));
        let erwartet = v.schluessel.clone();
        let app = std::thread::spawn(move || {
            let (strom, _) = horcher.accept().unwrap();
            let mut zeile = String::new();
            BufReader::new(&strom).read_line(&mut zeile).unwrap();
            let a: Auftrag = serde_json::from_str(&zeile).unwrap();
            assert_eq!(a.schluessel, erwartet);
            let antwort = Ergebnis::text(format!("{} {}", a.werkzeug, a.argumente["x"]));
            writeln!(&strom, "{}", serde_json::to_string(&antwort).unwrap()).unwrap();
        });
        let e = weiterreichen(&d, "status", serde_json::json!({ "x": 7 }));
        app.join().unwrap();
        assert_eq!(e, Ergebnis::text("status 7"));
        Verbindung::entfernen(&d);
        assert!(Verbindung::lesen(&d).is_none());
    }
}
