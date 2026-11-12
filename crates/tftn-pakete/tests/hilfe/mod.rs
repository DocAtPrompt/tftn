//! Archive für Tests bauen (PLAN-M2 AP9). Böse Namen werden direkt in den Kopf geschrieben, weil
//! `tar::Header::set_path` sie ablehnt.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{EntryType, Header};

/// Ein Eintrag: Art, roher Name, Inhalt, Ziel einer Verknüpfung.
pub struct Eintrag {
    pub art: EntryType,
    pub name: Vec<u8>,
    pub inhalt: Vec<u8>,
    pub verweis: Option<String>,
}

pub fn datei(name: &str, inhalt: &str) -> Eintrag {
    Eintrag {
        art: EntryType::Regular,
        name: name.as_bytes().to_vec(),
        inhalt: inhalt.as_bytes().to_vec(),
        verweis: None,
    }
}

pub fn ordner_eintrag(name: &str) -> Eintrag {
    Eintrag {
        art: EntryType::Directory,
        name: name.as_bytes().to_vec(),
        inhalt: Vec::new(),
        verweis: None,
    }
}

pub fn verweis(art: EntryType, name: &str, ziel: &str) -> Eintrag {
    Eintrag {
        art,
        name: name.as_bytes().to_vec(),
        inhalt: Vec::new(),
        verweis: Some(ziel.into()),
    }
}

pub fn typst_toml(name: &str, version: &str) -> Eintrag {
    datei(
        "typst.toml",
        &format!(
            "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"lib.typ\"\nlicense = \"MIT-0\"\n"
        ),
    )
}

/// Baut ein .tar.gz aus den Einträgen (Reihenfolge bleibt).
pub fn archiv(eintraege: &[Eintrag]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for e in eintraege {
        let mut h = Header::new_old();
        {
            let alt = h.as_old_mut();
            alt.name[..e.name.len()].copy_from_slice(&e.name);
        }
        h.set_entry_type(e.art);
        h.set_mode(0o644);
        h.set_mtime(0);
        h.set_size(e.inhalt.len() as u64);
        if let Some(v) = e.verweis.as_ref().filter(|v| !v.is_empty()) {
            h.set_link_name(v).unwrap();
        }
        h.set_cksum();
        tar.append(&h, e.inhalt.as_slice()).unwrap();
    }
    gz(&tar.into_inner().unwrap())
}

/// Ein Eintrag mit `groesse` Nullbytes, ohne den Inhalt im Speicher zu halten.
pub fn archiv_mit_nullen(vorher: &[Eintrag], name: &str, groesse: u64) -> Vec<u8> {
    let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
    for e in vorher {
        let mut h = Header::new_gnu();
        h.set_path(std::str::from_utf8(&e.name).unwrap()).unwrap();
        h.set_size(e.inhalt.len() as u64);
        h.set_mode(0o644);
        h.set_cksum();
        tar.append(&h, e.inhalt.as_slice()).unwrap();
    }
    let mut h = Header::new_gnu();
    h.set_path(name).unwrap();
    h.set_size(groesse);
    h.set_mode(0o644);
    h.set_cksum();
    tar.append(&h, std::io::Read::take(std::io::repeat(0), groesse))
        .unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

/// Ein Kopf der Art `art` mit `groesse` Bytes Inhalt (Buchstabe a), danach typst.toml.
pub fn archiv_mit_kopf(art: EntryType, groesse: u64) -> Vec<u8> {
    let mut tar = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
    let mut h = Header::new_gnu();
    {
        let alt = h.as_old_mut();
        let name = b"././@LongLink";
        alt.name[..name.len()].copy_from_slice(name);
    }
    h.set_entry_type(art);
    h.set_size(groesse);
    h.set_mode(0o644);
    h.set_cksum();
    tar.append(&h, std::io::Read::take(std::io::repeat(b'a'), groesse))
        .unwrap();
    let t = typst_toml("tftn-probe", "0.1.0");
    let mut h = Header::new_gnu();
    h.set_path("typst.toml").unwrap();
    h.set_size(t.inhalt.len() as u64);
    h.set_mode(0o644);
    h.set_cksum();
    tar.append(&h, t.inhalt.as_slice()).unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

pub fn gz(daten: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(daten).unwrap();
    e.finish().unwrap()
}

/// Leerer Testordner unter `CARGO_TARGET_TMPDIR`.
pub fn testordner(name: &str) -> PathBuf {
    let o = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ap9-{name}"));
    let _ = std::fs::remove_dir_all(&o);
    std::fs::create_dir_all(&o).unwrap();
    o
}

/// Namen der Einträge in `ordner` (sortiert).
pub fn inhalt(ordner: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(ordner)
        .map(|r| {
            r.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}
