//! Abnahme A4 (PLAN-M1): Eine Farbänderung über das Widget ist im git diff genau eine Zeile,
//! und die Vorschau ändert sich nur auf der Seite des Hinweiskastens.

use std::path::Path;
use std::process::Command;

use tftn_core::Projekt;
use tftn_core::vorschau::Vorschau;
use tftn_edit::{WidgetParameter, angebote, anwenden, farbe_patch};
use typst_syntax::Source;

fn git(ordner: &Path, args: &[&str]) -> String {
    let aus = Command::new("git")
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
            "-c",
            "core.autocrlf=false",
        ])
        .args(args)
        .current_dir(ordner)
        .output()
        .expect("git");
    assert!(
        aus.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&aus.stderr)
    );
    String::from_utf8_lossy(&aus.stdout).into_owned()
}

fn kopieren(von: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap() {
        let p = e.unwrap().path();
        let ziel = nach.join(p.file_name().unwrap());
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with('_') {
            continue; // _renders, _lokal
        }
        if p.is_dir() {
            kopieren(&p, &ziel)
        } else {
            std::fs::copy(&p, &ziel).unwrap();
        }
    }
}

#[test]
fn farbaenderung_ist_eine_zeile_im_git_diff() {
    let korpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/korpus/testartikel");
    let ordner = Path::new(env!("CARGO_TARGET_TMPDIR")).join("abnahme-a4");
    let _ = std::fs::remove_dir_all(&ordner);
    kopieren(&korpus, &ordner);
    git(&ordner, &["init", "-q"]);
    git(&ordner, &["add", "-A"]);
    git(&ordner, &["commit", "-q", "-m", "vorher"]);

    let projekt = Projekt::oeffnen(&ordner).unwrap();
    let mut vorschau = Vorschau::neu(&projekt).unwrap();
    let text = std::fs::read_to_string(projekt.haupt()).unwrap();
    let id = vorschau.world().haupt();
    let vorher = vorschau.lauf(1, Some(Source::new(id, text.clone())));
    assert!(vorher.gelungen);

    let quelle = Source::new(id, text.clone());
    let parameter = [WidgetParameter {
        position: None,
        funktion: "hinweis".into(),
        parameter: "farbe".into(),
        art: tftn_edit::WidgetArt::Farbe {
            raum: tftn_edit::Farbraum::Rgb,
        },
    }];
    let a = angebote(
        &quelle,
        text.find("Alle Befunde sind erfunden").unwrap(),
        &parameter,
    );
    assert_eq!(
        a[0].wert,
        Some(tftn_edit::Wert::Farbe(tftn_edit::Farbe::Rgb([
            0xe8, 0xf1, 0xf2
        ])))
    );
    let patch = farbe_patch(&quelle, &a[0], "fde8c8").unwrap();
    let neu = anwenden(&text, &patch);
    std::fs::write(projekt.haupt(), &neu).unwrap();

    let stat = git(&ordner, &["diff", "--stat"]);
    println!("{stat}");
    assert!(
        stat.contains("1 file changed, 1 insertion(+), 1 deletion(-)"),
        "{stat}"
    );

    let nachher = vorschau.lauf(2, Some(Source::new(id, neu)));
    assert!(nachher.gelungen);
    assert_eq!(
        nachher.geaendert.len(),
        1,
        "nur die Seite mit dem Kasten: {:?}",
        nachher.geaendert
    );
}
