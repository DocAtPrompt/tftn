//! Wächter für den Einstellungsordner (PLAN-M2 AP3, Task 2).

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use tftn_einstellungen::{TASTEN, beobachten};

struct Tmp(PathBuf);

impl Tmp {
    fn neu(name: &str) -> Tmp {
        let p = std::env::temp_dir().join(format!("tftn-waechter-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        // Unter macOS liegt temp_dir hinter einem Symlink (/var → /private/var); FSEvents meldet
        // den echten Pfad.
        Tmp(p.canonicalize().unwrap())
    }
}

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Startet den Wächter und wartet, bis er sicher steht.
fn starten(t: &Tmp) -> (notify::RecommendedWatcher, mpsc::Receiver<()>) {
    let (tx, rx) = mpsc::channel();
    let w = beobachten(&t.0, move || {
        let _ = tx.send(());
    })
    .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    // Was vor dem Start hereinkam, zählt nicht.
    while rx.try_recv().is_ok() {}
    (w, rx)
}

#[test]
fn schreiben_wird_gemeldet() {
    let t = Tmp::neu("schreiben");
    let (_w, rx) = starten(&t);
    fs::write(t.0.join(TASTEN), "[kuerzel]\n").unwrap();
    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
}

#[test]
fn umbenennen_wird_gemeldet() {
    let t = Tmp::neu("umbenennen");
    let (_w, rx) = starten(&t);
    fs::write(t.0.join("x.tmp"), "[kuerzel]\n").unwrap();
    fs::rename(t.0.join("x.tmp"), t.0.join(TASTEN)).unwrap();
    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
}

#[test]
fn andere_datei_still() {
    let t = Tmp::neu("still");
    let (_w, rx) = starten(&t);
    fs::write(t.0.join("notiz.txt"), "hallo").unwrap();
    fs::write(t.0.join(".tasten.toml.tmp"), "x").unwrap();
    assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
}

/// PLAN-M2 AP10: Eigene Paketbeschreibungen wirken ohne Neustart; andere Unterordner bleiben still.
#[test]
fn beobachten_meldet_beschreibung() {
    let t = Tmp::neu("beschreibung");
    fs::create_dir_all(t.0.join("beschreibungen")).unwrap();
    fs::create_dir_all(t.0.join("typen")).unwrap();
    let (_w, rx) = starten(&t);
    fs::write(t.0.join("typen/a.typ"), "x").unwrap();
    assert!(rx.recv_timeout(Duration::from_millis(800)).is_err());
    fs::write(t.0.join("beschreibungen/x.toml"), "paket = 1\n").unwrap();
    assert!(rx.recv_timeout(Duration::from_secs(5)).is_ok());
}
