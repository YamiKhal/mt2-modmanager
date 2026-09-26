use crate::merge::{Level, Report};
use crate::record::Record;
use crate::util::extension;
use std::collections::{BTreeMap, HashSet};


pub fn available_files<'a>(vanilla_paths: Vec<String>, mod_files: impl Iterator<Item = &'a String>) -> HashSet<String> {
    let mut files: HashSet<String> = vanilla_paths.into_iter().map(|p| p.to_lowercase()).collect();
    files.extend(mod_files.map(|p| p.to_lowercase()));

    files
}

// The game opens a material's texture when it loads the material, and a file it can't open fails an assertion.
pub fn check_mod_textures(ns: &str, records: &BTreeMap<String, Vec<Record>>, files: &HashSet<String>, report: &mut Report) {
    for (rel, recs) in records.iter().filter(|(rel, _)| extension(rel) == "mat") {
        for texture in recs.iter().filter_map(|r| r.prop("texture")) {
            let wanted = texture.replace('\\', "/").to_lowercase();
            if !files.contains(&wanted) {
                report.push(
                    Level::Error,
                    ns,
                    rel,
                    "",
                    format!("uses the texture '{texture}', but no enabled mod or the game has it; the game most likely closes when it loads this material"),
                );
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::parse;

    fn check(texture: &str, files: &[&str]) -> usize {
        let text = format!("Material {{\n\tmode lit\n\ttexture \"{texture}\"\n\tshader \"tint_v.glsl\" \"tint_f.glsl\"\n}}\n");
        let records = BTreeMap::from([("materials/mymod_crate.mat".to_string(), parse(text.as_bytes()).unwrap().records)]);
        let files = available_files(files.iter().map(|f| f.to_string()).collect(), std::iter::empty());
        let mut report = Report::default();
        check_mod_textures("mymod", &records, &files, &mut report);

        report.events.len()
    }

    #[test]
    fn missing_texture_is_an_error() {
        assert_eq!(check("textures/mymod_crate.png", &["Clouds.png"]), 1);
    }

    #[test]
    fn texture_found_in_any_case() {
        assert_eq!(check("textures/MyMod_Crate.PNG", &["textures/mymod_crate.png"]), 0);
        assert_eq!(check("Clouds.png", &["Clouds.png"]), 0);
    }
}
