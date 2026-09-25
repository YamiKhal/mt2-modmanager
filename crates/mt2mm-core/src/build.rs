use crate::library::LibraryMod;
use crate::manifest::{Manifest, ICON_FILE, MANIFEST_FILE};
use crate::merge::{strip_all_directives, Level, Merger, Report};
use crate::modconfig::{self, CONFIG_FILE};
use crate::namespace::{self, VanillaIds};
use crate::record::{self, Document, Record};
use crate::safety;
use crate::util::{i18n_language, is_record_path, rel_string};
use crate::vanilla::Vanilla;
use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct OutFile {
    pub rel: String,
    #[serde(skip)]
    pub contents: Contents,
    pub how: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Contents {
    Bytes(Vec<u8>),
    // Read only at deploy, so a build check never loads icon packs of hundreds of MB.
    CopyOf(PathBuf),
}

#[derive(Debug, Clone, Serialize)]
pub struct Rename {
    pub mod_id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct BuildPlan {
    pub mods: Vec<String>,
    pub files: Vec<OutFile>,
    pub renames: Vec<Rename>,
    pub report: Report,
    pub game_version: Option<String>,
}

struct ModFiles {
    manifest: Manifest,
    records: BTreeMap<String, Vec<Record>>,
    formats: BTreeMap<String, (bool, bool)>,
    opaque: BTreeMap<String, PathBuf>,
    filled: BTreeMap<String, Vec<u8>>,
}


impl OutFile {
    pub fn read(&self) -> Result<Vec<u8>> {
        match &self.contents {
            Contents::Bytes(bytes) => Ok(bytes.clone()),
            Contents::CopyOf(source) => Ok(std::fs::read(source)?),
        }
    }

    pub fn write_to(&self, dest: &std::path::Path) -> Result<()> {
        match &self.contents {
            Contents::Bytes(bytes) => std::fs::write(dest, bytes)?,
            // Read and write rather than fs::copy: copy would also carry over the source's
            // timestamps and read-only flag, which the deployed files never had.
            Contents::CopyOf(source) => std::fs::write(dest, std::fs::read(source)?)?,
        }
        Ok(())
    }
}

impl BuildPlan {
    pub fn ok(&self) -> bool {
        self.report.count(Level::Error) == 0
    }
}


fn is_doc_file(rel: &str) -> bool {
    if rel.contains('/') {
        return false;
    }
    let l = rel.to_ascii_lowercase();
    ["readme", "license", "licence", "changelog", "credits"].iter().any(|p| l.starts_with(p))
        && (l.ends_with(".md") || l.ends_with(".txt") || !l.contains('.'))
}

fn load_mod(m: &LibraryMod, manifest: &Manifest, report: &mut Report) -> Result<ModFiles> {
    let ns = &manifest.id;
    let mut out = ModFiles {
        manifest: manifest.clone(),
        records: BTreeMap::new(),
        formats: BTreeMap::new(),
        opaque: BTreeMap::new(),
        filled: BTreeMap::new(),
    };
    if let Some(e) = &m.config_error {
        report.push(Level::Error, ns, CONFIG_FILE, "", e.clone());
    }
    for n in &m.settings.notes {
        report.push(Level::Warning, ns, CONFIG_FILE, "", format!("settings: {n}"));
    }
    let texts: BTreeMap<String, String> =
        m.config.iter().map(|o| (o.key.clone(), o.render(m.settings.values.get(&o.key).unwrap_or(&o.default)))).collect();
    let mut used = HashSet::new();
    for e in walkdir::WalkDir::new(&m.dir).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if !e.file_type().is_file() {
            continue;
        }
        let rel = rel_string(e.path().strip_prefix(&m.dir)?);
        let name = e.file_name().to_string_lossy();
        if rel == MANIFEST_FILE || rel == ICON_FILE || rel == CONFIG_FILE || name.starts_with('.') || is_doc_file(&rel) {
            continue;
        }
        if is_record_path(&rel) {
            let raw = std::fs::read(e.path())?;
            let (bytes, keys, unknown) = modconfig::fill(&raw, &texts);
            used.extend(keys);
            if (!m.config.is_empty() || m.config_error.is_some()) && i18n_language(&rel).is_none() {
                for k in unknown {
                    report.push(Level::Warning, ns, &rel, "", format!("<{k}> looks like a setting, but {CONFIG_FILE} doesn't declare '{k}'"));
                }
            }
            match record::parse(&bytes) {
                Ok(doc) => {
                    out.formats.insert(rel.clone(), (doc.crlf, doc.latin1));
                    out.records.insert(rel, doc.records);
                }
                Err(err) => {
                    report.push(Level::Warning, ns, &rel, "", format!("could not parse ({err}); copied as-is, not merged"));
                    if bytes != raw {
                        out.filled.insert(rel.clone(), bytes);
                    }
                    out.opaque.insert(rel, e.path().to_path_buf());
                }
            }
        } else {
            out.opaque.insert(rel, e.path().to_path_buf());
        }
    }
    for o in m.config.iter().filter(|o| !used.contains(&o.key)) {
        report.push(Level::Warning, ns, CONFIG_FILE, "", format!("setting '{}' is not used: no text file contains <{}>", o.key, o.key));
    }
    Ok(out)
}


fn check_mods(mods: &[&Manifest], game_version: Option<&str>, report: &mut Report) -> bool {
    let mut ok = true;
    let mut seen: Vec<&str> = Vec::new();
    for m in mods {
        let ns = m.id.as_str();
        if seen.contains(&ns) {
            report.push(Level::Error, ns, "", "", "two enabled mods use the same mod id");
            ok = false;
        }
        for d in &m.dependencies {
            if !mods.iter().any(|o| &o.id == d) {
                report.push(Level::Error, ns, "", "", format!("needs '{d}', which is not enabled"));
                ok = false;
            } else if !seen.contains(&d.as_str()) {
                report.push(Level::Error, ns, "", "", format!("needs '{d}' to load before it; move it higher in the load order"));
                ok = false;
            }
        }
        for i in &m.incompatible {
            if mods.iter().any(|o| &o.id == i) {
                report.push(Level::Error, ns, "", "", format!("is incompatible with '{i}'"));
                ok = false;
            }
        }
        if let (Some(gv), false) = (game_version, m.game_versions.is_empty()) {
            if !m.game_versions.iter().any(|p| gv.starts_with(p.as_str())) {
                report.push(Level::Warning, ns, "", "", format!("made for game {} but you have {gv}", m.game_versions.join(", ")));
            }
        }
        seen.push(ns);
    }
    ok
}


pub fn plan(vanilla: &Vanilla, mods: &[LibraryMod], game_version: Option<String>) -> Result<BuildPlan> {
    let mut plan = BuildPlan { game_version: game_version.clone(), ..Default::default() };
    let enabled: Vec<(&LibraryMod, &Manifest)> =
        mods.iter().filter(|m| m.enabled).filter_map(|m| m.manifest.as_ref().map(|man| (m, man))).collect();
    for m in mods.iter().filter(|m| m.manifest.is_none()) {
        plan.report.push(Level::Warning, "", &m.folder, "", format!("skipped: {}", m.error.clone().unwrap_or_default()));
    }
    plan.mods = enabled.iter().map(|(_, m)| m.id.clone()).collect();
    let manifests: Vec<&Manifest> = enabled.iter().map(|(_, m)| *m).collect();
    if !check_mods(&manifests, game_version.as_deref(), &mut plan.report) {
        return Ok(plan);
    }
    let known: HashSet<String> = plan.mods.iter().cloned().collect();
    let vids = VanillaIds::collect(vanilla)?;

    let mut loaded: Vec<ModFiles> = Vec::new();
    for (lm, man) in &enabled {
        let mut mf = load_mod(lm, man, &mut plan.report)?;
        for (rel, recs) in &mf.records {
            safety::check_mod_file(&man.id, rel, recs, vanilla, &mut plan.report)?;
        }
        let renames = namespace::plan_renames(&man.id, &mf.records, &vids, &known, &mut plan.report);
        for (rel, recs) in mf.records.iter_mut() {
            namespace::apply(rel, recs, &renames, &known);
        }
        for (kind, from, to) in &renames.list {
            plan.renames.push(Rename { mod_id: man.id.clone(), kind: kind.name().into(), from: from.clone(), to: to.clone() });
        }
        loaded.push(mf);
    }

    let target_of = |rel: &str| -> String {
        match i18n_language(rel) {
            Some(lang) => format!("i18n/{lang}/00-base.vrt"),
            None => rel.to_string(),
        }
    };
    let mut record_targets: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new(); // target -> (mod idx, source rel)
    let mut opaque_targets: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, mf) in loaded.iter().enumerate() {
        for rel in mf.records.keys() {
            record_targets.entry(target_of(rel)).or_default().push((i, rel.clone()));
        }
        for rel in mf.opaque.keys() {
            opaque_targets.entry(rel.clone()).or_default().push(i);
        }
    }

    // case-only differences confuse PhysFS on case-sensitive systems
    {
        let mut lower: BTreeMap<String, String> = BTreeMap::new();
        for p in record_targets.keys().chain(opaque_targets.keys()) {
            if let Some(prev) = lower.insert(p.to_lowercase(), p.clone()) {
                if &prev != p {
                    plan.report.push(Level::Warning, "", p, "", format!("differs from '{prev}' only by letter case"));
                }
            }
        }
    }

    for (target, providers) in &record_targets {
        let in_vanilla = vanilla.contains(target);
        let lang = i18n_language(target).map(str::to_string);
        let multi = providers.iter().map(|(i, _)| i).collect::<HashSet<_>>().len() > 1;
        let replaced_by: Vec<usize> =
            providers.iter().filter(|(i, rel)| loaded[*i].manifest.replace.iter().any(|r| r == rel)).map(|(i, _)| *i).collect();

        if !in_vanilla && !multi && lang.is_none() {
            // a new file only one mod provides: write it as-is (namespaced, directives removed)
            let (i, rel) = &providers[0];
            let mut recs = loaded[*i].records[rel].clone();
            strip_all_directives(&mut recs);
            let (crlf, latin1) = loaded[*i].formats[rel];
            plan.files.push(OutFile {
                rel: target.clone(),
                contents: Contents::Bytes(Document { records: recs, crlf, latin1 }.to_bytes()),
                how: "record".into(),
                sources: vec![loaded[*i].manifest.id.clone()],
            });
            continue;
        }

        // base: vanilla (for i18n: every vanilla file of that language)
        let mut base: Vec<Record> = Vec::new();
        let mut fmt = (false, false);
        if let Some(lang) = &lang {
            let prefix = format!("i18n/{lang}/");
            for p in vanilla.paths().into_iter().filter(|p| p.starts_with(&prefix) && i18n_language(p).is_some()) {
                if let Some(b) = vanilla.read(&p)? {
                    let d = record::parse(&b).map_err(|e| anyhow::anyhow!("vanilla {p}: {e}"))?;
                    fmt = (d.crlf, d.latin1);
                    base.extend(d.records);
                }
            }
        } else if in_vanilla {
            let b = vanilla.read(target)?.unwrap_or_default();
            let d = record::parse(&b).map_err(|e| anyhow::anyhow!("vanilla {target}: {e}"))?;
            fmt = (d.crlf, d.latin1);
            base = d.records;
        }

        let mut sources = Vec::new();
        {
            let mut merger = Merger::new(target, &mut plan.report);
            for (i, rel) in providers {
                let mf = &loaded[*i];
                let ns = &mf.manifest.id;
                if replaced_by.contains(i) {
                    base = mf.records[rel].clone();
                    strip_all_directives(&mut base);
                    merger.report.push(Level::Info, ns, target, "", "replaces the whole file (manifest 'replace')");
                } else {
                    merger.apply(&mut base, &mf.records[rel], ns);
                }
                if !sources.contains(ns) {
                    sources.push(ns.clone());
                }
                if !in_vanilla && lang.is_none() {
                    fmt = mf.formats[rel];
                }
            }
        }
        strip_all_directives(&mut base);
        plan.files.push(OutFile {
            rel: target.clone(),
            contents: Contents::Bytes(Document { records: base, crlf: fmt.0, latin1: fmt.1 }.to_bytes()),
            how: "merged".into(),
            sources,
        });
    }

    for (rel, providers) in &opaque_targets {
        if record_targets.contains_key(rel) {
            plan.report.push(Level::Warning, "", rel, "", "provided both as text and as an unparsable file; the text version is used");
            continue;
        }
        let winner = *providers.last().unwrap();
        let wns = loaded[winner].manifest.id.clone();
        if providers.len() > 1 {
            let losers: Vec<String> = providers[..providers.len() - 1].iter().map(|i| loaded[*i].manifest.id.clone()).collect();
            plan.report.push(Level::Conflict, &wns, rel, "", format!("file also provided by {}; '{wns}' wins (load order)", losers.join(", ")));
        }
        let contents = match loaded[winner].filled.get(rel) {
            Some(b) => Contents::Bytes(b.clone()),
            None => Contents::CopyOf(loaded[winner].opaque[rel].clone()),
        };
        plan.files.push(OutFile {
            rel: rel.clone(),
            contents,
            how: if vanilla.contains(rel) { "replaced".into() } else { "copied".into() },
            sources: providers.iter().map(|i| loaded[*i].manifest.id.clone()).collect(),
        });
    }
    plan.files.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(plan)
}
