//! Ein kleiner Zip-Schreiber für das Archivpaket: Deflate (oder gespeichert, wenn das kleiner ist),
//! Namen in UTF-8, ohne Zip64. Gleiche Dateien geben gleiche Bytes: Die Reihenfolge bestimmt der
//! Aufrufer, die Zeit jedes Eintrags ist der Zeitstempel des Projekts.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::Crc;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

const LOKAL: u32 = 0x0403_4b50;
const ZENTRAL: u32 = 0x0201_4b50;
const ENDE: u32 = 0x0605_4b50;
/// Bit 11: Name in UTF-8.
const UTF8: u16 = 0x0800;
/// Version 2.0 (Deflate); „erstellt von“ Unix, damit die Rechte 0644 gelten.
const VERSION: u16 = 20;
const ERSTELLT: u16 = (3 << 8) | VERSION;
const RECHTE: u32 = 0o100_644 << 16;

fn zu_gross() -> io::Error {
    io::Error::other("Archiv zu groß für Zip ohne Zip64 (über 4 GB oder 65 535 Dateien)")
}

/// Uhrzeit und Datum im DOS-Format (lokal ohne Zone; hier UTC), frühestens 1980.
fn dos_zeit(sekunden: i64) -> (u16, u16) {
    let tage = sekunden.div_euclid(86_400);
    let rest = sekunden.rem_euclid(86_400);
    // Kalenderdatum aus Tagen seit 1970 (Verfahren nach H. Hinnant).
    let z = tage + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let tag = doy - (153 * mp + 2) / 5 + 1;
    let monat = if mp < 10 { mp + 3 } else { mp - 9 };
    let jahr = yoe + era * 400 + i64::from(monat <= 2);
    if jahr < 1980 {
        return (0, (1 << 5) | 1);
    }
    let jahr = jahr.min(2107);
    let zeit = ((rest / 3600) << 11) | (((rest % 3600) / 60) << 5) | ((rest % 60) / 2);
    let datum = ((jahr - 1980) << 9) | (monat << 5) | tag;
    (zeit as u16, datum as u16)
}

/// Schreibt `eintraege` (Name im Zip, Datei auf der Platte) in dieser Reihenfolge nach `ziel`.
/// Liefert die Größe des Zips in Byte.
pub fn schreiben(ziel: &Path, eintraege: &[(String, PathBuf)], sekunden: i64) -> io::Result<u64> {
    if eintraege.len() > usize::from(u16::MAX) {
        return Err(zu_gross());
    }
    let (zeit, datum) = dos_zeit(sekunden);
    let mut aus = io::BufWriter::new(std::fs::File::create(ziel)?);
    let mut stelle: u64 = 0;
    let mut zentral = Vec::new();
    for (name, pfad) in eintraege {
        let roh = std::fs::read(pfad)?;
        let mut crc = Crc::new();
        crc.update(&roh);
        let mut packer = DeflateEncoder::new(Vec::new(), Compression::best());
        packer.write_all(&roh)?;
        let gepackt = packer.finish()?;
        let (methode, daten): (u16, &[u8]) = if gepackt.len() < roh.len() {
            (8, &gepackt)
        } else {
            (0, &roh)
        };
        let gross = u32::try_from(roh.len()).map_err(|_| zu_gross())?;
        let klein = u32::try_from(daten.len()).map_err(|_| zu_gross())?;
        let versatz = u32::try_from(stelle).map_err(|_| zu_gross())?;
        let n = name.as_bytes();
        let n_len = u16::try_from(n.len()).map_err(|_| zu_gross())?;

        let mut kopf = Vec::with_capacity(30 + n.len());
        kopf.extend(LOKAL.to_le_bytes());
        kopf.extend(VERSION.to_le_bytes());
        kopf.extend(UTF8.to_le_bytes());
        kopf.extend(methode.to_le_bytes());
        kopf.extend(zeit.to_le_bytes());
        kopf.extend(datum.to_le_bytes());
        kopf.extend(crc.sum().to_le_bytes());
        kopf.extend(klein.to_le_bytes());
        kopf.extend(gross.to_le_bytes());
        kopf.extend(n_len.to_le_bytes());
        kopf.extend(0u16.to_le_bytes());
        kopf.extend(n);
        aus.write_all(&kopf)?;
        aus.write_all(daten)?;
        stelle += (kopf.len() + daten.len()) as u64;

        zentral.extend(ZENTRAL.to_le_bytes());
        zentral.extend(ERSTELLT.to_le_bytes());
        zentral.extend(VERSION.to_le_bytes());
        zentral.extend(UTF8.to_le_bytes());
        zentral.extend(methode.to_le_bytes());
        zentral.extend(zeit.to_le_bytes());
        zentral.extend(datum.to_le_bytes());
        zentral.extend(crc.sum().to_le_bytes());
        zentral.extend(klein.to_le_bytes());
        zentral.extend(gross.to_le_bytes());
        zentral.extend(n_len.to_le_bytes());
        zentral.extend([0u8; 8]); // Extra, Kommentar, Datenträger, interne Attribute
        zentral.extend(RECHTE.to_le_bytes());
        zentral.extend(versatz.to_le_bytes());
        zentral.extend(n);
    }
    let anzahl = eintraege.len() as u16;
    let z_groesse = u32::try_from(zentral.len()).map_err(|_| zu_gross())?;
    let z_stelle = u32::try_from(stelle).map_err(|_| zu_gross())?;
    aus.write_all(&zentral)?;
    let mut ende = Vec::with_capacity(22);
    ende.extend(ENDE.to_le_bytes());
    ende.extend([0u8; 4]); // Datenträger
    ende.extend(anzahl.to_le_bytes());
    ende.extend(anzahl.to_le_bytes());
    ende.extend(z_groesse.to_le_bytes());
    ende.extend(z_stelle.to_le_bytes());
    ende.extend(0u16.to_le_bytes());
    aus.write_all(&ende)?;
    aus.flush()?;
    Ok(stelle + zentral.len() as u64 + ende.len() as u64)
}

