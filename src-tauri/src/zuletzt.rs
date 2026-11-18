//! Zuletzt geöffnete Projekte: eine Zeile je Projektordner in `zuletzt.txt` im Konfigurationsordner
//! der App (je Rechner, nicht im Projekt), das neueste oben. Die erste Zeile ist das Projekt, mit
//! dem TFTN startet; früher stand nur sie in der Datei, das alte Format gilt also weiter.
//!
//! Gemerkt werden mehr Ordner, als das Menü „Öffnen“ zeigt: Ein Ordner, der gerade fehlt (etwa ein
//! Tresor, der nicht eingebunden ist), wird nur übersprungen, nicht vergessen.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// So viele Ordner merkt sich die Datei.
pub const GEMERKT_MAX: usize = 20;
/// So viele zeigt das Menü „Öffnen“.
pub const ANGEZEIGT_MAX: usize = 10;

/// Die gemerkten Ordner, das neueste zuerst; leere Zeilen und doppelte fallen weg.
pub fn aus_text(text: &str) -> Vec<PathBuf> {
    let mut liste: Vec<PathBuf> = Vec::new();
    for zeile in text.lines().map(str::trim).filter(|z| !z.is_empty()) {
        let p = PathBuf::from(zeile);
        if !liste.contains(&p) {
            liste.push(p);
        }
    }
    liste
}

pub fn als_text(liste: &[PathBuf]) -> String {
    liste
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `neu` nach vorn; ein früherer Eintrag desselben Ordners fällt weg, höchstens `GEMERKT_MAX`.
pub fn vorn(liste: &[PathBuf], neu: &Path) -> Vec<PathBuf> {
    let mut aus = vec![neu.to_path_buf()];
    aus.extend(liste.iter().filter(|p| p.as_path() != neu).cloned());
    aus.truncate(GEMERKT_MAX);
    aus
}

#[derive(Serialize, Debug, PartialEq)]
pub struct ZuletztJs {
    pub pfad: String,
    /// Name des Projektordners.
    pub name: String,
    /// Name des Ordners darüber (zum Unterscheiden gleichnamiger Projekte).
    pub ort: String,
}

/// Was das Menü zeigt: vorhandene Projekte ohne das offene und ohne den Entwurf, höchstens
/// `ANGEZEIGT_MAX`. `ist_projekt` prüft, ob ein Ordner (noch) ein Projekt ist.
pub fn fuer_menue(
    liste: &[PathBuf],
    offen: Option<&Path>,
    entwurf: Option<&Path>,
    ist_projekt: impl Fn(&Path) -> bool,
) -> Vec<ZuletztJs> {
    liste
        .iter()
        .filter(|p| Some(p.as_path()) != offen && Some(p.as_path()) != entwurf)
        .filter(|p| ist_projekt(p))
        .take(ANGEZEIGT_MAX)
        .map(|p| ZuletztJs {
            pfad: p.to_string_lossy().into_owned(),
            name: name(p),
            ort: p.parent().map(name).unwrap_or_default(),
        })
        .collect()
}

fn name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn altes_format_mit_einer_zeile_gilt_weiter() {
        assert_eq!(aus_text("/a/Brief\n"), vec![p("/a/Brief")]);
        assert_eq!(aus_text(""), Vec::<PathBuf>::new());
        assert_eq!(aus_text("/a\n\n  /b  \n/a\n"), vec![p("/a"), p("/b")]);
    }

    #[test]
    fn neues_nach_vorn_ohne_doppelte_und_begrenzt() {
        let liste = vec![p("/a"), p("/b"), p("/c")];
        assert_eq!(vorn(&liste, &p("/b")), vec![p("/b"), p("/a"), p("/c")]);
        assert_eq!(vorn(&liste, &p("/d"))[0], p("/d"));
        let viele: Vec<PathBuf> = (0..30).map(|i| p(&format!("/p{i}"))).collect();
        let neu = vorn(&viele, &p("/neu"));
        assert_eq!(neu.len(), GEMERKT_MAX);
        assert_eq!(neu[0], p("/neu"));
        assert_eq!(aus_text(&als_text(&neu)), neu);
    }

    #[test]
    fn menue_ohne_offenes_entwurf_und_fehlende() {
        let liste = vec![
            p("/doku/Brief"),
            p("/entwurf"),
            p("/weg/Alt"),
            p("/tresor/Brief"),
            p("/doku/Bericht"),
        ];
        let m = fuer_menue(
            &liste,
            Some(Path::new("/doku/Bericht")),
            Some(Path::new("/entwurf")),
            |q| !q.starts_with("/weg"),
        );
        assert_eq!(
            m,
            vec![
                ZuletztJs {
                    pfad: "/doku/Brief".into(),
                    name: "Brief".into(),
                    ort: "doku".into()
                },
                ZuletztJs {
                    pfad: "/tresor/Brief".into(),
                    name: "Brief".into(),
                    ort: "tresor".into()
                },
            ]
        );
    }

    #[test]
    fn menue_zeigt_hoechstens_zehn() {
        let viele: Vec<PathBuf> = (0..15).map(|i| p(&format!("/p{i}"))).collect();
        assert_eq!(
            fuer_menue(&viele, None, None, |_| true).len(),
            ANGEZEIGT_MAX
        );
    }
}
