use crate::merge::{Level, Report};
use crate::util::extension;
use crate::vanilla::Vanilla;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct ModelSummary {
    pub materials: BTreeSet<String>,
    pub problems: Vec<String>,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

const MAX_VERTICES: i32 = 65535;
const MAX_COUNT: i32 = 50_000_000;
const MAX_DEPTH: usize = 64;

const SCENERY_TYPES: &[&str] = &[
    "tree", "tree_pine", "tree_palm", "tree_snow", "tree_swamp", "tree_redwood", "tree_savannah", "tree_feature",
    "cactus", "cactus_spiked", "cactus_feature", "bamboo", "tree_jungle", "sandstone", "tree_dead", "mushroom_giant",
    "mushroom", "crystal", "spike", "stone", "stone_large", "reed", "plant_small", "plant_medium", "cart", "chair",
    "post", "shelf", "table", "tower", "warfare", "water", "ice", "desert", "structure", "prop", "light",
    "wall_decorations", "festival", "xeno", "western", "confection", "ruin", "banner_stand", "eldritch", "volcanic",
    "tagged",
];


impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.data.len()).ok_or("the file ends early")?;
        let bytes = &self.data[self.pos..end];
        self.pos = end;

        Ok(bytes)
    }

    fn int16(&mut self) -> Result<i16, String> {
        let b = self.take(2)?;

        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    fn int32(&mut self) -> Result<i32, String> {
        let b = self.take(4)?;

        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn count(&mut self) -> Result<i32, String> {
        let n = self.int32()?;
        if !(0..=MAX_COUNT).contains(&n) {
            return Err(format!("impossible count {n}"));
        }

        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        let n = self.int16()?;
        if n < 0 {
            return Err("negative string length".into());
        }

        Ok(self.take(n as usize)?.iter().map(|&b| b as char).collect())
    }
}


fn vertex_size(format: &str) -> Option<usize> {
    let size = match format {
        "P" => 3,
        "PC" => 7,
        "PN" => 6,
        "PT" => 5,
        "PCN" => 10,
        "PCT" => 9,
        "PNT" => 8,
        "PCNT" => 12,
        _ => return None,
    };

    Some(size)
}

fn read_fragment(r: &mut Reader, summary: &mut ModelSummary) -> Result<(), String> {
    if r.string()? != "Fragment" {
        return Err("expected a fragment".into());
    }
    let material = r.string()?;
    let format = r.string()?;
    let size = vertex_size(&format).ok_or_else(|| format!("unknown vertex format '{format}'"))?;
    let vertices = r.count()?;
    if vertices > MAX_VERTICES {
        summary.problems.push(format!("a '{material}' fragment has {vertices} vertices; the game reads at most {MAX_VERTICES}"));
    }
    r.take(vertices as usize * size * 4)?;
    if r.string()? != "IndexBuffer" {
        return Err("expected an index buffer".into());
    }
    let indices = r.count()?;
    let mut out_of_range = false;
    for _ in 0..indices {
        let index = r.int32()?;
        out_of_range |= index < 0 || index >= vertices;
    }
    if out_of_range {
        summary.problems.push(format!("a '{material}' fragment has triangles pointing at vertices that don't exist"));
    }
    summary.materials.insert(material);

    Ok(())
}

fn read_node(r: &mut Reader, summary: &mut ModelSummary, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err("nodes nested too deeply".into());
    }
    let has_lods = match r.string()?.as_str() {
        "ModelV1" => false,
        "ModelV2" => true,
        _ => return Err("not a model file".into()),
    };
    r.string()?;
    r.take(10 * 4)?;
    let lod_count = if has_lods { r.count()? } else { 1 };
    for _ in 0..lod_count {
        for _ in 0..r.count()? {
            read_fragment(r, summary)?;
        }
    }
    for _ in 0..r.count()? {
        read_node(r, summary, depth + 1)?;
    }

    Ok(())
}

pub fn inspect(bytes: &[u8]) -> Result<ModelSummary, String> {
    let mut reader = Reader { data: bytes, pos: 0 };
    let mut summary = ModelSummary::default();
    read_node(&mut reader, &mut summary, 0)?;

    Ok(summary)
}


fn material_name(rel: &str) -> Option<&str> {
    let name = rel.strip_prefix("materials/")?.strip_suffix(".mat")?;

    (!name.contains('/')).then_some(name)
}

pub fn available_materials<'a>(vanilla: &Vanilla, mod_files: impl Iterator<Item = &'a String>) -> HashSet<String> {
    let mut names: HashSet<String> = vanilla.paths().iter().filter_map(|p| material_name(p)).map(String::from).collect();
    names.extend(mod_files.filter_map(|p| material_name(p)).map(String::from));

    names
}