fn u16_bei(d: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(i..i + 2)?.try_into().ok()?))
}

fn u32_bei(d: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(i..i + 4)?.try_into().ok()?))
}

/// Liest ein Zip aus `schreiben` zurück (Name, Inhalt) und prüft jede Prüfsumme.
pub fn inhalt(d: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let kaputt = || "kein lesbares Zip".to_string();
    let ende = d.len().checked_sub(22).ok_or_else(kaputt)?;
    if u32_bei(d, ende) != Some(ENDE) {
        return Err(kaputt());
    }
    let anzahl = u16_bei(d, ende + 10).ok_or_else(kaputt)?;
    let mut z = u32_bei(d, ende + 16).ok_or_else(kaputt)? as usize;
    let mut aus = Vec::new();
    for _ in 0..anzahl {
        if u32_bei(d, z) != Some(ZENTRAL) {
            return Err(kaputt());
        }
        let methode = u16_bei(d, z + 10).ok_or_else(kaputt)?;
        let crc = u32_bei(d, z + 16).ok_or_else(kaputt)?;
        let klein = u32_bei(d, z + 20).ok_or_else(kaputt)? as usize;
        let n_len = u16_bei(d, z + 28).ok_or_else(kaputt)? as usize;
        let rest = (u16_bei(d, z + 30).ok_or_else(kaputt)? as usize)
            + (u16_bei(d, z + 32).ok_or_else(kaputt)? as usize);
        let lokal = u32_bei(d, z + 42).ok_or_else(kaputt)? as usize;
        let name = String::from_utf8(d.get(z + 46..z + 46 + n_len).ok_or_else(kaputt)?.to_vec())
            .map_err(|_| kaputt())?;
        z += 46 + n_len + rest;
        if u32_bei(d, lokal) != Some(LOKAL) {
            return Err(kaputt());
        }
        let start = lokal
            + 30
            + u16_bei(d, lokal + 26).ok_or_else(kaputt)? as usize
            + u16_bei(d, lokal + 28).ok_or_else(kaputt)? as usize;
        let gepackt = d.get(start..start + klein).ok_or_else(kaputt)?;
        let daten = match methode {
            0 => gepackt.to_vec(),
            8 => {
                let mut v = Vec::new();
                DeflateDecoder::new(gepackt)
                    .read_to_end(&mut v)
                    .map_err(|e| e.to_string())?;
                v
            }
            m => return Err(format!("{name}: Methode {m} unbekannt")),
        };
        let mut c = Crc::new();
        c.update(&daten);
        if c.sum() != crc {
            return Err(format!("{name}: Prüfsumme stimmt nicht"));
        }
        aus.push((name, daten));
    }
    Ok(aus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dos_zeit_stimmt() {
        // 2026-11-17 13:45:30 UTC
        let (zeit, datum) = dos_zeit(1_794_923_130);
        assert_eq!(datum >> 9, 2026 - 1980);
        assert_eq!((datum >> 5) & 0xf, 11);
        assert_eq!(datum & 0x1f, 17);
        assert_eq!(zeit >> 11, 13);
        assert_eq!((zeit >> 5) & 0x3f, 45);
        assert_eq!((zeit & 0x1f) * 2, 30);
        assert_eq!(dos_zeit(0), (0, 0x21));
    }

    #[test]
    fn hin_und_zurueck_mit_umlauten() {
        let o = std::env::temp_dir().join(format!("tftn-zip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        let a = o.join("a.txt");
        let b = o.join("b.bin");
        std::fs::write(&a, "Grüße ".repeat(200)).unwrap();
        std::fs::write(&b, [7u8, 1, 255]).unwrap();
        let eintraege = vec![
            ("Ordner/Straße.txt".to_string(), a.clone()),
            ("Ordner/b.bin".to_string(), b.clone()),
        ];
        let ziel = o.join("x.zip");
        let groesse = schreiben(&ziel, &eintraege, 1_794_923_130).unwrap();
        let daten = std::fs::read(&ziel).unwrap();
        assert_eq!(groesse, daten.len() as u64);
        let zurueck = inhalt(&daten).unwrap();
        assert_eq!(zurueck.len(), 2);
        assert_eq!(zurueck[0].0, "Ordner/Straße.txt");
        assert_eq!(zurueck[0].1, std::fs::read(&a).unwrap());
        assert_eq!(zurueck[1].1, vec![7, 1, 255]);
        // gleiche Eingabe, gleiche Bytes
        let ziel2 = o.join("y.zip");
        schreiben(&ziel2, &eintraege, 1_794_923_130).unwrap();
        assert_eq!(daten, std::fs::read(&ziel2).unwrap());
        let _ = std::fs::remove_dir_all(&o);
    }
}
