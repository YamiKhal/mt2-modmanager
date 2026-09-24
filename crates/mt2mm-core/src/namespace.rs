use crate::merge::{Level, Report};
use crate::record::{Record, Token};
use crate::vanilla::Vanilla;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Tech,
    Item,
    ItemKey,
    BarTab,
    TabKey,
    Scenario,
}

#[derive(Debug, Default)]
pub struct VanillaIds {
    pub ids: HashMap<Kind, HashSet<String>>,
    pub i18n_keys: HashSet<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Renames {
    pub tokens: BTreeMap<String, String>,
    pub keys: BTreeMap<String, String>,
    pub list: Vec<(Kind, String, String)>,
}

const TEXT_FIELDS: &[&str] = &["tooltip", "Tooltip", "text", "title", "description", "displayName", "label", "service"];


impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Tech => "tech",
            Kind::Item => "item",
            Kind::ItemKey => "item key",
            Kind::BarTab => "action bar tab",
            Kind::TabKey => "tab key",
            Kind::Scenario => "scenario",
        }
    }
}


fn is_item_file(rel: &str) -> bool {
    rel == "Powers.txt" || rel == "CursorBehaviours.txt"
}

fn definitions(rel: &str, records: &[Record], out: &mut Vec<(Kind, String)>) {
    if rel.starts_with("techs/") {
        for r in records {
            if r.label.as_deref().is_some_and(|l| l.starts_with("mmoTech")) {
                if let Some(id) = r.prop("id") {
                    out.push((Kind::Tech, id.to_string()));
                }
            }
        }
    }
    if is_item_file(rel) {
        for r in records {
            if let Some(n) = r.prop("name") {
                out.push((Kind::Item, n.to_string()));
            }
            if let Some(k) = r.prop("key") {
                out.push((Kind::ItemKey, k.to_string()));
            }
        }
    }
    if rel == "actionbars.win" {
        fn walk(r: &Record, out: &mut Vec<(Kind, String)>) {
            match r.label.as_deref() {
                Some("mmoActionBarTab") => {
                    if let Some(n) = r.prop("name") {
                        out.push((Kind::BarTab, n.to_string()));
                    }
                    if let Some(k) = r.prop("key") {
                        out.push((Kind::TabKey, k.to_string()));
                    }
                }
                Some("mmoActionBarContent") => {
                    if let Some(n) = r.prop("id") {
                        out.push((Kind::BarTab, n.to_string()));
                    }
                }
                _ => {}
            }
            for c in &r.children {
                walk(c, out);
            }
        }
        for r in records {
            walk(r, out);
        }
    }
    if rel == "scenarios/library.txt" {
        fn walk(r: &Record, out: &mut Vec<(Kind, String)>) {
            if r.label.as_deref().is_some_and(|l| l.starts_with("mmoScenario")) {
                if let Some(id) = r.prop("id") {
                    out.push((Kind::Scenario, id.to_string()));
                }
            }
            for c in &r.children {
                walk(c, out);
            }
        }
        for r in records {
            walk(r, out);
        }
    }
}


impl VanillaIds {
    pub fn collect(v: &Vanilla) -> anyhow::Result<Self> {
        let mut out = VanillaIds::default();
        for rel in v.paths() {
            let wanted = rel.starts_with("techs/") || is_item_file(&rel) || rel == "actionbars.win" || rel == "scenarios/library.txt";
            let is_i18n = crate::util::i18n_language(&rel).is_some();
            if !(wanted || is_i18n) {
                continue;
            }
            let Some(bytes) = v.read(&rel)? else { continue };
            let Ok(doc) = crate::record::parse(&bytes) else { continue };
            if is_i18n {
                for r in &doc.records {
                    if let Some(l) = &r.label {
                        out.i18n_keys.insert(l.clone());
                    }
                }
                continue;
            }
            let mut defs = Vec::new();
            definitions(&rel, &doc.records, &mut defs);
            for (k, id) in defs {
                out.ids.entry(k).or_default().insert(id);
            }
        }
        Ok(out)
    }

    pub fn has(&self, k: Kind, id: &str) -> bool {
        self.ids.get(&k).is_some_and(|s| s.contains(id))
    }
}


fn prefixed(ns: &str, kind: Kind, id: &str) -> Option<String> {
    let p = format!("{ns}_");
    if kind == Kind::Item {
        if let Some(rest) = id.strip_prefix('@') {
            return if rest.starts_with(&p) { None } else { Some(format!("@{p}{rest}")) };
        }
    }
    if id.starts_with(&p) {
        None
    } else {
        Some(format!("{p}{id}"))
    }
}

