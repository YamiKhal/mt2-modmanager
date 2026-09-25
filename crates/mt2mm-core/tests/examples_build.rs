use mt2mm_core::build;
use mt2mm_core::library::Library;
use mt2mm_core::merge::Level;
use mt2mm_core::vanilla::Vanilla;
use std::path::PathBuf;


#[test]
fn example_mods_build_cleanly() {
    let Ok(data) = std::env::var("MT2_GAMEDATA") else {
        eprintln!("MT2_GAMEDATA not set; skipping");
        return;
    };
    let tmp = std::env::temp_dir().join(format!("mt2mm-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let lib = Library::new(&tmp);
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    for m in ["toolbox", "crunch_techs", "cash_plus", "milestones"] {
        lib.import(&examples.join(m), false).unwrap();
    }
    let changes = [("megainns_price".to_string(), serde_json::json!(55000))].into_iter().collect();
    lib.set_settings("crunch", &changes, &[]).unwrap();
    let vanilla = Vanilla::open(&PathBuf::from(data)).unwrap();
    let plan = build::plan(&vanilla, &lib.list().unwrap(), None).unwrap();
    let _ = std::fs::remove_dir_all(&tmp);

    assert_eq!(plan.report.count(Level::Error), 0, "{:#?}", plan.report);
    assert_eq!(plan.report.count(Level::Conflict), 0, "{:#?}", plan.report);
    let file = |rel: &str| String::from_utf8(plan.files.iter().find(|f| f.rel == rel).unwrap().read().unwrap()).unwrap();

    let techs = file("techs/crunch.vrt");
    assert!(techs.contains("id \"crunch_crunchtime2\""));
    assert!(techs.contains("prerequisite \"crunch_crunchtime\""));
    assert!(techs.contains("prerequisite \"gamelog\""));
    assert!(techs.contains("cost 55000;"), "changed setting");
    assert!(techs.contains("arg 1.25;") && techs.contains("arg 2;") && techs.contains("playerVisible true;"), "defaults");
    assert!(!techs.contains("<crunch_speed>;"));
    assert_eq!(plan.report.count(Level::Warning), 0, "{:#?}", plan.report);

    let bars = file("actionbars.win");
    assert!(bars.contains("id \"toolbox_Mod\""));
    assert!(bars.contains("itemName \"@cashplus_AddMillion\""));
    assert_eq!(bars.matches("id \"toolbox_Mod\"").count(), 1, "cross-mod patch must merge into the same content");

    let inv = file("EditInventory.cfg");
    assert!(!inv.contains("\"@Finance\""));
    assert!(inv.contains("\"@toolbox_AddCash\""));

    let i18n = file("i18n/english/00-base.vrt");
    assert!(i18n.contains("tech_crunch_megainns_displayname"));
    assert!(i18n.contains("toolbox_actionbar_tab_mod \"Mod\""));
    assert!(i18n.contains("tech_friendslist_displayname"), "vanilla strings must be kept");

    let library = file("scenarios/library.txt");
    assert!(library.contains("id \"milestones_inns\""));
    assert!(library.contains("id \"milestones_landmarks\""));
    assert!(library.contains("id \"streamer\""), "vanilla scenarios must be kept");

    let inns = file("scenarios/milestones_inns");
    assert!(inns.contains("timesNeeded 3;"));
    assert!(inns.contains("command \"add_cash 25000\";"));
    assert!(i18n.contains("{button_hl}$25000{clear_hl}"));
}
