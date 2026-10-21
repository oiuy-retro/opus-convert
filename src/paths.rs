//! Finished file locations and Windows name checks.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::model::{Group, Item, Kind, Options};

pub fn sanitize(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return "Untitled".into();
    }
    let upper = trimmed.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    let reserved = is_reserved(stem);
    let mut text = if reserved {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    };
    if text.chars().count() > 250 {
        text = text.chars().take(250).collect();
        text = text.trim().trim_end_matches(['.', ' ']).to_string();
    }
    if text.is_empty() {
        "Untitled".into()
    } else {
        text
    }
}

fn is_reserved(stem: &str) -> bool {
    matches!(
        stem,
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
            | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    )
}

pub fn filename_problem(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        return Some("Type a file name.");
    }
    if name
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Some("A file name cannot contain \\ / : * ? \" < > |.");
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Some("A file name cannot end with a space or a period.");
    }
    if name.chars().count() > 250 {
        return Some("That file name is too long.");
    }
    let stem = name.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    if is_reserved(&stem) {
        return Some("That file name is reserved by Windows.");
    }
    None
}

pub fn name_problem_line(groups: &[Group]) -> Option<String> {
    let mut count = 0usize;
    let mut first: Option<(String, &'static str)> = None;
    for group in groups {
        if group.rel.is_empty() {
            continue;
        }
        if let Some(problem) = filename_problem(&group.name) {
            count += 1;
            if first.is_none() {
                first = Some((group.name.clone(), problem));
            }
        }
    }
    for group in groups {
        for item in &group.items {
            if let Some(problem) = filename_problem(&item.stem) {
                count += 1;
                if first.is_none() {
                    first = Some((item.stem.clone(), problem));
                }
            }
        }
    }
    let (name, problem) = first?;
    let detail = if name.trim().is_empty() {
        problem.to_string()
    } else {
        format!("{name}: {problem}")
    };
    if count > 1 {
        Some(format!("{detail} {count} names need a change."))
    } else {
        Some(detail)
    }
}

pub fn folder_map(groups: &[Group]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for group in groups {
        if group.rel.is_empty() {
            continue;
        }
        map.insert(group.rel.to_ascii_lowercase(), group.name.clone());
    }
    map
}

pub fn output_dir(output: &Path, rel: &str, map: &HashMap<String, String>) -> PathBuf {
    let mut path = output.to_path_buf();
    let mut acc = String::new();
    for component in rel.split(['\\', '/']).filter(|part| !part.is_empty()) {
        acc = if acc.is_empty() {
            component.to_string()
        } else {
            format!("{acc}\\{component}")
        };
        let name = map
            .get(&acc.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| component.to_string());
        path = path.join(sanitize(&name));
    }
    path
}

pub fn output_name(item: &Item, opts: &Options) -> (String, String) {
    let stem = sanitize(&item.stem);
    if opts.converts(item.kind) && item.kind != Kind::Junk {
        (stem, "mkv".into())
    } else {
        let ext = item.extension.trim_start_matches('.').to_string();
        (stem, ext)
    }
}

pub fn unique_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    let first = if extension.is_empty() {
        dir.join(stem)
    } else {
        dir.join(format!("{stem}.{extension}"))
    };
    if !first.exists() {
        return first;
    }
    for number in 2..10_000 {
        let candidate = if extension.is_empty() {
            dir.join(format!("{stem} ({number})"))
        } else {
            dir.join(format!("{stem} ({number}).{extension}"))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    if extension.is_empty() {
        dir.join(format!("{stem} copy"))
    } else {
        dir.join(format!("{stem} copy.{extension}"))
    }
}

pub fn extended(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if text.starts_with(r"\\?\") {
        return path.to_path_buf();
    }
    if text.starts_with(r"\\") {
        PathBuf::from(format!(r"\\?\UNC\{}", text.trim_start_matches('\\')))
    } else {
        PathBuf::from(format!(r"\\?\{text}"))
    }
}

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().trim_start_matches(r"\\?\").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_forbidden_characters_and_reserved_names() {
        assert_eq!(sanitize("A:B*C?"), "A_B_C_");
        assert_eq!(sanitize("  CON  "), "_CON");
        assert_eq!(sanitize("   "), "Untitled");
        assert_eq!(sanitize("Song."), "Song");
    }

    #[test]
    fn rejects_names_windows_will_not_store() {
        assert!(filename_problem("Song").is_none());
        assert!(filename_problem("Song: Live").is_some());
        assert!(filename_problem("Song.").is_some());
        assert!(filename_problem("CON").is_some());
        assert!(filename_problem(" ").is_some());
    }

    #[test]
    fn rebuilt_folders_use_the_renamed_components() {
        let mut map = HashMap::new();
        map.insert("artist".into(), "Queen".into());
        map.insert(r"artist\album".into(), "News".into());
        let path = output_dir(Path::new(r"D:\Out"), r"artist\album", &map);
        assert_eq!(path, PathBuf::from(r"D:\Out\Queen\News"));
    }
}