// Saves store ids by value, so a mod's ids must never depend on which other mods are installed:
// every id a mod adds always gets its prefix.
pub fn plan_renames(
    ns: &str,
    files: &BTreeMap<String, Vec<Record>>,
    vanilla: &VanillaIds,
    known_namespaces: &HashSet<String>,
    report: &mut Report,
) -> Renames {
    let mut defs = Vec::new();
    for (rel, recs) in files {
        definitions(rel, recs, &mut defs);
    }
    let mut r = Renames::default();
    let mut seen = HashSet::new();
    for (kind, id) in defs {
        if !seen.insert((kind, id.clone())) || vanilla.has(kind, &id) || refers_to_other_mod(&id, ns, known_namespaces) {
            continue;
        }
        let Some(new) = prefixed(ns, kind, &id) else { continue };
        match r.tokens.get(&id) {
            Some(existing) if existing != &new => {
                report.push(
                    Level::Warning,
                    ns,
                    "",
                    &id,
                    format!("'{id}' is used as a {} and as another kind of id; renamed to '{existing}'", kind.name()),
                );
                continue;
            }
            Some(_) => {}
            None => {
                r.tokens.insert(id.clone(), new.clone());
            }
        }
        match kind {
            Kind::Tech => {
                for suffix in ["displayname", "description"] {
                    r.keys.insert(format!("tech_{id}_{suffix}"), format!("tech_{new}_{suffix}"));
                }
            }
            Kind::ItemKey => {
                for suffix in ["displayname", "description"] {
                    r.keys.insert(format!("{id}_{suffix}"), format!("{new}_{suffix}"));
                }
            }
            Kind::TabKey => {
                r.keys.insert(id.clone(), new.clone());
            }
            _ => {}
        }
        r.list.push((kind, id, new));
    }
    r
}


fn refers_to_other_mod(id: &str, own: &str, known: &HashSet<String>) -> bool {
    let body = id.strip_prefix('@').unwrap_or(id);
    known.iter().filter(|k| k.as_str() != own).any(|k| {
        body.strip_prefix(k.as_str()).is_some_and(|rest| rest.starts_with(':') || rest.starts_with('_'))
    })
}

fn is_label_text(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|ch| ch.is_ascii_alphanumeric() || "_.+-".contains(ch))
}

fn resolve_cross_ref(s: &str, known: &HashSet<String>) -> Option<String> {
    let (at, body) = match s.strip_prefix('@') {
        Some(b) => ("@", b),
        None => ("", s),
    };
    let (ns, id) = body.split_once(':')?;
    if known.contains(ns) && !id.is_empty() && !id.contains(char::is_whitespace) {
        Some(format!("{at}{ns}_{id}"))
    } else {
        None
    }
}

