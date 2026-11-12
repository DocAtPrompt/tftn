//! Formular-Schema lesen und gegen das Profil „TFTN-Schema 1“ prüfen (KONZEPT 6).
//!
//! Das Profil ist eine kleine Teilmenge von JSON Schema 2020-12. Was außerhalb liegt, ist ein Fehler
//! mit Ort (JSON-Pointer), damit jedes gültige Schema ein brauchbares Formular ergibt. Gesammelt
//! werden alle Fehler, nicht nur der erste.

use serde::Serialize;
use serde_json::{Map, Value};
use tftn_sprachen::tr;

/// Das Formular eines Dokumenttyps: die Wurzel des Schemas.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Formular {
    pub titel: String,
    pub felder: Vec<Feld>,
}

/// Ein Feld des Formulars (eine Eigenschaft im Schema).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Feld {
    /// Schlüssel in `dokument.toml`
    pub name: String,
    /// `title`, sonst der Name
    pub titel: String,
    /// `description`
    pub hilfe: Option<String>,
    pub pflicht: bool,
    /// `x-tftn-gruppe`
    pub gruppe: Option<String>,
    /// `x-tftn-sensibel`
    pub sensibel: bool,
    /// `x-tftn-pandoc`
    pub pandoc: Option<String>,
    pub art: Art,
}

/// Wie ein Feld bearbeitet wird.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "art",
    rename_all = "kebab-case",
    rename_all_fields = "kebab-case"
)]
pub enum Art {
    Text {
        widget: TextWidget,
        format: Option<String>,
        min_laenge: Option<u64>,
        max_laenge: Option<u64>,
        muster: Option<String>,
        standard: Option<String>,
    },
    Zahl {
        ganzzahl: bool,
        min: Option<f64>,
        max: Option<f64>,
        standard: Option<f64>,
    },
    Schalter {
        standard: Option<bool>,
    },
    /// `string` mit `format: "date"`; Werte als `JJJJ-MM-TT`
    Datum {
        standard: Option<String>,
    },
    /// `enum` oder `oneOf` aus `{const, title}`
    Wahl {
        werte: Vec<Value>,
        beschriftungen: Vec<String>,
        standard: Option<Value>,
    },
    /// Array von Skalaren
    Liste {
        element: Box<Art>,
        min: Option<u64>,
        max: Option<u64>,
    },
    /// `object`
    Gruppe {
        felder: Vec<Feld>,
    },
    /// Array von `object`
    Eintraege {
        felder: Vec<Feld>,
        min: Option<u64>,
        max: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextWidget {
    Einzeilig,
    Mehrzeilig,
    Markup,
}

/// Ein Verstoß gegen das Profil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfilFehler {
    /// JSON-Pointer, z. B. `/properties/autoren/items`; die Wurzel ist `""`
    pub ort: String,
    pub text: String,
}

/// Erlaubte Schlüsselwörter (KONZEPT 6).
const ERLAUBT: [&str; 24] = [
    "$schema",
    "title",
    "description",
    "type",
    "properties",
    "required",
    "items",
    "default",
    "enum",
    "oneOf",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "pattern",
    "minItems",
    "maxItems",
    "format",
    "x-tftn-widget",
    "x-tftn-reihenfolge",
    "x-tftn-gruppe",
    "x-tftn-pandoc",
    "x-tftn-sensibel",
    "x-tftn-rechnen",
];

/// Erlaubte Werte von `format`; `literaturdatei` und `zitierstil` seit PLAN-M2 AP7.
pub const FORMATE: [&str; 9] = [
    "date",
    "email",
    "uri",
    "orcid",
    "iban",
    "uid-at",
    "sv-nummer",
    "literaturdatei",
    "zitierstil",
];

/// Liest ein Formular-Schema und prüft es gegen das Profil.
pub fn formular_lesen(json: &str) -> Result<Formular, Vec<ProfilFehler>> {
    let wert: Value = serde_json::from_str(json).map_err(|e| {
        vec![ProfilFehler {
            ort: String::new(),
            text: tr!("kein gültiges JSON: {fehler}", fehler = e),
        }]
    })?;
    let mut fehler = Vec::new();
    let mut l = Leser {
        fehler: &mut fehler,
    };
    let formular = match wert.as_object() {
        Some(wurzel) => {
            l.schluessel_pruefen(wurzel, "");
            if wurzel.get("type").and_then(Value::as_str) != Some("object") {
                l.fehler("", tr!("die Wurzel muss type \"object\" haben"));
            }
            let titel = wurzel
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Formular")
                .to_string();
            let felder = l.eigenschaften(wurzel, "");
            Some(Formular { titel, felder })
        }
        None => {
            l.fehler("", tr!("die Wurzel muss ein JSON-Objekt sein"));
            None
        }
    };
    match formular {
        Some(f) if fehler.is_empty() => Ok(f),
        _ => Err(fehler),
    }
}

/// Hat das Formular irgendwo ein Feld mit `x-tftn-sensibel`?
pub fn sensibel(formular: &Formular) -> bool {
    fn feld(f: &Feld) -> bool {
        f.sensibel
            || match &f.art {
                Art::Gruppe { felder } | Art::Eintraege { felder, .. } => felder.iter().any(feld),
                _ => false,
            }
    }
    formular.felder.iter().any(feld)
}

struct Leser<'a> {
    fehler: &'a mut Vec<ProfilFehler>,
}

