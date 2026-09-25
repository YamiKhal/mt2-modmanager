use crate::merge::{Level, Report, KEY_FIELDS};
use crate::record::{self, Record};
use crate::vanilla::Vanilla;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub level: Level,
    pub path: String,
    pub message: String,
}

const CRASH_COMMANDS: &[&str] = &["crash", "backgroundcrash", "assert"];
const SAVE_COMMANDS: &[&str] = &["Save", "ConfirmSave", "Autosave", "ImmediateAutosave"];
const ACHIEVEMENT_COMMANDS: &[&str] = &["GrantAchievement", "RevokeAchievement", "RevokeAchievementConfirm"];


pub fn check_mod_file(ns: &str, rel: &str, records: &[Record], vanilla: &Vanilla, report: &mut Report) -> Result<()> {
    let findings = scan(records);

    if findings.is_empty() {
        return Ok(());
    }

    // A mod that ships a full copy of a vanilla file also ships vanilla's own commands,
    // such as the Discord link in gamemenu.win. Only what the mod adds is reported.
    let vanilla_findings = match vanilla.read(rel)? {
        Some(bytes) => record::parse(&bytes).map(|doc| scan(&doc.records)).unwrap_or_default(),
        None => Vec::new(),
    };

    for finding in findings {
        let in_vanilla = vanilla_findings.iter().any(|known| known.message == finding.message);

        if !in_vanilla {
            report.push(finding.level, ns, rel, &finding.path, finding.message);
        }
    }

    Ok(())
}

pub fn scan(records: &[Record]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for record in records {
        scan_record(record, None, "", &mut findings);
    }

    findings
}


fn scan_record(record: &Record, parent_label: Option<&str>, parent_path: &str, findings: &mut Vec<Finding>) {
    let label = record.label.as_deref().unwrap_or("");
    let path = if parent_path.is_empty() { describe(record) } else { format!("{parent_path}/{}", describe(record)) };

    match label {
        "command" => {
            for command in record.first_text().unwrap_or("").split("&&") {
                if let Some((level, message)) = check_command(command) {
                    findings.push(Finding { level, path: path.clone(), message });
                }
            }
        }
        "mmoStepSave" => {
            let save_name = record.prop("saveName").unwrap_or("");
            let message = format!(
                "scenario step writes a save named '{save_name}'; a save with that name in the current MMO is overwritten"
            );

            findings.push(Finding { level: Level::Warning, path: path.clone(), message });
        }
        "mmoPredicate_Event" => {
            let message = "mmoPredicate_Event can't be configured from data files; the game shows a failed assertion \
                           and closes while loading it. Use a Subscribers, Version or Level trigger instead";

            findings.push(Finding { level: Level::Error, path: path.clone(), message: message.into() });
        }
        "prerequisites" if parent_label.is_some_and(|parent| parent.starts_with("mmoScenario")) => {
            let message = "the game never checks scenario prerequisites; this scenario starts whenever its trigger is true";

            findings.push(Finding { level: Level::Warning, path: path.clone(), message: message.into() });
        }
        "errorJump" => {
            let message = "errorJump is not a field the game knows; it is ignored";

            findings.push(Finding { level: Level::Info, path: path.clone(), message: message.into() });
        }
        _ => {}
    }

    for child in &record.children {
        scan_record(child, Some(label), &path, findings);
    }
}

fn describe(record: &Record) -> String {
    let label = record.label.clone().unwrap_or_default();

    for key in KEY_FIELDS {
        if let Some(value) = record.prop(key) {
            return format!("{label}[{key}={value}]");
        }
    }

    label
}


fn check_command(command: &str) -> Option<(Level, String)> {
    let words = split_words(command);
    let word = |index: usize| words.get(index).map(String::as_str).unwrap_or("");
    let shown = command.trim();

    if CRASH_COMMANDS.contains(&word(0)) {
        return Some((Level::Warning, format!("'{shown}' makes the game close")));
    }

    if word(0) != "MessageTo" {
        return None;
    }

    let finding = match (word(1), word(2)) {
        ("Game", "OpenURL") => {
            let target = word(3);
            let lower = target.to_ascii_lowercase();

            if lower.starts_with("https://") || lower.starts_with("http://") {
                (Level::Info, format!("opens {target} in your browser"))
            } else {
                (
                    Level::Error,
                    format!(
                        "'{shown}' hands '{target}' to the Windows shell, which can start any program or open any file. \
                         Only web links (http/https) are allowed"
                    ),
                )
            }
        }
        ("Mode", "ConfirmDelete") => (Level::Error, format!("'{shown}' deletes a save file without asking you")),
        ("Mode", "Delete") => (Level::Warning, format!("'{shown}' opens the delete-save dialog")),
        ("Mode", action) if SAVE_COMMANDS.contains(&action) => (Level::Warning, format!("'{shown}' writes a save")),
        ("Game", action) if ACHIEVEMENT_COMMANDS.contains(&action) => {
            (Level::Warning, format!("'{shown}' changes your Steam achievements"))
        }
        ("Core", _) => (Level::Warning, format!("'{shown}' switches the engine's running game; this can break the session")),
        _ => return None,
    };

    Some(finding)
}

