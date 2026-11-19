//! „Über TFTN“: Version, Lizenzen und eine Handvoll fester Links (Projektseite, Handbuch, Fehler
//! melden, Ko-fi). Ein Link öffnet nur auf Klick den Standardbrowser; TFTN selbst geht dabei nicht
//! ins Netz. Die Oberfläche nennt nur den Namen des Links, nie eine Adresse – geöffnet wird
//! ausschließlich, was hier fest steht. Jeder Klick steht im Netzprotokoll; „Ganz offline“ sperrt
//! die Links.

use serde::Serialize;
use tftn_sprachen::tr;

/// Projektseite auf GitHub.
pub const PROJEKT: &str = "https://github.com/DocAtPrompt/tftn";

/// Name des Ko-fi-Kontos (Adresse `https://ko-fi.com/<name>`). Leer hieße: kein Konto, dann
/// fehlen Link und Einstellung. Derselbe Name steht in `.github/FUNDING.yml`.
pub const KOFI_NAME: &str = "docatprompt";

/// Die Rechner, die ein Link überhaupt ansprechen kann (für das Datenschutzblatt).
pub const GEGENSTELLEN: &[&str] = &["github.com", "ko-fi.com"];

/// Die Links in „Über TFTN“.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Projekt,
    Handbuch,
    Fehler,
    Kofi,
}

impl Link {
    pub const ALLE: [Link; 4] = [Link::Projekt, Link::Handbuch, Link::Fehler, Link::Kofi];

    pub fn name(self) -> &'static str {
        match self {
            Link::Projekt => "projekt",
            Link::Handbuch => "handbuch",
            Link::Fehler => "fehler",
            Link::Kofi => "kofi",
        }
    }

    pub fn aus_name(name: &str) -> Option<Link> {
        Link::ALLE.into_iter().find(|l| l.name() == name)
    }

    /// Die feste Adresse; `englisch`: das Handbuch auf Englisch. `None` für Ko-fi ohne Konto.
    pub fn adresse(self, englisch: bool, kofi: &str) -> Option<String> {
        match self {
            Link::Projekt => Some(PROJEKT.to_string()),
            Link::Handbuch => Some(format!(
                "{PROJEKT}/blob/main/docs/{}",
                if englisch { "manual.md" } else { "handbuch.md" }
            )),
            Link::Fehler => Some(format!("{PROJEKT}/issues")),
            Link::Kofi => kofi_adresse(kofi),
        }
    }

    pub fn titel(self) -> String {
        match self {
            Link::Projekt => tr!("Projektseite und Quelltext"),
            Link::Handbuch => tr!("Handbuch"),
            Link::Fehler => tr!("Fehler melden oder Wunsch äußern"),
            Link::Kofi => tr!("TFTN unterstützen (Ko-fi)"),
        }
    }
}