impl Leser<'_> {
    fn fehler(&mut self, ort: &str, text: impl Into<String>) {
        self.fehler.push(ProfilFehler {
            ort: ort.to_string(),
            text: text.into(),
        });
    }

    /// Meldet unerlaubte Schlüsselwörter; `true`, wenn alle erlaubt sind.
    fn schluessel_pruefen(&mut self, obj: &Map<String, Value>, ort: &str) -> bool {
        let mut gut = true;
        for k in obj.keys() {
            if !ERLAUBT.contains(&k.as_str()) {
                self.fehler(
                    ort,
                    tr!(
                        "Schlüsselwort {k} ist im Profil TFTN-Schema 1 nicht erlaubt",
                        k
                    ),
                );
                gut = false;
            }
        }
        gut
    }

    /// Die Felder eines `object`-Knotens in Anzeigereihenfolge.
    fn eigenschaften(&mut self, obj: &Map<String, Value>, ort: &str) -> Vec<Feld> {
        let pflicht: Vec<&str> = match obj.get("required") {
            None => Vec::new(),
            Some(Value::Array(a)) if a.iter().all(Value::is_string) => {
                a.iter().filter_map(Value::as_str).collect()
            }
            Some(_) => {
                self.fehler(ort, tr!("required muss eine Liste von Namen sein"));
                Vec::new()
            }
        };
        let props = match obj.get("properties") {
            None => {
                self.fehler(ort, tr!("ein object braucht properties"));
                return Vec::new();
            }
            Some(Value::Object(p)) => p,
            Some(_) => {
                self.fehler(ort, tr!("properties muss ein Objekt sein"));
                return Vec::new();
            }
        };
        for p in &pflicht {
            if !props.contains_key(*p) {
                self.fehler(ort, tr!("required nennt {p}, das es nicht gibt", p));
            }
        }
        let mut reihenfolge: Vec<&str> = Vec::new();
        match obj.get("x-tftn-reihenfolge") {
            None => {}
            Some(Value::Array(a)) => {
                for n in a {
                    match n.as_str() {
                        Some(n) if props.contains_key(n) => {
                            if !reihenfolge.contains(&n) {
                                reihenfolge.push(n)
                            }
                        }
                        _ => self.fehler(
                            ort,
                            tr!("x-tftn-reihenfolge nennt {n}, das es nicht gibt", n),
                        ),
                    }
                }
            }
            Some(_) => self.fehler(
                ort,
                tr!("x-tftn-reihenfolge muss eine Liste von Namen sein"),
            ),
        }
        for n in props.keys() {
            if !reihenfolge.contains(&n.as_str()) {
                reihenfolge.push(n);
            }
        }
        reihenfolge
            .into_iter()
            .filter_map(|name| {
                let unterort = format!("{ort}/properties/{}", zeiger(name));
                self.feld(&props[name], &unterort, name, pflicht.contains(&name))
            })
            .collect()
    }

    fn feld(&mut self, wert: &Value, ort: &str, name: &str, pflicht: bool) -> Option<Feld> {
        let Some(obj) = wert.as_object() else {
            self.fehler(ort, tr!("ein Feld muss ein JSON-Objekt sein"));
            return None;
        };
        let art = self.art(obj, ort)?;
        let text = |k: &str| obj.get(k).and_then(Value::as_str).map(str::to_string);
        for k in ["title", "description", "x-tftn-gruppe", "x-tftn-pandoc"] {
            if obj.get(k).is_some_and(|v| !v.is_string()) {
                self.fehler(ort, tr!("{k} muss Text sein", k));
            }
        }
        if obj.get("x-tftn-sensibel").is_some_and(|v| !v.is_boolean()) {
            self.fehler(ort, tr!("x-tftn-sensibel muss true oder false sein"));
        }
        Some(Feld {
            name: name.to_string(),
            titel: text("title").unwrap_or_else(|| name.to_string()),
            hilfe: text("description"),
            pflicht,
            gruppe: text("x-tftn-gruppe"),
            sensibel: obj
                .get("x-tftn-sensibel")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            pandoc: text("x-tftn-pandoc"),
            art,
        })
    }

    /// Die Art eines Knotens; `None` nach einem Fehler, der kein Feld zulässt.
    fn art(&mut self, obj: &Map<String, Value>, ort: &str) -> Option<Art> {
        let bekannt = self.schluessel_pruefen(obj, ort);
        let typ = obj.get("type").and_then(Value::as_str);
        if obj.get("type").is_some() && typ.is_none() {
            self.fehler(ort, tr!("type muss ein einzelner Name sein"));
            return None;
        }
        // Erlaubte Zusatzschlüssel je Typ
        let nur =
            |k: &str, typen: &[&str]| obj.contains_key(k) && !typen.contains(&typ.unwrap_or(""));
        for (k, typen) in [
            ("properties", &["object"][..]),
            ("required", &["object"]),
            ("x-tftn-reihenfolge", &["object"]),
            ("items", &["array"]),
            ("minItems", &["array"]),
            ("maxItems", &["array"]),
            ("minLength", &["string"]),
            ("maxLength", &["string"]),
            ("pattern", &["string"]),
            ("format", &["string"]),
            ("minimum", &["integer", "number"]),
            ("maximum", &["integer", "number"]),
        ] {
            if nur(k, typen) {
                self.fehler(
                    ort,
                    tr!(
                        "{k} passt nicht zu type {typ}",
                        k,
                        typ = typ.unwrap_or("(fehlt)")
                    ),
                );
            }
        }
        let widget = match obj.get("x-tftn-widget") {
            None => None,
            Some(Value::String(w)) => Some(w.as_str()),
            Some(_) => {
                self.fehler(ort, tr!("x-tftn-widget muss Text sein"));
                None
            }
        };
        let widget_passt = match widget {
            None => true,
            Some("einzeilig" | "mehrzeilig" | "markup") => typ == Some("string"),
            Some("liste") => typ == Some("array"),
            Some(w) => {
                self.fehler(ort, tr!("x-tftn-widget {w} gibt es nicht", w));
                true
            }
        };
        if !widget_passt {
            self.fehler(
                ort,
                tr!(
                    "x-tftn-widget {widget} passt nicht zu type {typ}",
                    widget = widget.unwrap_or(""),
                    typ = typ.unwrap_or("(fehlt)")
                ),
            );
        }

        if obj.contains_key("enum") || obj.contains_key("oneOf") {
            return self.wahl(obj, ort, typ);
        }
        let zahl = |k: &str| obj.get(k).and_then(Value::as_f64);
        let ganz = |l: &mut Self, k: &str| -> Option<u64> {
            match obj.get(k) {
                None => None,
                Some(v) => v.as_u64().or_else(|| {
                    l.fehler(ort, tr!("{k} muss eine ganze Zahl ≥ 0 sein", k));
                    None
                }),
            }
        };
        match typ {
            None => {
                // Neben einem unerlaubten Schlüsselwort (etwa $ref) wäre das nur ein Folgefehler.
                if bekannt {
                    self.fehler(ort, tr!("type fehlt"));
                }
                None
            }
            Some("string") => {
                let format = match obj.get("format") {
                    None => None,
                    Some(Value::String(f)) if FORMATE.contains(&f.as_str()) => Some(f.clone()),
                    Some(f) => {
                        let f = f
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| f.to_string());
                        self.fehler(ort, tr!("format {f} ist im Profil nicht erlaubt", f));
                        None
                    }
                };
                let standard = obj
                    .get("default")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if obj.get("default").is_some_and(|d| !d.is_string()) {
                    self.fehler(ort, tr!("default passt nicht zu type string"));
                }
                if format.as_deref() == Some("date") {
                    return Some(Art::Datum { standard });
                }
                let muster = obj
                    .get("pattern")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let Some(m) = &muster
                    && regex::Regex::new(m).is_err()
                {
                    self.fehler(ort, tr!("pattern ist kein gültiger regulärer Ausdruck"));
                }
                Some(Art::Text {
                    widget: match widget {
                        Some("mehrzeilig") => TextWidget::Mehrzeilig,
                        Some("markup") => TextWidget::Markup,
                        _ => TextWidget::Einzeilig,
                    },
                    format,
                    min_laenge: ganz(self, "minLength"),
                    max_laenge: ganz(self, "maxLength"),
                    muster,
                    standard,
                })
            }
            Some(t @ ("integer" | "number")) => {
                if obj.get("default").is_some_and(|d| !d.is_number()) {
                    self.fehler(ort, tr!("default passt nicht zu type {t}", t));
                }
                Some(Art::Zahl {
                    ganzzahl: t == "integer",
                    min: zahl("minimum"),
                    max: zahl("maximum"),
                    standard: zahl("default"),
                })
            }
            Some("boolean") => {
                if obj.get("default").is_some_and(|d| !d.is_boolean()) {
                    self.fehler(ort, tr!("default passt nicht zu type boolean"));
                }
                Some(Art::Schalter {
                    standard: obj.get("default").and_then(Value::as_bool),
                })
            }
            Some("object") => {
                if obj.contains_key("default") {
                    self.fehler(ort, tr!("default ist bei object nicht erlaubt"));
                }
                Some(Art::Gruppe {
                    felder: self.eigenschaften(obj, ort),
                })
            }
            Some("array") => {
                if obj.contains_key("default") {
                    self.fehler(ort, tr!("default ist bei array nicht erlaubt"));
                }
                let unterort = format!("{ort}/items");
                let Some(items) = obj.get("items") else {
                    self.fehler(ort, tr!("ein array braucht items"));
                    return None;
                };
                let Some(items) = items.as_object() else {
                    self.fehler(&unterort, tr!("items muss ein JSON-Objekt sein"));
                    return None;
                };
                let min = ganz(self, "minItems");
                let max = ganz(self, "maxItems");
                match items.get("type").and_then(Value::as_str) {
                    Some("array") => {
                        self.fehler(
                            &unterort,
                            tr!("Listen von Listen sind im Profil nicht erlaubt"),
                        );
                        None
                    }
                    Some("object") => {
                        self.schluessel_pruefen(items, &unterort);
                        Some(Art::Eintraege {
                            felder: self.eigenschaften(items, &unterort),
                            min,
                            max,
                        })
                    }
                    _ => {
                        let element = self.art(items, &unterort)?;
                        Some(Art::Liste {
                            element: Box::new(element),
                            min,
                            max,
                        })
                    }
                }
            }
            Some(t) => {
                self.fehler(ort, tr!("type {t} ist im Profil nicht erlaubt", t));
                None
            }
        }
    }

    fn wahl(&mut self, obj: &Map<String, Value>, ort: &str, typ: Option<&str>) -> Option<Art> {
        if obj.contains_key("enum") && obj.contains_key("oneOf") {
            self.fehler(ort, tr!("enum und oneOf zusammen sind nicht erlaubt"));
            return None;
        }
        let mut werte = Vec::new();
        let mut beschriftungen = Vec::new();
        if let Some(e) = obj.get("enum") {
            match e.as_array() {
                Some(a) if !a.is_empty() => werte = a.clone(),
                _ => {
                    self.fehler(ort, tr!("enum muss eine nicht leere Liste sein"));
                    return None;
                }
            }
        } else {
            let Some(a) = obj
                .get("oneOf")
                .and_then(Value::as_array)
                .filter(|a| !a.is_empty())
            else {
                self.fehler(ort, tr!("oneOf muss eine nicht leere Liste sein"));
                return None;
            };
            let mut gut = true;
            for (i, eintrag) in a.iter().enumerate() {
                let unterort = format!("{ort}/oneOf/{i}");
                let passt = eintrag.as_object().filter(|o| {
                    o.contains_key("const")
                        && o.get("title").is_some_and(Value::is_string)
                        && o.keys().all(|k| k == "const" || k == "title")
                });
                match passt {
                    Some(o) => {
                        werte.push(o["const"].clone());
                        beschriftungen.push(o["title"].as_str().unwrap_or_default().to_string());
                    }
                    None => {
                        self.fehler(
                            &unterort,
                            tr!("oneOf erlaubt nur Einträge aus const und title"),
                        );
                        gut = false;
                    }
                }
            }
            if !gut {
                return None;
            }
        }
        let typ_passt = |v: &Value| match typ {
            None => v.is_string() || v.is_number() || v.is_boolean(),
            Some("string") => v.is_string(),
            Some("integer") => v.is_i64() || v.is_u64(),
            Some("number") => v.is_number(),
            Some("boolean") => v.is_boolean(),
            Some(_) => false,
        };
        if !werte.iter().all(typ_passt) {
            self.fehler(ort, tr!("die Werte der Auswahl passen nicht zu type"));
            return None;
        }
        let standard = obj.get("default").cloned();
        if standard.as_ref().is_some_and(|s| !werte.contains(s)) {
            self.fehler(ort, tr!("default ist keiner der Werte der Auswahl"));
        }
        Some(Art::Wahl {
            werte,
            beschriftungen,
            standard,
        })
    }
}

/// Escape für JSON-Pointer (RFC 6901).
fn zeiger(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}
