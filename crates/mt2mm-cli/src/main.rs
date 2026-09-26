use anyhow::{bail, Context, Result};
use mt2mm_core::build::BuildPlan;
use mt2mm_core::merge::Level;
use mt2mm_core::session::{Config, Session};
use std::path::PathBuf;

const HELP: &str = "\
mt2mm - MMORPG Tycoon 2 mod manager

USAGE: mt2mm [--profile DIR] [--data ZIP|DIR] [--install DIR] [--json] <command> [args]

MODS
  status                   detected folders, mod profiles, library and load order
  add <folder|zip>         add one mod (copies it; --move moves a folder). Same id = update.
  add-all <folder>         add every mod (sub-folder or .zip with a manifest.json) in <folder>
  remove <id>              delete a mod from the library and from every profile

DEV MODS (a mod you're making, kept in its own folder; the library holds a copy)
  dev add <folder>         add it and remember the folder
  dev refresh [<id>]       copy the folder into the library again (all dev mods when no id is given)
  dev stop <id>            forget the folder; the library copy stays

MOD SETTINGS (declared by the mod's config.json)
  settings <id>                    list the mod's settings and their values
  settings <id> set <key> <value>  change one (run deploy afterwards)
  settings <id> reset [<key> ...]  back to the default (all keys when none are given)

ACTIVE MOD PROFILE
  apply <id> [<id> ...]    add mods to the profile's applied list (enabled)
  unapply <id> [<id> ...]  take mods out of the profile (they stay in the library)
  enable <id> | disable <id>
  order <id> [<id> ...]    set load order (lowest priority first; unlisted mods keep their order after)

MOD PROFILES
  profile                  list profiles
  profile new <name> [--copy]   create a profile (--copy: start from the active one) and switch to it
  profile use <name>       switch to a profile (run deploy afterwards)
  profile rename <old> <new>
  profile delete <name>

BUILD
  check                    build in memory and print the report (changes nothing)
  export <dir>             build and write the result to <dir> for inspection
  deploy                   build and install into the game's mod folder
  launch                   check; deploy only if the game doesn't have the mods yet; start through Steam
  clean                    remove everything deploy installed
  config                   save --profile/--data/--install as defaults

--json prints machine-readable output for status and check.
Mods in the game's mod folder that were not deployed by the manager are left alone.
Save files are never read or written.
";


fn print_report(plan: &BuildPlan) {
    println!("Load order: {}", if plan.mods.is_empty() { "(no enabled mods)".into() } else { plan.mods.join(" -> ") });
    if !plan.renames.is_empty() {
        println!("\nRenamed ids:");
        for r in &plan.renames {
            println!("  [{}] {} {:?} -> {:?}", r.mod_id, r.kind, r.from, r.to);
        }
    }
    println!("\nFiles ({}):", plan.files.len());
    for f in &plan.files {
        println!("  {:<8} {}  <- {}", f.how, f.rel, f.sources.join(", "));
    }
    let evs = &plan.report.events;
    if !evs.is_empty() {
        println!("\nReport:");
        for e in evs {
            let lvl = match e.level {
                Level::Info => "info",
                Level::Warning => "WARN",
                Level::Conflict => "CONFLICT",
                Level::Error => "ERROR",
            };
            let loc = [e.file.as_str(), e.path.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
            let who = if e.mod_id.is_empty() { String::new() } else { format!("[{}] ", e.mod_id) };
            println!("  {lvl:<8} {who}{loc}: {}", e.message);
        }
    }
    println!(
        "\n{} error(s), {} conflict(s), {} warning(s)",
        plan.report.count(Level::Error),
        plan.report.count(Level::Conflict),
        plan.report.count(Level::Warning)
    );
}


fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut ov = Config::default();
    let take = |flag: &str, args: &mut Vec<String>| -> Result<Option<PathBuf>> {
        if let Some(i) = args.iter().position(|a| a == flag) {
            if i + 1 >= args.len() {
                bail!("{flag} needs a value");
            }
            let v = args.remove(i + 1);
            args.remove(i);
            return Ok(Some(PathBuf::from(v)));
        }
        Ok(None)
    };
    ov.profile_dir = take("--profile", &mut args)?;
    ov.data = take("--data", &mut args)?;
    ov.install_dir = take("--install", &mut args)?;
    let json = if let Some(i) = args.iter().position(|a| a == "--json") {
        args.remove(i);
        true
    } else {
        false
    };
    let move_folder = if let Some(i) = args.iter().position(|a| a == "--move") {
        args.remove(i);
        true
    } else {
        false
    };

    let Some(cmd) = args.first().cloned() else {
        print!("{HELP}");
        return Ok(());
    };
    let rest = &args[1..];
    let s = Session::open(&ov);

    match cmd.as_str() {
        "help" | "-h" | "--help" => print!("{HELP}"),
        "status" if json => println!("{}", serde_json::to_string_pretty(&s.status())?),
        "check" if json => {
            let plan = s.plan()?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
            if !plan.ok() {
                std::process::exit(1);
            }
        }
        "status" => {
            let st = s.status();
            let show = |p: &Option<PathBuf>| p.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(not found)".into());
            println!("Install:   {}", show(&st.paths.install_dir));
            println!("Data:      {}", show(&st.paths.data_zip));
            println!("Profile:   {}", show(&st.paths.profile_dir));
            println!("Game:      {}", st.game_version.as_deref().unwrap_or("(unknown; run the game once)"));
            println!("Mod profile: {} (of {})", st.active_profile, st.profiles.join(", "));
            for p in &st.problems {
                println!("PROBLEM:   {p}");
            }
            println!("\nApplied mods (load order, lowest priority first), then the rest of the library:");
            if st.mods.is_empty() {
                println!("  (empty; use `mt2mm add <folder|zip>`)");
            }
            for (i, m) in st.mods.iter().enumerate() {
                match &m.manifest {
                    Some(man) if m.applied => println!(
                        "  {:>2}. [{}] {:<16} {} v{}",
                        i + 1,
                        if m.enabled { "x" } else { " " },
                        man.id,
                        man.name,
                        man.version
                    ),
                    Some(man) => println!("    -      {:<16} {} v{} (not applied)", man.id, man.name, man.version),
                    None => println!("   -. [!] {:<16} {}", m.folder, m.error.clone().unwrap_or_default()),
                }
            }
            if !st.unmanaged.is_empty() {
                println!("\nNot managed (in mod/, still loaded by the game):");
                for u in &st.unmanaged {
                    println!("  {}", u.file_name().unwrap().to_string_lossy());
                }
            }
            println!(
                "\nDeployed: {}",
                match st.deploy_state.as_str() {
                    "none" => "nothing",
                    "current" => "up to date",
                    _ => "out of date (run `mt2mm deploy`)",
                }
            );
        }
        "add" | "import" => {
            let src = rest.first().context("add needs a folder or .zip")?;
            let r = s.library()?.import(&PathBuf::from(src), move_folder)?;
            let id = r.id.unwrap_or_default();
            println!("{} '{id}'. Run `mt2mm deploy` to apply.", if r.updated { "Updated" } else { "Added" });
        }
        "add-all" => {
            let dir = rest.first().context("add-all needs a folder")?;
            let results = s.library()?.import_all(&PathBuf::from(dir))?;
            if results.is_empty() {
                println!("No mods found (a mod is a folder or .zip with a manifest.json).");
            }
            for r in results {
                match (r.id, r.error) {
                    (Some(id), _) => println!("  {} {id}", if r.updated { "updated" } else { "added  " }),
                    (None, e) => println!("  FAILED  {}: {}", r.source.display(), e.unwrap_or_default()),
                }
            }
        }
        "dev" => {
            let lib = s.library()?;
            match (rest.first().map(String::as_str), rest.get(1)) {
                (Some("add"), Some(src)) => {
                    let r = lib.import_dev(&PathBuf::from(src))?;
                    println!("{} dev mod '{}'.", if r.updated { "Updated" } else { "Added" }, r.id.unwrap_or_default());
                }
                (Some("refresh"), Some(id)) => {
                    lib.refresh_dev(id)?;
                    println!("Refreshed {id}.");
                }
                (Some("refresh"), None) => {
                    for (id, r) in lib.refresh_all_dev() {
                        match r {
                            Ok(_) => println!("  refreshed {id}"),
                            Err(e) => println!("  FAILED    {id}: {e:#}"),
                        }
                    }
                }
                (Some("stop"), Some(id)) => {
                    lib.stop_dev(id)?;
                    println!("{id} is no longer a dev mod.");
                }
                _ => bail!("usage: dev add <folder> | dev refresh [<id>] | dev stop <id>"),
            }
        }
        "apply" => {
            s.library()?.apply(rest)?;
            println!("Applied {}.", rest.join(", "));
        }
        "unapply" => {
            s.library()?.unapply(rest)?;
            println!("Unapplied {}.", rest.join(", "));
        }
        "profile" => {
            let lib = s.library()?;
            match rest.first().map(String::as_str) {
                None | Some("list") => {
                    let st = lib.load_state();
                    for (name, p) in &st.profiles {
                        let enabled = p.mods.iter().filter(|e| e.enabled).count();
                        println!("{} {name}  ({enabled} of {} applied mods enabled)", if *name == st.active { "*" } else { " " }, p.mods.len());
                    }
                }
                Some("new") => {
                    let name = rest.get(1).context("profile new needs a name")?;
                    let copy = rest.iter().any(|a| a == "--copy");
                    let from = lib.load_state().active;
                    lib.create_profile(name, copy.then_some(from.as_str()))?;
                    println!("Created and switched to '{name}'.");
                }
                Some("use") => {
                    let name = rest.get(1).context("profile use needs a name")?;
                    lib.switch_profile(name)?;
                    println!("Switched to '{name}'. Run `mt2mm deploy` to apply it to the game.");
                }
                Some("rename") => {
                    let (a, b) = (rest.get(1).context("needs <old> <new>")?, rest.get(2).context("needs <old> <new>")?);
                    lib.rename_profile(a, b)?;
                    println!("Renamed '{a}' to '{b}'.");
                }
                Some("delete") => {
                    let name = rest.get(1).context("profile delete needs a name")?;
                    lib.delete_profile(name)?;
                    println!("Deleted '{name}'.");
                }
                Some(o) => bail!("unknown profile command '{o}'"),
            }
        }
        "enable" | "disable" => {
            let ns = rest.first().context("needs a mod id")?;
            s.library()?.set_enabled(ns, cmd == "enable")?;
            println!("{ns} {}d", cmd);
        }
        "order" => {
            s.library()?.set_order(rest)?;
            let st = s.library()?.load_state();
            println!("Load order: {}", st.active_profile().mods.iter().map(|e| e.id.as_str()).collect::<Vec<_>>().join(" -> "));
        }
        "remove" => {
            let ns = rest.first().context("needs a mod id")?;
            s.library()?.remove(ns)?;
            println!("Removed {ns} from the library.");
        }
        "settings" => {
            let lib = s.library()?;
            let id = rest.first().context("settings needs a mod id")?;
            let find = || -> Result<mt2mm_core::library::LibraryMod> {
                lib.list()?.into_iter().find(|m| m.id() == Some(id)).with_context(|| format!("no mod '{id}' in the library"))
            };
            match rest.get(1).map(String::as_str) {
                None => {
                    let m = find()?;
                    if let Some(e) = &m.config_error {
                        bail!("{e}");
                    }
                    if m.config.is_empty() {
                        println!("'{id}' has no settings.");
                    }
                    for o in &m.config {
                        let v = &m.settings.values[&o.key];
                        let mark = if m.settings.changed.contains(&o.key) { "*" } else { " " };
                        println!("{mark} {:<24} {:<10} {}  (default {}){}", o.key, o.render(v), o.label, o.render(&o.default),
                            if o.unit.is_empty() { String::new() } else { format!(" [{}]", o.unit) });
                        if !o.options.is_empty() {
                            println!("    options: {}", o.options.iter().map(|c| c.value.as_str()).collect::<Vec<_>>().join(", "));
                        }
                    }
                    for n in &m.settings.notes {
                        println!("NOTE: {n}");
                    }
                }
                Some("set") => {
                    let (k, v) = (rest.get(2).context("needs <key> <value>")?, rest.get(3).context("needs <key> <value>")?);
                    let val = serde_json::from_str(v).unwrap_or_else(|_| serde_json::Value::String(v.clone()));
                    lib.set_settings(id, &[(k.clone(), val)].into_iter().collect(), &[])?;
                    let m = find()?;
                    let o = m.config.iter().find(|o| &o.key == k).unwrap();
                    println!("{k} = {}. Run `mt2mm deploy` to apply.", o.render(&m.settings.values[k]));
                }
                Some("reset") => {
                    let keys: Vec<String> =
                        if rest.len() > 2 { rest[2..].to_vec() } else { find()?.config.iter().map(|o| o.key.clone()).collect() };
                    lib.set_settings(id, &Default::default(), &keys)?;
                    println!("Reset {}. Run `mt2mm deploy` to apply.", keys.join(", "));
                }
                Some(o) => bail!("unknown settings command '{o}'"),
            }
        }
        "check" => {
            let plan = s.plan()?;
            print_report(&plan);
            if !plan.ok() {
                std::process::exit(1);
            }
        }
        "export" => {
            let dest = PathBuf::from(rest.first().context("export needs a folder")?);
            let plan = s.plan()?;
            print_report(&plan);
            mt2mm_core::deploy::export(&plan, &dest)?;
            println!("\nWritten to {}", dest.display());
        }
        "deploy" => {
            let plan = s.plan()?;
            print_report(&plan);
            if !plan.ok() {
                bail!("not deployed because of errors");
            }
            s.install(&plan)?;
            println!("\nDeployed to {}", s.paths.mod_dir().unwrap().display());
        }
        "launch" => {
            let (plan, applied) = s.launch_checked()?;
            if applied {
                print_report(&plan);
                println!("\nDeployed to {}", s.paths.mod_dir().unwrap().display());
            } else {
                println!("Mods already applied, no errors.");
            }
            println!("Starting the game through Steam.");
        }
        "clean" => {
            let removed = s.clean()?;
            println!("Removed {} managed folder(s).", removed.len());
        }
        "config" => {
            let mut c = Config::load();
            if ov.profile_dir.is_some() {
                c.profile_dir = ov.profile_dir;
            }
            if ov.data.is_some() {
                c.data = ov.data;
            }
            if ov.install_dir.is_some() {
                c.install_dir = ov.install_dir;
            }
            c.save()?;
            println!("Saved to {}", Config::path().unwrap().display());
        }
        other => bail!("unknown command '{other}' (try `mt2mm help`)"),
    }
    Ok(())
}