/// `https://ko-fi.com/<name>`, wenn der Name nur aus Buchstaben, Ziffern und `_` besteht.
pub fn kofi_adresse(name: &str) -> Option<String> {
    let gut = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    gut.then(|| format!("https://ko-fi.com/{name}"))
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct LinkJs {
    pub name: String,
    pub titel: String,
    /// Nur zum Anzeigen (und Abschreiben, wenn „Ganz offline“ an ist).
    pub adresse: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct UeberJs {
    pub version: String,
    pub typst_version: String,
    pub links: Vec<LinkJs>,
    /// Es gibt ein Ko-fi-Konto (dann zeigen die Einstellungen den Schalter).
    pub kofi_vorhanden: bool,
    pub kofi_ausblenden: bool,
    pub ganz_offline: bool,
}

/// Der Stand für das Fenster; ein ausgeblendeter oder fehlender Ko-fi-Link fehlt in `links`.
pub fn stand(englisch: bool, kofi: &str, rechner: &tftn_einstellungen::Rechner) -> UeberJs {
    let links = Link::ALLE
        .into_iter()
        .filter(|l| *l != Link::Kofi || !rechner.kofi_ausblenden)
        .filter_map(|l| {
            Some(LinkJs {
                name: l.name().to_string(),
                titel: l.titel(),
                adresse: l.adresse(englisch, kofi)?,
            })
        })
        .collect();
    UeberJs {
        version: env!("CARGO_PKG_VERSION").to_string(),
        typst_version: tftn_manifest::anlegen::TYPST_VERSION.to_string(),
        links,
        kofi_vorhanden: kofi_adresse(kofi).is_some(),
        kofi_ausblenden: rechner.kofi_ausblenden,
        ganz_offline: rechner.ganz_offline,
    }
}

/// Welche Adresse ein Klick auf `name` öffnen darf; Fehlertext, wenn keine.
pub fn ziel(
    name: &str,
    englisch: bool,
    kofi: &str,
    rechner: &tftn_einstellungen::Rechner,
) -> Result<String, String> {
    let link = Link::aus_name(name).ok_or_else(|| tr!("Unbekannter Link."))?;
    let adresse = link
        .adresse(englisch, kofi)
        .filter(|_| link != Link::Kofi || !rechner.kofi_ausblenden)
        .ok_or_else(|| tr!("Unbekannter Link."))?;
    if rechner.ganz_offline {
        return Err(tr!(
            "„Ganz offline“ ist eingeschaltet: TFTN öffnet keine Webseiten. Die Adresse lautet {adresse}.",
            adresse
        ));
    }
    Ok(adresse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tftn_einstellungen::Rechner;

    #[test]
    fn feste_adressen_und_nur_bekannte_namen() {
        let r = Rechner::default();
        assert_eq!(
            ziel("handbuch", false, "", &r).unwrap(),
            "https://github.com/DocAtPrompt/tftn/blob/main/docs/handbuch.md"
        );
        assert!(
            ziel("handbuch", true, "", &r)
                .unwrap()
                .ends_with("/manual.md")
        );
        assert_eq!(
            ziel("fehler", false, "", &r).unwrap(),
            "https://github.com/DocAtPrompt/tftn/issues"
        );
        for falsch in [
            "",
            "https://example.org",
            "projekt ",
            "PROJEKT",
            "../projekt",
        ] {
            assert!(ziel(falsch, false, "", &r).is_err(), "{falsch}");
        }
        for l in Link::ALLE {
            assert_eq!(Link::aus_name(l.name()), Some(l));
            if let Some(a) = l.adresse(false, "beispiel") {
                let host = crate::netzprotokoll::host(&a);
                assert!(GEGENSTELLEN.contains(&host.as_str()), "{host}");
            }
        }
    }

    #[test]
    fn kofi_nur_mit_konto_und_nicht_ausgeblendet() {
        let r = Rechner::default();
        // Platzhalter: kein Konto, kein Link
        let s = stand(false, "", &r);
        assert!(!s.kofi_vorhanden);
        assert!(s.links.iter().all(|l| l.name != "kofi"));
        assert!(ziel("kofi", false, "", &r).is_err());
        // mit Konto
        let s = stand(false, "tftn_test", &r);
        assert!(s.kofi_vorhanden);
        let k = s.links.iter().find(|l| l.name == "kofi").unwrap();
        assert_eq!(k.adresse, "https://ko-fi.com/tftn_test");
        assert_eq!(ziel("kofi", false, "tftn_test", &r).unwrap(), k.adresse);
        // ausgeblendet: weder gezeigt noch zu öffnen
        let aus = Rechner {
            kofi_ausblenden: true,
            ..Rechner::default()
        };
        let s = stand(false, "tftn_test", &aus);
        assert!(s.kofi_vorhanden && s.kofi_ausblenden);
        assert!(s.links.iter().all(|l| l.name != "kofi"));
        assert!(ziel("kofi", false, "tftn_test", &aus).is_err());
        // Name mit fremden Zeichen ergibt keine Adresse
        for falsch in ["a/b", "x?y", "ä", "a b", "a.b"] {
            assert_eq!(kofi_adresse(falsch), None, "{falsch}");
        }
    }

    #[test]
    fn ganz_offline_sperrt_und_nennt_die_adresse() {
        let r = Rechner {
            ganz_offline: true,
            ..Rechner::default()
        };
        let f = ziel("projekt", false, "", &r).unwrap_err();
        assert!(f.contains(PROJEKT), "{f}");
        assert!(stand(false, "", &r).ganz_offline);
    }

    #[test]
    fn version_und_typst() {
        let s = stand(false, "", &Rechner::default());
        assert_eq!(s.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(s.typst_version, tftn_manifest::anlegen::TYPST_VERSION);
        assert_eq!(
            s.links.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            ["projekt", "handbuch", "fehler"]
        );
    }
}