fn rewrite_braces(s: &str, keys: &BTreeMap<String, String>) -> String {
    if keys.is_empty() || !s.contains('{') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let key = &after[..close];
                out.push('{');
                out.push_str(keys.get(key).map(String::as_str).unwrap_or(key));
                out.push('}');
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn apply(rel: &str, records: &mut [Record], r: &Renames, known_namespaces: &HashSet<String>) -> usize {
    let mut changed = 0usize;
    let i18n = crate::util::i18n_language(rel).is_some();
    for rec in records.iter_mut() {
        if i18n {
            if let Some(l) = rec.label.clone() {
                if let Some(n) = r.keys.get(&l) {
                    rec.label = Some(n.clone());
                    changed += 1;
                }
            }
        }
        rec.for_each_token_mut(&mut |owner, t| {
            let text = match t {
                Token::Str(s) | Token::Label(s) | Token::Bare(s) => s.clone(),
                _ => return,
            };
            // Display text is never an id reference.
            let is_text = i18n || owner.is_some_and(|o| TEXT_FIELDS.contains(&o));
            let mut new = if is_text {
                None
            } else {
                r.tokens.get(&text).cloned().or_else(|| resolve_cross_ref(&text, known_namespaces))
            };
            if new.is_none() {
                if let Token::Str(s) = &*t {
                    let b = rewrite_braces(s, &r.keys);
                    if &b != s {
                        new = Some(b);
                    }
                }
            }
            if let Some(n) = new {
                changed += 1;
                *t = match t {
                    Token::Label(_) if is_label_text(&n) => Token::Label(n),
                    Token::Bare(_) if !n.contains(|c: char| c == ' ' || c == '\t' || c == ',') => Token::Bare(n),
                    _ => Token::Str(n),
                };
            }
        });
    }
    changed
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::parse;

    fn files(list: &[(&str, &str)]) -> BTreeMap<String, Vec<Record>> {
        list.iter().map(|(p, s)| (p.to_string(), parse(s.as_bytes()).unwrap().records)).collect()
    }

    #[test]
    fn prefixes_new_ids_and_rewrites_references() {
        let mut vanilla = VanillaIds::default();
        vanilla.ids.entry(Kind::Tech).or_default().insert("gamelog".into());
        let mut fs = files(&[
            ("techs/x.vrt", "mmoTech\n{\n\tid \"crunch\";\n\tprerequisite \"gamelog\";\n}\nmmoTech\n{\n\tid \"crunch2\";\n\tprerequisite \"crunch\";\n}\n"),
            ("i18n/english/50-x.vrt", "tech_crunch_displayname \"Crunch\"\ntech_crunch2_description \"after {tech_crunch_displayname}\"\n"),
            ("Powers.txt", "mmoItemTypeCommand\n{\n\tname \"@Cash\"\n\tkey \"item_cash\"\n}\n"),
            ("EditInventory.cfg", "Inventory\n{\n\tItem \"@Cash\";\n\tItem \"@Potion\";\n}\n"),
        ]);
        let mut rep = Report::default();
        let r = plan_renames("mymod", &fs, &vanilla, &HashSet::new(), &mut rep);
        let known: HashSet<String> = ["mymod".to_string()].into();
        for (rel, recs) in fs.iter_mut() {
            apply(rel, recs, &r, &known);
        }
        let t = &fs["techs/x.vrt"];
        assert_eq!(t[0].prop("id"), Some("mymod_crunch"));
        assert_eq!(t[0].prop("prerequisite"), Some("gamelog"));
        assert_eq!(t[1].prop("prerequisite"), Some("mymod_crunch"));
        let i = &fs["i18n/english/50-x.vrt"];
        assert_eq!(i[0].label.as_deref(), Some("tech_mymod_crunch_displayname"));
        assert_eq!(i[1].first_text(), Some("after {tech_mymod_crunch_displayname}"));
        assert_eq!(fs["Powers.txt"][0].prop("name"), Some("@mymod_Cash"));
        assert_eq!(fs["Powers.txt"][0].prop("key"), Some("mymod_item_cash"));
        let inv: Vec<_> = fs["EditInventory.cfg"][0].children.iter().map(|c| c.first_text().unwrap().to_string()).collect();
        assert_eq!(inv, vec!["@mymod_Cash", "@Potion"]);
    }

    #[test]
    fn display_text_equal_to_an_id_is_not_renamed() {
        let vanilla = VanillaIds::default();
        let mut fs = files(&[
            ("actionbars.win", "mmoActionBar
{
	tab
	{
		mmoActionBarTab
		{
			name \"Mod\"
			key \"actionbar_tab_mod\";
		}
	}
	actionBarContent
	{
		mmoActionBarContent
		{
			id \"Mod\";
			tooltip \"Mod\"
		}
	}
}
"),
            ("i18n/english/50-x.vrt", "actionbar_tab_mod \"Mod\"
"),
        ]);
        let mut rep = Report::default();
        let r = plan_renames("mymod", &fs, &vanilla, &HashSet::new(), &mut rep);
        for (rel, recs) in fs.iter_mut() {
            apply(rel, recs, &r, &HashSet::new());
        }
        let bar = &fs["actionbars.win"][0];
        assert_eq!(bar.children[0].children[0].prop("name"), Some("mymod_Mod"));
        assert_eq!(bar.children[0].children[0].prop("key"), Some("mymod_actionbar_tab_mod"));
        assert_eq!(bar.children[1].children[0].prop("id"), Some("mymod_Mod"));
        assert_eq!(bar.children[1].children[0].prop("tooltip"), Some("Mod"));
        let i = &fs["i18n/english/50-x.vrt"][0];
        assert_eq!(i.label.as_deref(), Some("mymod_actionbar_tab_mod"));
        assert_eq!(i.first_text(), Some("Mod"));
    }

    #[test]
    fn stable_regardless_of_other_mods_and_idempotent() {
        let vanilla = VanillaIds::default();
        let mut fs = files(&[("techs/x.vrt", "mmoTech\n{\n\tid \"mymod_already\";\n}\n")]);
        let mut rep = Report::default();
        let r = plan_renames("mymod", &fs, &vanilla, &HashSet::new(), &mut rep);
        assert!(r.list.is_empty());
        apply("techs/x.vrt", fs.get_mut("techs/x.vrt").unwrap(), &r, &HashSet::new());
        assert_eq!(fs["techs/x.vrt"][0].prop("id"), Some("mymod_already"));
    }

    #[test]
    fn references_to_other_mods_are_not_definitions() {
        let vanilla = VanillaIds::default();
        let mut fs = files(&[("actionbars.win", "x
{
	mmoActionBarContent
	{
		id \"toolbox:Mod\";
	}
	mmoActionBarContent
	{
		id \"toolbox_Other\";
	}
}
")]);
        let known: HashSet<String> = ["toolbox".to_string(), "cash".to_string()].into();
        let mut rep = Report::default();
        let r = plan_renames("cash", &fs, &vanilla, &known, &mut rep);
        assert!(r.list.is_empty(), "{:?}", r.list);
        apply("actionbars.win", fs.get_mut("actionbars.win").unwrap(), &r, &known);
        assert_eq!(fs["actionbars.win"][0].children[0].prop("id"), Some("toolbox_Mod"));
        assert_eq!(fs["actionbars.win"][0].children[1].prop("id"), Some("toolbox_Other"));
    }

    #[test]
    fn cross_mod_reference() {
        let known: HashSet<String> = ["base".to_string(), "other".to_string()].into();
        assert_eq!(resolve_cross_ref("other:crunch", &known).as_deref(), Some("other_crunch"));
        assert_eq!(resolve_cross_ref("@other:Cash", &known).as_deref(), Some("@other_Cash"));
        assert_eq!(resolve_cross_ref("MessageTo Game:x", &known), None);
        assert_eq!(resolve_cross_ref("nope:x", &known), None);
    }
}
