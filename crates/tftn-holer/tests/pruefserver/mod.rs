//! Lokaler Prüfserver für die Tests des Holers (PLAN-M2 AP9): HTTP/1.1 auf 127.0.0.1, eine Antwort
//! je Verbindung, zählt die angenommenen Verbindungen. Kein Netz.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use flate2::Compression;
use flate2::write::GzEncoder;

#[derive(Clone)]
pub enum Rumpf {
    Bytes(Vec<u8>),
    /// So viele Nullbytes, in Stücken geschrieben
    Nullen(u64),
}

#[derive(Clone)]
pub struct Antwort {
    pub status: u16,
    pub kopf: Vec<(String, String)>,
    pub rumpf: Rumpf,
}

pub fn ok(rumpf: Vec<u8>) -> Antwort {
    Antwort {
        status: 200,
        kopf: Vec::new(),
        rumpf: Rumpf::Bytes(rumpf),
    }
}

pub struct Server {
    pub basis: String,
    verbindungen: Arc<AtomicUsize>,
    wege: Arc<Mutex<HashMap<String, Antwort>>>,
}

impl Server {
    pub fn starten() -> Server {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let basis = format!("http://{}", l.local_addr().unwrap());
        let verbindungen = Arc::new(AtomicUsize::new(0));
        let wege: Arc<Mutex<HashMap<String, Antwort>>> = Arc::default();
        let (v, w) = (verbindungen.clone(), wege.clone());
        std::thread::spawn(move || {
            for strom in l.incoming() {
                let Ok(mut strom) = strom else { continue };
                v.fetch_add(1, Ordering::SeqCst);
                let w = w.clone();
                std::thread::spawn(move || {
                    let mut leser = BufReader::new(strom.try_clone().unwrap());
                    let mut zeile = String::new();
                    if leser.read_line(&mut zeile).is_err() {
                        return;
                    }
                    let pfad = zeile.split_whitespace().nth(1).unwrap_or("").to_string();
                    loop {
                        let mut k = String::new();
                        if leser.read_line(&mut k).unwrap_or(0) == 0 || k == "\r\n" {
                            break;
                        }
                    }
                    let a = w.lock().unwrap().get(&pfad).cloned().unwrap_or(Antwort {
                        status: 404,
                        kopf: Vec::new(),
                        rumpf: Rumpf::Bytes(b"nicht da".to_vec()),
                    });
                    let laenge = match &a.rumpf {
                        Rumpf::Bytes(b) => b.len() as u64,
                        Rumpf::Nullen(n) => *n,
                    };
                    let mut kopf = format!(
                        "HTTP/1.1 {} X\r\nContent-Length: {laenge}\r\nConnection: close\r\n",
                        a.status
                    );
                    for (n, wert) in &a.kopf {
                        kopf.push_str(&format!("{n}: {wert}\r\n"));
                    }
                    kopf.push_str("\r\n");
                    if strom.write_all(kopf.as_bytes()).is_err() {
                        return;
                    }
                    match a.rumpf {
                        Rumpf::Bytes(b) => {
                            let _ = strom.write_all(&b);
                        }
                        Rumpf::Nullen(mut n) => {
                            let stueck = vec![0u8; 64 * 1024];
                            while n > 0 {
                                let k = n.min(stueck.len() as u64) as usize;
                                if strom.write_all(&stueck[..k]).is_err() {
                                    return;
                                }
                                n -= k as u64;
                            }
                        }
                    }
                });
            }
        });
        Server {
            basis,
            verbindungen,
            wege,
        }
    }

    pub fn weg(&self, pfad: &str, a: Antwort) {
        self.wege.lock().unwrap().insert(pfad.into(), a);
    }

    pub fn verbindungen(&self) -> usize {
        self.verbindungen.load(Ordering::SeqCst)
    }

    /// Bietet ein Testpaket aus `tests/pakete/<name>/<version>/` als Archiv an; gibt die Bytes zurück.
    pub fn paket(&self, name: &str, version: &str) -> Vec<u8> {
        let daten = archiv_aus(&testpaket(name, version));
        self.weg(
            &format!("/preview/{name}-{version}.tar.gz"),
            ok(daten.clone()),
        );
        daten
    }

    /// Index mit den angegebenen (Name, Version).
    pub fn index(&self, pakete: &[(&str, &str)]) {
        let eintraege: Vec<String> = pakete
            .iter()
            .map(|(n, v)| format!(r#"{{"name":"{n}","version":"{v}","description":"Test"}}"#))
            .collect();
        self.weg(
            "/preview/index.json",
            ok(format!("[{}]", eintraege.join(",")).into_bytes()),
        );
    }
}

pub fn testpaket(name: &str, version: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/pakete")
        .join(name)
        .join(version)
}

/// .tar.gz eines Ordners, Einträge relativ zur Wurzel (wie die Archive des Universe).
pub fn archiv_aus(ordner: &Path) -> Vec<u8> {
    let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    let mut dateien: Vec<_> = std::fs::read_dir(ordner)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dateien.sort();
    for d in dateien {
        let mut h = tar::Header::new_gnu();
        let inhalt = std::fs::read(&d).unwrap();
        h.set_path(d.file_name().unwrap()).unwrap();
        h.set_size(inhalt.len() as u64);
        h.set_mode(0o644);
        h.set_mtime(0);
        h.set_cksum();
        tar.append(&h, inhalt.as_slice()).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap()
}

pub fn testordner(name: &str) -> std::path::PathBuf {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ap9-holer-{name}"));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    o
}
