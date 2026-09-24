use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Int,
    Float,
    Bool,
    String,
    Choice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Input {
    #[default]
    Field,
    Slider,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RawChoice {
    Full { value: Value, #[serde(default)] label: String },
    Plain(Value),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOption {
    key: String,
    #[serde(default)]
    label: String,
    #[serde(rename = "type")]
    kind: Kind,
    #[serde(default)]
    default: Option<Value>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    input: Input,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    step: Option<f64>,
    #[serde(default)]
    unit: String,
    #[serde(default)]
    options: Vec<RawChoice>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ConfigOption {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: Kind,
    pub default: Value,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub group: String,
    pub input: Input,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub unit: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<Choice>,
}

enum Checked {
    Ok(Value),
    Adjusted(Value),
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Saved {
    pub version: String,
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
}

pub type SavedAll = BTreeMap<String, Saved>;

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Resolved {
    pub values: BTreeMap<String, Value>,
    pub changed: Vec<String>,
    pub notes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_for: Option<String>,
}

pub const CONFIG_FILE: &str = "config.json";
pub const LEGACY_SETTINGS_FILE: &str = "mod_settings.json";


pub fn valid_key(k: &str) -> bool {
    (1..=64).contains(&k.len())
        && k.starts_with(|c: char| c.is_ascii_alphabetic())
        && k.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn load(mod_dir: &Path) -> Result<Vec<ConfigOption>> {
    let p = mod_dir.join(CONFIG_FILE);
    if !p.is_file() {
        return Ok(vec![]);
    }
    let text = std::fs::read_to_string(&p).with_context(|| format!("reading {CONFIG_FILE}"))?;
    parse(&text).with_context(|| format!("in {CONFIG_FILE}"))
}

pub fn parse(text: &str) -> Result<Vec<ConfigOption>> {
    let raw: Vec<RawOption> = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
    let mut out: Vec<ConfigOption> = Vec::new();
    for r in raw {
        let k = r.key.clone();
        if !valid_key(&k) {
            bail!("key '{k}': use 1-64 characters of A-Z a-z 0-9 _ - . starting with a letter");
        }
        if out.iter().any(|o| o.key == k) {
            bail!("key '{k}' is declared twice");
        }
        if let (Some(a), Some(b)) = (r.min, r.max) {
            if a > b {
                bail!("key '{k}': min is larger than max");
            }
        }
        if r.step.is_some_and(|s| s <= 0.0) {
            bail!("key '{k}': step must be above 0");
        }
        let options: Vec<Choice> = r
            .options
            .iter()
            .map(|c| match c {
                RawChoice::Plain(v) => Choice { value: value_text(v), label: value_text(v) },
                RawChoice::Full { value, label } => {
                    Choice { value: value_text(value), label: if label.is_empty() { value_text(value) } else { label.clone() } }
                }
            })
            .collect();
        match r.kind {
            Kind::Choice if options.is_empty() => bail!("key '{k}': a choice needs 'options'"),
            Kind::Choice => {}
            _ if !options.is_empty() => bail!("key '{k}': 'options' is only for type \"choice\""),
            _ => {}
        }
        if r.input == Input::Slider && !(matches!(r.kind, Kind::Int | Kind::Float) && r.min.is_some() && r.max.is_some()) {
            bail!("key '{k}': a slider needs type int or float and both 'min' and 'max'");
        }
        let mut o = ConfigOption {
            label: if r.label.trim().is_empty() { k.clone() } else { r.label.trim().to_string() },
            key: k.clone(),
            kind: r.kind,
            default: Value::Null,
            description: r.description,
            group: r.group,
            input: r.input,
            min: r.min,
            max: r.max,
            step: r.step,
            unit: r.unit,
            options,
        };
        o.default = match &r.default {
            None => o.fallback_default(),
            Some(v) => match o.check(v) {
                Checked::Ok(v) => v,
                _ => bail!("key '{k}': default {v} is not a valid {}", o.kind_name()),
            },
        };
        out.push(o);
    }
    Ok(out)
}


impl ConfigOption {
    fn kind_name(&self) -> &'static str {
        match self.kind {
            Kind::Int => "whole number",
            Kind::Float => "number",
            Kind::Bool => "true/false",
            Kind::String => "text",
            Kind::Choice => "choice",
        }
    }

    fn fallback_default(&self) -> Value {
        match self.kind {
            Kind::Int => Value::from(self.min.map_or(0, |m| m.ceil() as i64).max(0).min(self.max.map_or(i64::MAX, |m| m.floor() as i64))),
            Kind::Float => Value::from(self.min.unwrap_or(0.0).max(0.0).min(self.max.unwrap_or(f64::MAX))),
            Kind::Bool => Value::Bool(false),
            Kind::String => Value::String(String::new()),
            Kind::Choice => Value::String(self.options[0].value.clone()),
        }
    }

    fn clamp(&self, x: f64) -> f64 {
        let x = self.min.map_or(x, |m| x.max(m));
        self.max.map_or(x, |m| x.min(m))
    }

    fn check(&self, v: &Value) -> Checked {
        let num = match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        };
        match self.kind {
            Kind::Int => {
                let Some(x) = num.filter(|x| x.is_finite()) else { return Checked::Invalid };
                let lo = self.min.map_or(i64::MIN as f64, f64::ceil);
                let hi = self.max.map_or(i64::MAX as f64, f64::floor);
                let y = x.round().clamp(lo, hi) as i64;
                if y as f64 == x && v.is_number() { Checked::Ok(Value::from(y)) } else { Checked::Adjusted(Value::from(y)) }
            }
            Kind::Float => {
                let Some(x) = num.filter(|x| x.is_finite()) else { return Checked::Invalid };
                let y = self.clamp(x);
                if y == x && v.is_number() { Checked::Ok(Value::from(y)) } else { Checked::Adjusted(Value::from(y)) }
            }
            Kind::Bool => match v {
                Value::Bool(b) => Checked::Ok(Value::Bool(*b)),
                _ => Checked::Invalid,
            },
            Kind::String => match v {
                Value::String(s) => {
                    let clean = clean_text(s);
                    if &clean == s { Checked::Ok(Value::String(clean)) } else { Checked::Adjusted(Value::String(clean)) }
                }
                Value::Number(_) | Value::Bool(_) => Checked::Adjusted(Value::String(v.to_string())),
                _ => Checked::Invalid,
            },
            Kind::Choice => {
                let t = value_text(v);
                if self.options.iter().any(|c| c.value == t) { Checked::Ok(Value::String(t)) } else { Checked::Invalid }
            }
        }
    }

    pub fn shown(&self, v: &Value) -> String {
        let t = self.render(v);
        self.options.iter().find(|c| c.value == t).map_or(t, |c| c.label.clone())
    }

    pub fn render(&self, v: &Value) -> String {
        match (self.kind, v) {
            (Kind::Int, Value::Number(n)) => n.as_f64().map_or_else(|| n.to_string(), |x| (x as i64).to_string()),
            (Kind::Float, Value::Number(n)) => fmt_float(n.as_f64().unwrap_or(0.0)),
            _ => value_text(v),
        }
    }
}

fn clean_text(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).map(|c| if c == '"' { '\'' } else if c == '\\' { '/' } else { c }).collect()
}

fn fmt_float(x: f64) -> String {
    let s = format!("{x}");
    if s.contains('e') { format!("{x:.6}") } else { s }
}


pub fn load_legacy(manager_dir: &Path) -> Option<SavedAll> {
    serde_json::from_str(&std::fs::read_to_string(manager_dir.join(LEGACY_SETTINGS_FILE)).ok()?).ok()
}


pub fn resolve(options: &[ConfigOption], saved: Option<&Saved>, version: &str) -> Resolved {
    let mut r = Resolved::default();
    let stored = saved.map(|s| &s.values);
    for o in options {
        let mut v = o.default.clone();
        if let Some(sv) = stored.and_then(|m| m.get(&o.key)) {
            match o.check(sv) {
                Checked::Ok(x) => v = x,
                Checked::Adjusted(x) => {
                    r.notes.push(format!("{}: {} changed to {} to fit this version", o.label, value_text(sv), o.render(&x)));
                    v = x;
                }
                Checked::Invalid => r.notes.push(format!(
                    "{}: {} is no longer a valid {}; reset to the default ({})",
                    o.label,
                    value_text(sv),
                    o.kind_name(),
                    o.shown(&o.default)
                )),
            }
        }
        if v != o.default {
            r.changed.push(o.key.clone());
        }
        r.values.insert(o.key.clone(), v);
    }
    if let Some(m) = stored {
        for k in m.keys().filter(|k| !options.iter().any(|o| &o.key == *k)) {
            r.notes.push(format!("'{k}' was removed from the mod; your value for it is dropped"));
        }
    }
    if let Some(s) = saved.filter(|s| !s.values.is_empty() && s.version != version) {
        r.saved_for = Some(s.version.clone());
    }
    r
}

pub fn to_saved(options: &[ConfigOption], values: &BTreeMap<String, Value>, version: &str) -> Result<Saved> {
    let mut out = Saved { version: version.to_string(), values: BTreeMap::new() };
    for (k, v) in values {
        let o = options.iter().find(|o| &o.key == k).with_context(|| format!("the mod has no setting '{k}'"))?;
        let v = match o.check(v) {
            Checked::Ok(x) | Checked::Adjusted(x) => x,
            Checked::Invalid => bail!("{}: {} is not a valid {}", o.label, value_text(v), o.kind_name()),
        };
        if v != o.default {
            out.values.insert(k.clone(), v);
        }
    }
    Ok(out)
}

pub fn hash(values: &BTreeMap<String, Value>) -> String {
    let text = serde_json::to_string(values).unwrap_or_default();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}


fn tokens(bytes: &[u8]) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' && (i == 0 || bytes[i - 1] != b'<') {
            let end = bytes[i + 1..].iter().take(66).position(|&b| b == b'>').map(|p| i + 1 + p);
            if let Some(e) = end {
                if bytes.get(e + 1) != Some(&b'>') {
                    if let Ok(k) = std::str::from_utf8(&bytes[i + 1..e]) {
                        if valid_key(k) {
                            out.push((i, e + 1, k));
                            i = e + 1;
                            continue;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    out
}

// An undeclared `<word>` only warns where a value would go: not in a comment, and not glued to
// other text (`tech_<id>_name` in a comment explains a pattern).
fn looks_like_setting(bytes: &[u8], a: usize, b: usize) -> bool {
    let word = |c: Option<&u8>| c.is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
    if word(a.checked_sub(1).and_then(|i| bytes.get(i))) || word(bytes.get(b)) {
        return false;
    }
    let line_start = bytes[..a].iter().rposition(|&c| c == b'\n').map_or(0, |p| p + 1);
    let line = bytes[line_start..a].trim_ascii_start();
    !(line.starts_with(b"#") || line.starts_with(b"//"))
}

pub fn fill(bytes: &[u8], texts: &BTreeMap<String, String>) -> (Vec<u8>, BTreeSet<String>, BTreeSet<String>) {
    let mut out = Vec::with_capacity(bytes.len());
    let mut used = BTreeSet::new();
    let mut unknown = BTreeSet::new();
    let mut last = 0;
    for (a, b, k) in tokens(bytes) {
        match texts.get(k) {
            Some(t) => {
                out.extend_from_slice(&bytes[last..a]);
                out.extend_from_slice(t.as_bytes());
                last = b;
                used.insert(k.to_string());
            }
            None if looks_like_setting(bytes, a, b) => {
                unknown.insert(k.to_string());
            }
            None => {}
        }
    }
    out.extend_from_slice(&bytes[last..]);
    (out, used, unknown)
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn opts() -> Vec<ConfigOption> {
        parse(
            r#"[
            {"key":"price","label":"Price","type":"int","default":40000,"min":0,"max":100000,"input":"slider"},
            {"key":"mult","type":"float","default":1.5},
            {"key":"enabled","type":"bool","default":true},
            {"key":"name","type":"string","default":"Mega"},
            {"key":"mode","type":"choice","default":"b","options":["a",{"value":"b","label":"Bee"}]}
        ]"#,
        )
        .unwrap()
    }

    #[test]
    fn parses_and_rejects() {
        let o = opts();
        assert_eq!(o.len(), 5);
        assert_eq!(o[1].label, "mult");
        assert_eq!(o[4].options[1].label, "Bee");
        assert!(parse(r#"[{"key":"x","type":"int","input":"slider"}]"#).is_err());
        assert!(parse(r#"[{"key":"x","type":"int","default":"no"}]"#).is_err());
        assert!(parse(r#"[{"key":"x","type":"int"},{"key":"x","type":"bool"}]"#).is_err());
        assert!(parse(r#"[{"key":"x","type":"choice"}]"#).is_err());
        assert!(parse(r#"[{"key":"x","type":"int","colour":1}]"#).is_err());
        assert_eq!(parse(r#"[{"key":"x","type":"int","min":5}]"#).unwrap()[0].default, json!(5));
    }

    #[test]
    fn fills_placeholders() {
        let o = opts();
        let texts: BTreeMap<String, String> = o.iter().map(|x| (x.key.clone(), x.render(&x.default))).collect();
        let src = b"cost <price>;\nmul <mult>; a <enabled> \"<name>\" <<price>> <other> <mode>\n# <note> x_<id>";
        let (out, used, unknown) = fill(src, &texts);
        assert_eq!(String::from_utf8(out).unwrap(), "cost 40000;\nmul 1.5; a true \"Mega\" <<price>> <other> b\n# <note> x_<id>");
        assert_eq!(used.len(), 5);
        assert_eq!(unknown.into_iter().collect::<Vec<_>>(), vec!["other"]);
    }

    #[test]
    fn saved_values_survive_changes() {
        let o = opts();
        let mut vals = BTreeMap::new();
        vals.insert("price".to_string(), json!(20000));
        vals.insert("enabled".to_string(), json!(true)); // same as default: not stored
        let s = to_saved(&o, &vals, "1.0").unwrap();
        assert_eq!(s.values.len(), 1);
        let r = resolve(&o, Some(&s), "1.0");
        assert_eq!(r.values["price"], json!(20000));
        assert_eq!(r.changed, vec!["price"]);
        assert!(r.notes.is_empty() && r.saved_for.is_none());

        // new version: range shrinks, a key goes away, a type changes
        let o2 = parse(r#"[{"key":"price","type":"int","default":500,"max":10000},{"key":"mode","type":"int","default":1}]"#).unwrap();
        let mut old = s.clone();
        old.values.insert("mode".into(), json!("b"));
        old.values.insert("gone".into(), json!(1));
        let r = resolve(&o2, Some(&old), "2.0");
        assert_eq!(r.values["price"], json!(10000));
        assert_eq!(r.values["mode"], json!(1));
        assert_eq!(r.notes.len(), 3);
        assert_eq!(r.saved_for.as_deref(), Some("1.0"));
        assert!(to_saved(&o2, &BTreeMap::from([("nope".to_string(), json!(1))]), "2.0").is_err());
    }
}