// Splits like the engine's record parser: spaces separate words, double quotes group them,
// and a backslash escapes the next character inside quotes.
fn split_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut had_quotes = false;
    let mut chars = command.chars();

    while let Some(c) = chars.next() {
        match c {
            '\\' if in_quotes => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            '"' => {
                in_quotes = !in_quotes;
                had_quotes = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() || had_quotes {
                    words.push(std::mem::take(&mut current));
                }

                had_quotes = false;
            }
            c => current.push(c),
        }
    }

    if !current.is_empty() || had_quotes {
        words.push(current);
    }

    words
}


#[cfg(test)]
mod tests {
    use super::*;

    fn scan_text(text: &str) -> Vec<Finding> {
        scan(&record::parse(text.as_bytes()).unwrap().records)
    }

    fn levels(text: &str) -> Vec<Level> {
        scan_text(text).into_iter().map(|finding| finding.level).collect()
    }

    #[test]
    fn web_links_are_info_other_targets_are_errors() {
        assert_eq!(levels("mmoButton\n{\n\tcommand \"MessageTo Game OpenURL \\\"https://example.com\\\"\";\n}\n"), vec![Level::Info]);
        assert_eq!(levels("mmoButton\n{\n\tcommand \"MessageTo Game OpenURL \\\"C:/Windows/notepad.exe\\\"\";\n}\n"), vec![Level::Error]);
        assert_eq!(levels("mmoButton\n{\n\tcommand \"MessageTo Game OpenURL notepad\";\n}\n"), vec![Level::Error]);
    }

    #[test]
    fn save_commands() {
        assert_eq!(levels("a\n{\n\tcommand \"MessageTo Mode ConfirmDelete \\\"x\\\" \\\"y\\\"\";\n}\n"), vec![Level::Error]);
        assert_eq!(levels("a\n{\n\tcommand \"MessageTo Mode Save\";\n}\n"), vec![Level::Warning]);
        assert_eq!(levels("mmoSequence\n{\n\tsteps\n\t{\n\t\tmmoStepSave\n\t\t{\n\t\t\tsaveName \"x\";\n\t\t}\n\t}\n}\n"), vec![Level::Warning]);
    }

    #[test]
    fn ui_chains_are_split() {
        assert_eq!(levels("a\n{\n\tcommand \"add_cash 5 && crash\";\n}\n"), vec![Level::Warning]);
    }

    #[test]
    fn harmless_commands_and_sound_commands_pass() {
        assert!(scan_text("a\n{\n\tcommand \"add_cash 100000\";\n}\nmmoStepSound\n{\n\tcommand start;\n}\n").is_empty());
        assert!(scan_text("a\n{\n\tcommand \"MessageTo Hud LockMode \\\"Tutorial_01\\\"\";\n}\n").is_empty());
    }

    #[test]
    fn scenario_fields() {
        let library = "mmoScenarioLibrary\n{\n\tscenarios\n\t{\n\t\tmmoScenarioSequence\n\t\t{\n\t\t\tid \"a\";\n\t\t\tprerequisites \"b\";\n\t\t\ttrigger\n\t\t\t{\n\t\t\t\tmmoPredicate_Event\n\t\t\t\t{\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n";
        let findings = scan_text(library);

        assert_eq!(findings.iter().map(|finding| finding.level).collect::<Vec<_>>(), vec![Level::Warning, Level::Error]);
        assert_eq!(findings[0].path, "mmoScenarioLibrary/scenarios/mmoScenarioSequence[id=a]/prerequisites");
    }

    #[test]
    fn building_prerequisites_are_not_scenario_prerequisites() {
        assert!(scan_text("mmoBuildingFeature\n{\n\tprerequisites \"NoHome\";\n}\n").is_empty());
    }

    #[test]
    fn words_split_like_the_engine() {
        assert_eq!(split_words("MessageTo Game OpenURL \"a b\\\"c\""), vec!["MessageTo", "Game", "OpenURL", "a b\"c"]);
        assert_eq!(split_words("  x   \"\"  "), vec!["x", ""]);
    }
}
