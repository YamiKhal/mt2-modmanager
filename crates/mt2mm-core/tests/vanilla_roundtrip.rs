use mt2mm_core::record;

const TEXT_EXTS: &[&str] = &[
    "txt", "cfg", "win", "def", "vrt", "costume", "variant", "conf", "defaults", "mat", "block",
    "prototype", "adv", "sequence", "vsprite", "tcolor", "tshape", "tstyle",
];


#[test]
fn roundtrip_all_vanilla_text_files() {
    let Ok(root) = std::env::var("MT2_GAMEDATA") else {
        eprintln!("MT2_GAMEDATA not set; skipping");
        return;
    };
    let mut ok = 0;
    let mut failures = Vec::new();
    for e in walkdir::WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        let p = e.path();
        let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase();
        let no_ext = p.extension().is_none() && p.is_file();
        if !(TEXT_EXTS.contains(&ext.as_str()) || no_ext) || !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(p).unwrap();
        match record::parse(&bytes) {
            Err(err) => failures.push(format!("{}: parse error: {err}", p.display())),
            Ok(doc) => {
                let out = doc.to_bytes();
                let again = record::parse(&out).unwrap();
                if again.records != doc.records {
                    failures.push(format!("{}: tree changed after rewrite", p.display()));
                } else {
                    ok += 1;
                }
            }
        }
    }
    eprintln!("{ok} files round-tripped");
    for f in &failures {
        eprintln!("{f}");
    }
    assert!(failures.is_empty(), "{} failures", failures.len());
}
