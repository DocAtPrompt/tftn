//! Lokale Verbindung zwischen Brücke und App: TCP nur auf `127.0.0.1`, je Aufruf eine Zeile hin
//! (Auftrag mit Schlüssel) und eine zurück (Ergebnis). Port und Schlüssel stehen in der
//! Verbindungsdatei im Konfigurationsordner der App; es gibt sie nur, solange MCP dort an ist.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Ergebnis;

/// Dateiname im Konfigurationsordner der App.
pub const VERBINDUNGSDATEI: &str = "mcp-verbindung.json";

/// Längster Aufruf: Öffnen wartet, bis der Benutzer ein Dokument ohne Namen benannt hat. Die App
/// teilt ihre Wartezeiten so ein, dass sie vorher antwortet (`src-tauri/src/mcp.rs`, `wechseln`).
pub const GEDULD: Duration = Duration::from_secs(300);

/// Größter Auftrag in Bytes (ein ganzer Text für `text_setzen` passt reichlich hinein).
pub const GRENZE_AUFTRAG: u64 = 64 << 20;

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

/// 128 Bit Zufall aus dem Zufallsgenerator des Betriebssystems, hexadezimal.
pub fn schluessel() -> String {
    let mut bytes = [0u8; 16];
    // Ohne Zufall des Betriebssystems kein sicherer Schlüssel; das gibt es auf keiner Zielplattform.
    getrandom::fill(&mut bytes).expect("Zufall des Betriebssystems nicht verfügbar");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Vergleicht Schlüssel ohne früh abzubrechen: Die Dauer verrät nicht, wie viele Zeichen stimmen.
pub fn schluessel_gleich(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |unterschied, (x, y)| unterschied | (x ^ y))
            == 0
}

/// Liest eine Zeile (den Auftrag) mit höchstens `grenze` Bytes; `None` bei sofortigem Ende.
pub fn auftrag_lesen(strom: impl std::io::Read, grenze: u64) -> std::io::Result<Option<String>> {
    let mut zeile = String::new();
    let n = BufReader::new(strom.take(grenze)).read_line(&mut zeile)?;
    if n == 0 {
        return Ok(None);
    }
    if !zeile.ends_with('\n') && n as u64 >= grenze {
        return Err(std::io::Error::other("Auftrag zu groß"));
    }
    Ok(Some(zeile))
}

