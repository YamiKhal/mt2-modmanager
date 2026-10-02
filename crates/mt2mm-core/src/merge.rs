use crate::record::{Record, Token};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Conflict,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub level: Level,
    pub mod_id: String,
    pub file: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Report {
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Ident {
    Keyed(String, &'static str, String),
    Unique(String),
    Member(String, Vec<String>),
    None,
}

struct Siblings {
    counts: HashMap<String, usize>,
}

pub struct Merger<'a> {
    pub file: &'a str,
    pub keyed_lines: bool,
    pub report: &'a mut Report,
    owners: HashMap<String, (String, Vec<String>)>,
}

pub const KEY_FIELDS: &[&str] = &["id", "name", "boneName", "slot"];
pub const REMOVE: &str = "__remove";
pub const REPLACE: &str = "__replace";
// Top-level lines keyed by their first word, like i18n strings: `chair_a 0.72`.
pub const KEYED_LINE_FILES: &[&str] = &["base_scales.txt"];
// Records a mod replaces whole: merging two animations' keyframes would mix them.
pub const WHOLE_RECORDS: &[&str] = &["mmoAnimation"];
// One-line word lists mods add to: the vehicle tool's `typeNames "ship" "balloon" …` lists every vehicle it offers.
pub const UNION_LINES: &[&str] = &["typeNames"];


impl Report {
    pub fn push(&mut self, level: Level, ns: &str, file: &str, path: &str, msg: impl Into<String>) {
        self.events.push(Event { level, mod_id: ns.into(), file: file.into(), path: path.into(), message: msg.into() });
    }
    pub fn count(&self, level: Level) -> usize {
        self.events.iter().filter(|e| e.level == level).count()
    }
}


pub fn value_text(r: &Record) -> Vec<String> {
    r.values().iter().filter_map(|t| t.text().map(str::to_string)).collect()
}

impl Ident {
    fn describe(&self) -> String {
        match self {
            Ident::Keyed(l, k, v) => format!("{l}[{k}={v}]"),
            Ident::Unique(l) => l.clone(),
            Ident::Member(l, v) => format!("{l} {}", v.join(" ")),
            Ident::None => "?".into(),
        }
    }
}

fn is_block(r: &Record) -> bool {
    r.had_block || !r.children.is_empty()
}

fn key_of(r: &Record) -> Option<(&'static str, String)> {
    for k in KEY_FIELDS {
        if let Some(v) = r.prop(k) {
            return Some((k, v.to_string()));
        }
    }
    None
}

fn label_of(r: &Record) -> String {
    r.label.clone().unwrap_or_default()
}

impl Siblings {
    fn of(a: &[Record], b: &[Record]) -> Self {
        let mut ca: HashMap<String, usize> = HashMap::new();
        let mut cb: HashMap<String, usize> = HashMap::new();
        for r in a {
            *ca.entry(label_of(r)).or_default() += 1;
        }
        for r in b {
            if r.label.as_deref() != Some(REMOVE) {
                *cb.entry(label_of(r)).or_default() += 1;
            }
        }
        let mut counts = ca;
        for (k, v) in cb {
            let e = counts.entry(k).or_default();
            *e = (*e).max(v);
        }
        Siblings { counts }
    }
    fn repeats(&self, label: &str) -> bool {
        self.counts.get(label).copied().unwrap_or(0) > 1
    }
}

fn ident(r: &Record, sib: &Siblings, top_level: bool, keyed_lines: bool) -> Ident {
    let label = label_of(r);
    if is_block(r) {
        if let Some((k, v)) = key_of(r) {
            return Ident::Keyed(label, k, v);
        }
        if !sib.repeats(&label) && !label.is_empty() {
            return Ident::Unique(label);
        }
        return Ident::None;
    }
    if top_level && keyed_lines && !label.is_empty() {
        return Ident::Unique(label);
    }
    if top_level || sib.repeats(&label) || label.is_empty() {
        return Ident::Member(label, value_text(r));
    }
    Ident::Unique(label)
}


impl<'a> Merger<'a> {
    pub fn new(file: &'a str, report: &'a mut Report) -> Self {
        let keyed_lines = crate::util::i18n_language(file).is_some() || KEYED_LINE_FILES.contains(&file);
        Merger { file, keyed_lines, report, owners: HashMap::new() }
    }

    pub fn apply(&mut self, base: &mut Vec<Record>, patch: &[Record], ns: &str) {
        self.merge_list(base, patch, ns, "", true);
    }

    fn merge_list(&mut self, base: &mut Vec<Record>, patch: &[Record], ns: &str, path: &str, top: bool) {
        let sib = Siblings::of(base, patch);
        let build_index = |base: &Vec<Record>, keyed_lines: bool| {
            let mut idx: HashMap<Ident, usize> = HashMap::new();
            for (i, b) in base.iter().enumerate() {
                let id = ident(b, &sib, top, keyed_lines);
                if id != Ident::None {
                    idx.entry(id).or_insert(i);
                }
            }
            idx
        };
        let mut index = build_index(base, self.keyed_lines);
        for p in patch {
            if p.label.as_deref() == Some(REMOVE) {
                self.remove(base, p, ns, path, top);
                index = build_index(base, self.keyed_lines);
                continue;
            }
            let id = ident(p, &sib, top, self.keyed_lines);
            let here = format!("{path}/{}", id.describe());
            let found = if id == Ident::None { None } else { index.get(&id).copied() };
            match found {
                None => {
                    let mut rec = p.clone();
                    strip_directives(&mut rec);
                    self.claim_tree(&rec, ns, &here);
                    if id != Ident::None {
                        index.insert(id, base.len());
                    }
                    base.push(rec);
                }
                Some(i) => {
                    if is_block(p) {
                        let replace = p.children.iter().any(|c| c.label.as_deref() == Some(REPLACE))
                            || p.label.as_deref().is_some_and(|l| WHOLE_RECORDS.contains(&l));
                        if replace {
                            let mut rec = p.clone();
                            strip_directives(&mut rec);
                            self.note_change(ns, &here, &value_text(&rec), true);
                            self.claim_tree(&rec, ns, &here);
                            base[i] = rec;
                        } else {
                            if !p.tokens.is_empty() && value_text(&base[i]) != value_text(p) {
                                self.note_change(ns, &here, &value_text(p), false);
                                base[i].tokens = p.tokens.clone();
                            }
                            let mut children = std::mem::take(&mut base[i].children);
                            self.merge_list(&mut children, &p.children, ns, &here, false);
                            base[i].children = children;
                            base[i].had_block = true;
                        }
                    } else if matches!(id, Ident::Unique(_)) && p.label.as_deref().is_some_and(|l| UNION_LINES.contains(&l)) {
                        base[i].tokens = union_tokens(&base[i], p);
                        self.owners.insert(here, (ns.to_string(), value_text(&base[i])));
                    } else if matches!(id, Ident::Unique(_)) {
                        let new = value_text(p);
                        if value_text(&base[i]) != new {
                            self.note_change(ns, &here, &new, false);
                            base[i].tokens = p.tokens.clone();
                        } else {
                            // same value as current: still record who last asserted it
                            self.owners.insert(here, (ns.to_string(), new));
                        }
                    }
                }
            }
        }
    }

    fn remove(&mut self, base: &mut Vec<Record>, p: &Record, ns: &str, path: &str, top: bool) {
        let v = value_text(p);
        let Some((label, rest)) = v.split_first() else {
            self.report.push(Level::Warning, ns, self.file, path, "__remove without a label is ignored");
            return;
        };
        let before = base.len();
        base.retain(|b| {
            if b.label.as_deref() != Some(label.as_str()) {
                return true;
            }
            if rest.is_empty() {
                return false;
            }
            if is_block(b) {
                key_of(b).map_or(true, |(_, k)| k != rest[0])
            } else {
                value_text(b) != rest
            }
        });
        let removed = before - base.len();
        let what = format!("{path}/{}", v.join(" "));
        if removed == 0 {
            self.report.push(Level::Warning, ns, self.file, &what, "__remove matched nothing");
        } else {
            let _ = top;
            self.report.push(Level::Info, ns, self.file, &what, format!("removed {removed} entr{}", if removed == 1 { "y" } else { "ies" }));
        }
    }

    fn note_change(&mut self, ns: &str, path: &str, new: &[String], whole: bool) {
        if let Some((prev_ns, prev_val)) = self.owners.get(path) {
            if prev_ns != ns && (whole || prev_val.as_slice() != new) {
                self.report.push(
                    Level::Conflict,
                    ns,
                    self.file,
                    path,
                    format!("overrides '{prev_ns}' ({} -> {})", show(prev_val), show(new)),
                );
            }
        }
        self.owners.insert(path.to_string(), (ns.to_string(), new.to_vec()));
    }

    fn claim_tree(&mut self, r: &Record, ns: &str, path: &str) {
        self.owners.insert(path.to_string(), (ns.to_string(), value_text(r)));
        let sib = Siblings::of(&r.children, &[]);
        for c in &r.children {
            let id = ident(c, &sib, false, false);
            self.claim_tree(c, ns, &format!("{path}/{}", id.describe()));
        }
    }
}

fn union_tokens(base: &Record, patch: &Record) -> Vec<Token> {
    let mut tokens: Vec<Token> = base.values().into_iter().cloned().collect();
    for token in patch.values() {
        if !tokens.iter().any(|t| t.text() == token.text()) {
            tokens.push(token.clone());
        }
    }
    if base.tokens.last().is_some_and(Token::is_semicolon) {
        tokens.push(Token::Semicolon);
    }

    tokens
}

fn show(v: &[String]) -> String {
    if v.is_empty() {
        "(block)".into()
    } else {
        v.join(" ")
    }
}


fn strip_directives(r: &mut Record) {
    r.children.retain(|c| !matches!(c.label.as_deref(), Some(REMOVE) | Some(REPLACE)));
    for c in r.children.iter_mut() {
        strip_directives(c);
    }
}

pub fn strip_all_directives(records: &mut Vec<Record>) {
    records.retain(|c| !matches!(c.label.as_deref(), Some(REMOVE) | Some(REPLACE)));
    for r in records.iter_mut() {
        strip_directives(r);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{parse, Document};

    fn recs(s: &str) -> Vec<Record> {
        parse(s.as_bytes()).unwrap().records
    }
    fn text(r: &[Record]) -> String {
        String::from_utf8(Document { records: r.to_vec(), crlf: false, latin1: false }.to_bytes()).unwrap()
    }

    const POWERS: &str = "mmoItemTypeWindow\n{\n\tname \"@Finance\"\n\twindowName \"daily_finance\"\n}\nmmoItemTypeCommand\n{\n\tname \"@Web\"\n\tcommand \"MessageTo Game OpenWeb\"\n}\n";

    #[test]
    fn add_and_patch_keyed_entries() {
        let mut base = recs(POWERS);
        let mut rep = Report::default();
        let mut m = Merger::new("Powers.txt", &mut rep);
        m.apply(&mut base, &recs("mmoItemTypeCommand\n{\n\tname \"@Cash\"\n\tcommand \"add_cash 1\"\n}\nmmoItemTypeWindow\n{\n\tname \"@Finance\"\n\ticonName \"icon-x\"\n}\n"), "a");
        assert_eq!(base.len(), 3);
        assert_eq!(base[0].prop("windowName"), Some("daily_finance"));
        assert_eq!(base[0].prop("iconName"), Some("icon-x"));
        assert_eq!(base[2].prop("name"), Some("@Cash"));
    }

    #[test]
    fn base_scales_lines_replace_by_name() {
        let mut base = recs("chair_a 0.72
bench_a 0.72
");
        let mut rep = Report::default();
        let mut m = Merger::new("base_scales.txt", &mut rep);
        m.apply(&mut base, &recs("chair_a 0.9
mymod_rock 0.5
"), "a");
        assert_eq!(text(&base), "chair_a 0.9
bench_a 0.72
mymod_rock 0.5
");
    }

    #[test]
    fn animations_replace_whole() {
        let old = "mmoAnimation
{
	name \"idle\"
	timeline
	{
		mmoAnimationNodeTimeline
		{
			nodeName \"head\"
		}
		mmoAnimationNodeTimeline
		{
			nodeName \"torso\"
		}
	}
}
mmoAnimation
{
	name \"run\"
}
";
        let new = "mmoAnimation
{
	name \"idle\"
	timeline
	{
		mmoAnimationNodeTimeline
		{
			nodeName \"hat\"
		}
	}
}
";
        let mut base = recs(old);
        let mut rep = Report::default();
        let mut m = Merger::new("skeletons/humanoid.van", &mut rep);
        m.apply(&mut base, &recs(new), "a");
        assert_eq!(base.len(), 2);
        let timelines = &base[0].children.iter().find(|c| c.label.as_deref() == Some("timeline")).unwrap().children;
        assert_eq!(timelines.len(), 1);
        assert_eq!(timelines[0].prop("nodeName"), Some("hat"));
        assert_eq!(base[1].prop("name"), Some("run"));
    }

    #[test]
    fn vehicle_names_add_up() {
        let mut base = recs("mmoCursorBehaviourVehicle\n{\n\tname \"@Travel Vehicle\"\n\ttypeNames \"ship\" \"balloon\"\n\ticonName \"icon-vehicle\"\n}\n");
        let mut rep = Report::default();
        let mut m = Merger::new("CursorBehaviours.txt", &mut rep);
        let patch = |id: &str| recs(&format!("mmoCursorBehaviourVehicle\n{{\n\tname \"@Travel Vehicle\"\n\ttypeNames \"{id}\"\n}}\n"));
        m.apply(&mut base, &patch("a_blimp"), "a");
        m.apply(&mut base, &patch("b_raft"), "b");
        assert_eq!(value_text(&base[0].children[1]), vec!["ship", "balloon", "a_blimp", "b_raft"]);
        assert_eq!(base[0].prop("iconName"), Some("icon-vehicle"));
        assert_eq!(rep.count(Level::Conflict), 0);
    }

    #[test]
    fn set_members_and_remove() {
        let mut base = recs("Inventory\n{\n\tItem \"@A\";\n\tItem \"@B\";\n}\n");
        let mut rep = Report::default();
        let mut m = Merger::new("EditInventory.cfg", &mut rep);
        m.apply(&mut base, &recs("Inventory\n{\n\tItem \"@B\";\n\tItem \"@C\";\n\t__remove Item \"@A\"\n}\n"), "a");
        let items: Vec<_> = base[0].children.iter().map(|c| c.first_text().unwrap().to_string()).collect();
        assert_eq!(items, vec!["@B", "@C"]);
    }

    #[test]
    fn conflicts_between_mods() {
        let mut base = recs("mmoBusinessConfig\n{\n\tdevSalary 500;\n}\n");
        let mut rep = Report::default();
        {
            let mut m = Merger::new("businessconfig.vrt", &mut rep);
            m.apply(&mut base, &recs("mmoBusinessConfig\n{\n\tdevSalary 400;\n}\n"), "a");
            m.apply(&mut base, &recs("mmoBusinessConfig\n{\n\tdevSalary 300;\n}\n"), "b");
        }
        assert_eq!(base[0].prop("devSalary"), Some("300"));
        assert_eq!(rep.count(Level::Conflict), 1);
    }

    #[test]
    fn nested_slots_and_replace() {
        let base_s = "mmoActionBar\n{\n\tid \"Edit\";\n\tactionBarContent\n\t{\n\t\tmmoActionBarContent\n\t\t{\n\t\t\tid \"Paths\";\n\t\t\tconfiguration\n\t\t\t{\n\t\t\t\tmmoActionBarButtonConfiguration\n\t\t\t\t{\n\t\t\t\t\tslot 0;\n\t\t\t\t\titemName \"@Road\";\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
        let mut base = recs(base_s);
        let mut rep = Report::default();
        let mut m = Merger::new("actionbars.win", &mut rep);
        let patch = "mmoActionBar\n{\n\tid \"Edit\";\n\tactionBarContent\n\t{\n\t\tmmoActionBarContent\n\t\t{\n\t\t\tid \"Paths\";\n\t\t\tconfiguration\n\t\t\t{\n\t\t\t\tmmoActionBarButtonConfiguration\n\t\t\t\t{\n\t\t\t\t\tslot 9;\n\t\t\t\t\titemName \"@X\";\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
        m.apply(&mut base, &recs(patch), "a");
        let cfg = &base[0].children[1].children[0].children[1];
        assert_eq!(cfg.children.len(), 2);
        assert_eq!(cfg.children[1].prop("itemName"), Some("@X"));
        let patch2 = "mmoActionBar\n{\n\tid \"Edit\";\n\tactionBarContent\n\t{\n\t\tmmoActionBarContent\n\t\t{\n\t\t\tid \"Paths\";\n\t\t\t__replace\n\t\t}\n\t}\n}\n";
        m.apply(&mut base, &recs(patch2), "b");
        let content = &base[0].children[1].children[0];
        assert_eq!(content.children.len(), 1, "{}", text(&base));
    }

    #[test]
    fn i18n_overrides_by_key() {
        let mut base = recs("a_key \"old\"\nb_key \"keep\"\n");
        let mut rep = Report::default();
        let mut m = Merger::new("i18n/english/00-base.vrt", &mut rep);
        m.apply(&mut base, &recs("a_key \"new\"\nc_key \"added\"\n"), "a");
        assert_eq!(text(&base), "a_key \"new\"\nb_key \"keep\"\nc_key \"added\"\n");
    }

    #[test]
    fn word_lists_are_sets() {
        let mut base = recs("Warrior\nDark Knight\n");
        let mut rep = Report::default();
        let mut m = Merger::new("ClassNoun.txt", &mut rep);
        m.apply(&mut base, &recs("Dark Lord\nWarrior\n"), "a");
        assert_eq!(text(&base), "Warrior\nDark Knight\nDark Lord\n");
    }
}
