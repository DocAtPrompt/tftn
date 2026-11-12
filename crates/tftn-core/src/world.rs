//! Die World der Vorschau (KONZEPT 4, E12).
//!
//! Sie bekommt dieselben Eingaben wie die Gegenprobe-Zeile der typst-CLI:
//! Schriften nur aus `fonts/`, Pakete nur aus `packages/`, Datum aus `tftn.toml`.
//! Aufbau und Reihenfolge folgen `typst-cli` 0.15.1 (`world.rs`, `fonts.rs`).
//! Es gibt keinen Netzzugriff: Fehlt ein Paket, wird es gemeldet, nicht geladen.

use std::collections::HashMap;
use std::sync::Mutex;
use tftn_sprachen::tr;

use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime, Dict, Duration};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::datetime::Time;
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::FsPackages;

use crate::projekt::Projekt;

pub struct ProjektWorld {
    library: LazyHash<Library>,
    schriften: FontStore,
    dateien: FileStore<ProjektDateien>,
    haupt: FileId,
    zeit: Time,
    /// Quellen aus dem Editor (ungespeichert); haben Vorrang vor der Platte.
    ueberlagert: HashMap<FileId, Source>,
}

struct ProjektDateien {
    projekt: FsRoot,
    pakete: FsPackages,
    fehlende: Mutex<Vec<PackageSpec>>,
}

impl FileLoader for ProjektDateien {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        match id.root() {
            VirtualRoot::Project => self.projekt.load(id.vpath()),
            VirtualRoot::Package(spec) => match self.pakete.obtain(spec) {
                Some(wurzel) => wurzel.load(id.vpath()),
                None => {
                    let mut fehlende = self.fehlende.lock().unwrap();
                    if !fehlende.contains(spec) {
                        fehlende.push(spec.clone());
                    }
                    Err(FileError::Package(PackageError::NotFound(spec.clone())))
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum WorldFehler {
    Hauptdatei(String),
}

impl std::fmt::Display for WorldFehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hauptdatei(e) => f.write_str(&tr!("Hauptdatei: {fehler}", fehler = e)),
        }
    }
}

impl std::error::Error for WorldFehler {}

impl ProjektWorld {
    pub fn neu(projekt: &Projekt) -> Result<Self, WorldFehler> {
        let haupt_pfad = projekt
            .haupt()
            .canonicalize()
            .map_err(|e| WorldFehler::Hauptdatei(format!("{}: {e}", projekt.haupt().display())))?;
        let vpath = VirtualPath::virtualize(&projekt.wurzel, &haupt_pfad)
            .map_err(|e| WorldFehler::Hauptdatei(format!("{e:?}")))?;
        let haupt = RootedPath::new(VirtualRoot::Project, vpath).intern();

        // Wie `typst compile --ignore-system-fonts --ignore-embedded-fonts --font-path fonts`;
        // ohne eingebundene Schriften der Vorrat der App.
        let mut schriften = FontStore::new();
        for ordner in projekt.schriftordner() {
            schriften.extend(fonts::scan(&ordner));
        }

        let library = Library::builder().with_inputs(Dict::new()).build();

        Ok(Self {
            library: LazyHash::new(library),
            schriften,
            dateien: FileStore::new(ProjektDateien {
                projekt: FsRoot::new(projekt.wurzel.clone()),
                pakete: FsPackages::new(projekt.pakete()),
                fehlende: Mutex::new(Vec::new()),
            }),
            haupt,
            ueberlagert: HashMap::new(),
            zeit: Time::fixed_timestamp(projekt.epoche()).expect("Zeitstempel im gültigen Bereich"),
        })
    }

    /// Kennung der Hauptdatei.
    pub fn haupt(&self) -> FileId {
        self.haupt
    }

    /// Legt eine Quelle aus dem Editor über die Datei auf der Platte.
    pub fn quelle_setzen(&mut self, quelle: Source) {
        self.ueberlagert.insert(quelle.id(), quelle);
    }

    /// Pakete, die beim letzten Kompilieren im Projekt fehlten.
    pub fn fehlende_pakete(&self) -> Vec<PackageSpec> {
        self.dateien.loader().fehlende.lock().unwrap().clone()
    }

    /// Bereitet einen neuen Lauf vor: geänderte Dateien werden neu gelesen.
    pub fn zuruecksetzen(&mut self) {
        self.dateien.reset();
        self.dateien.loader().fehlende.lock().unwrap().clear();
    }
}

impl World for ProjektWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.schriften.book()
    }

    fn main(&self) -> FileId {
        self.haupt
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        match self.ueberlagert.get(&id) {
            Some(quelle) => Ok(quelle.clone()),
            None => self.dateien.source(id),
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        match self.ueberlagert.get(&id) {
            Some(quelle) => Ok(Bytes::from_string(quelle.text().to_string())),
            None => self.dateien.file(id),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.schriften.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.zeit.today(offset)
    }
}

impl typst_ide::IdeWorld for ProjektWorld {
    fn upcast(&self) -> &dyn World {
        self
    }
}