fn placement_problem(rel: &str) -> Option<String> {
    let parts: Vec<&str> = rel.split('/').collect();
    match parts.as_slice() {
        ["scenery", kind, _] if !SCENERY_TYPES.contains(kind) => Some(format!(
            "'{kind}' is not a scenery type; the game only loads scenery from its 47 type folders, so this model never appears"
        )),
        ["weapons", _, file] if !has_level_prefix(file) => {
            Some("weapon files need an item-level prefix such as 045_; the game skips this one".into())
        }
        _ => None,
    }
}

fn has_level_prefix(file: &str) -> bool {
    let digits = file.bytes().take_while(u8::is_ascii_digit).count();

    digits > 0 && file.as_bytes().get(digits) == Some(&b'_')
}

pub fn check_mod_models(
    ns: &str,
    files: &BTreeMap<String, PathBuf>,
    materials: &HashSet<String>,
    report: &mut Report,
) -> Result<()> {
    for (rel, path) in files.iter().filter(|(rel, _)| extension(rel) == "vmb") {
        match inspect(&std::fs::read(path)?) {
            Err(problem) => {
                report.push(Level::Error, ns, rel, "", format!("not a valid model ({problem}); the game would fail to load it"));
            }
            Ok(summary) => {
                for problem in summary.problems {
                    report.push(Level::Error, ns, rel, "", problem);
                }
                for missing in summary.materials.iter().filter(|m| !materials.contains(*m)) {
                    report.push(
                        Level::Error,
                        ns,
                        rel,
                        "",
                        format!("uses material '{missing}', but no enabled mod or the game has materials/{missing}.mat; the game closes when it loads this model"),
                    );
                }
            }
        }
        if let Some(problem) = placement_problem(rel) {
            report.push(Level::Warning, ns, rel, "", problem);
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn string(out: &mut Vec<u8>, s: &str) {
        out.extend((s.len() as i16).to_be_bytes());
        out.extend(s.as_bytes());
    }

    fn model(material: &str, indices: &[i32]) -> Vec<u8> {
        let mut out = Vec::new();
        string(&mut out, "ModelV1");
        string(&mut out, "RootNode");
        out.extend([0u8; 40]);
        out.extend(1i32.to_be_bytes());
        string(&mut out, "Fragment");
        string(&mut out, material);
        string(&mut out, "P");
        out.extend(3i32.to_be_bytes());
        out.extend([0u8; 3 * 3 * 4]);
        string(&mut out, "IndexBuffer");
        out.extend((indices.len() as i32).to_be_bytes());
        for i in indices {
            out.extend(i.to_be_bytes());
        }
        out.extend(0i32.to_be_bytes());

        out
    }

    #[test]
    fn reads_materials() {
        let summary = inspect(&model("Material_tint", &[0, 1, 2])).unwrap();

        assert_eq!(summary.materials.into_iter().collect::<Vec<_>>(), vec!["Material_tint".to_string()]);
        assert!(summary.problems.is_empty());
    }

    #[test]
    fn reports_bad_indices() {
        let summary = inspect(&model("Material_tint", &[0, 1, 7])).unwrap();

        assert_eq!(summary.problems.len(), 1);
    }

    #[test]
    fn rejects_garbage_and_truncation() {
        assert!(inspect(b"not a model").is_err());
        let bytes = model("Material_tint", &[0, 1, 2]);
        assert!(inspect(&bytes[..bytes.len() - 10]).is_err());
    }

    #[test]
    fn placement_rules() {
        assert!(placement_problem("scenery/rocks/a.vmb").is_some());
        assert!(placement_problem("scenery/stone/a.vmb").is_none());
        assert!(placement_problem("weapons/swords/sword.vmb").is_some());
        assert!(placement_problem("weapons/swords/045_sword.vmb").is_none());
    }

    #[test]
    fn missing_material_is_an_error() {
        let dir = std::env::temp_dir().join(format!("mt2mm_models_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.vmb");
        std::fs::write(&path, model("Nope", &[0, 1, 2])).unwrap();
        let files = BTreeMap::from([("scenery/stone/a.vmb".to_string(), path)]);
        let materials = HashSet::from(["Material_tint".to_string()]);
        let mut report = Report::default();
        check_mod_models("m", &files, &materials, &mut report).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(report.count(Level::Error), 1);
    }

    #[test]
    fn every_vanilla_model_passes() {
        let Ok(root) = std::env::var("MT2_GAMEDATA") else { return };
        let vanilla = Vanilla::open(std::path::Path::new(&root)).unwrap();
        let materials = available_materials(&vanilla, std::iter::empty());
        for rel in vanilla.paths().iter().filter(|p| p.ends_with(".vmb")) {
            let summary = inspect(&vanilla.read(rel).unwrap().unwrap()).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(summary.problems.is_empty(), "{rel}: {:?}", summary.problems);
            for m in &summary.materials {
                assert!(materials.contains(m), "{rel} uses missing material {m}");
            }
        }
    }
}