impl Verbindung {
    pub fn lesen(ordner: &Path) -> Option<Verbindung> {
        let text = std::fs::read_to_string(ordner.join(VERBINDUNGSDATEI)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Schreibt die Datei; unter Unix nur für den Benutzer lesbar. Erst in eine frische Zwischendatei,
    /// dann umbenannt: Die Brücke liest nie eine halbe Datei, und die Rechte gelten auch, wenn
    /// schon eine Datei mit anderen Rechten dalag.
    pub fn schreiben(&self, ordner: &Path) -> std::io::Result<()> {
        let pfad = ordner.join(VERBINDUNGSDATEI);
        let zwischen = ordner.join(format!("{VERBINDUNGSDATEI}.{}.neu", std::process::id()));
        let _ = std::fs::remove_file(&zwischen);
        let mut optionen = std::fs::OpenOptions::new();
        optionen.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            optionen.mode(0o600);
        }
        let ergebnis = optionen
            .open(&zwischen)
            .and_then(|mut f| f.write_all(serde_json::to_string(self)?.as_bytes()))
            .and_then(|()| std::fs::rename(&zwischen, &pfad));
        if ergebnis.is_err() {
            let _ = std::fs::remove_file(&zwischen);
        }
        ergebnis
    }

    /// Entfernt die Datei nur, wenn sie noch diese Verbindung nennt: Eine zweite laufende Instanz
    /// kann sie inzwischen mit ihrer eigenen überschrieben haben.
    pub fn entfernen_wenn_eigen(&self, ordner: &Path) {
        if Verbindung::lesen(ordner).as_ref() == Some(self) {
            let _ = std::fs::remove_file(ordner.join(VERBINDUNGSDATEI));
        }
    }

    /// Beim Start: Eine liegengebliebene Datei (abgestürzte App, unlesbar) weg; die einer noch
    /// laufenden Instanz, die auf ihrem Port antwortet, bleibt.
    pub fn verwaiste_entfernen(ordner: &Path) {
        let pfad = ordner.join(VERBINDUNGSDATEI);
        if !pfad.exists() {
            return;
        }
        let lebt = Verbindung::lesen(ordner).is_some_and(|v| {
            let adresse = SocketAddr::from((Ipv4Addr::LOCALHOST, v.port));
            TcpStream::connect_timeout(&adresse, Duration::from_millis(300)).is_ok()
        });
        if !lebt {
            let _ = std::fs::remove_file(pfad);
        }
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
    fn zweite_instanz_entfernt_nur_ihre_eigene_datei() {
        let d = ordner("zwei");
        let erste = Verbindung {
            port: 1,
            schluessel: schluessel(),
        };
        let zweite = Verbindung {
            port: 2,
            schluessel: schluessel(),
        };
        erste.schreiben(&d).unwrap();
        zweite.schreiben(&d).unwrap();
        erste.entfernen_wenn_eigen(&d);
        assert_eq!(Verbindung::lesen(&d), Some(zweite.clone()));
        zweite.entfernen_wenn_eigen(&d);
        assert!(Verbindung::lesen(&d).is_none());
    }

    #[test]
    fn beim_start_bleibt_die_datei_einer_laufenden_instanz() {
        let d = ordner("laeuft");
        let horcher = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let lebt = Verbindung {
            port: horcher.local_addr().unwrap().port(),
            schluessel: schluessel(),
        };
        lebt.schreiben(&d).unwrap();
        Verbindung::verwaiste_entfernen(&d);
        assert_eq!(Verbindung::lesen(&d), Some(lebt.clone()));
        drop(horcher);
        Verbindung::verwaiste_entfernen(&d);
        assert!(Verbindung::lesen(&d).is_none());
        // Kaputte Datei gilt als verwaist.
        std::fs::write(d.join(VERBINDUNGSDATEI), "{").unwrap();
        Verbindung::verwaiste_entfernen(&d);
        assert!(!d.join(VERBINDUNGSDATEI).exists());
    }

    #[cfg(unix)]
    #[test]
    fn schreiben_macht_auch_eine_vorhandene_datei_privat() {
        use std::os::unix::fs::PermissionsExt;
        let d = ordner("privat");
        let pfad = d.join(VERBINDUNGSDATEI);
        std::fs::write(&pfad, "alt").unwrap();
        std::fs::set_permissions(&pfad, std::fs::Permissions::from_mode(0o644)).unwrap();
        let v = Verbindung {
            port: 3,
            schluessel: schluessel(),
        };
        v.schreiben(&d).unwrap();
        let modus = std::fs::metadata(&pfad).unwrap().permissions().mode();
        assert_eq!(modus & 0o777, 0o600);
        assert_eq!(Verbindung::lesen(&d), Some(v));
        let reste: Vec<_> = std::fs::read_dir(&d).unwrap().collect();
        assert_eq!(reste.len(), 1, "keine Zwischendatei bleibt liegen");
    }

    #[test]
    fn schluessel_vergleich() {
        assert!(schluessel_gleich("abc", "abc"));
        assert!(!schluessel_gleich("abc", "abd"));
        assert!(!schluessel_gleich("abc", "abcd"));
        assert!(!schluessel_gleich("", "a"));
    }

    #[test]
    fn auftrag_lesen_mit_grenze() {
        let kurz = b"{\"a\":1}\nRest";
        assert_eq!(
            auftrag_lesen(&kurz[..], 100).unwrap().as_deref(),
            Some("{\"a\":1}\n")
        );
        let lang = [b'x'; 200];
        assert!(auftrag_lesen(&lang[..], 100).is_err());
        assert_eq!(auftrag_lesen(&b""[..], 100).unwrap(), None);
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
        v.entfernen_wenn_eigen(&d);
        assert!(Verbindung::lesen(&d).is_none());
    }
}
