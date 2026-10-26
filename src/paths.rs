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
        (stem, "m4a".into())
    } else {
        let ext = item.extension.trim_start_matches('.').to_string();
        (stem, ext)
    }
}

pub fn unique_path(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    candidates(dir, stem, extension)
        .find(|candidate| !candidate.exists())
        .unwrap_or_else(|| last_resort(dir, stem, extension))
}

/// The names `unique_path` tries, in order: `Song.m4a`, `Song (2).m4a`, ...
pub fn candidates<'a>(dir: &'a Path, stem: &'a str, extension: &'a str) -> impl Iterator<Item = PathBuf> + 'a {
    (1..10_000).map(move |number| {
        let name = if number == 1 { stem.to_string() } else { format!("{stem} ({number})") };
        if extension.is_empty() {
            dir.join(name)
        } else {
            dir.join(format!("{name}.{extension}"))
        }
    })
}

fn last_resort(dir: &Path, stem: &str, extension: &str) -> PathBuf {
    if extension.is_empty() {
        dir.join(format!("{stem} copy"))
    } else {
        dir.join(format!("{stem} copy.{extension}"))
    }
}

/// Where a file should go when an earlier run may have left finished copies
/// behind. Walks `Song.m4a`, `Song (2).m4a`, ... skipping names already used in
/// this run (`claimed`, lowercase). The first name that is free is returned with
/// `false` (write it). A name that exists and `is_done` accepts is returned with
/// `true` (the file is already there; skip it). Files that exist but are not
/// accepted are left alone.
pub fn pick_dest(
    dir: &Path,
    stem: &str,
    extension: &str,
    claimed: &mut std::collections::HashSet<String>,
    is_done: impl Fn(&Path) -> bool,
) -> (PathBuf, bool) {
    for candidate in candidates(dir, stem, extension) {
        let key = candidate.to_string_lossy().to_lowercase();
        if claimed.contains(&key) {
            continue;
        }
        if !candidate.exists() {
            claimed.insert(key);
            return (candidate, false);
        }
        if candidate.is_file() && is_done(&candidate) {
            claimed.insert(key);
            return (candidate, true);
        }
    }
    let path = last_resort(dir, stem, extension);
    claimed.insert(path.to_string_lossy().to_lowercase());
    (path, false)
}

/// A folder the user typed, as a full Windows path: relative paths are taken
/// from the current folder, `/` becomes `\`, and `.` and `..` are resolved.
/// Paths that already start with `\\?\` are kept as they are.
pub fn full(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return path.to_path_buf();
    }
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn extended(path: &Path) -> PathBuf {
    let full = full(path);
    let text = full.as_os_str();
    let lossy = text.to_string_lossy();
    if lossy.starts_with(r"\\?\") {
        return full;
    }
    let mut out = std::ffi::OsString::new();
    if lossy.starts_with(r"\\") {
        // \\server\share\x -> \\?\UNC\server\share\x
        out.push(r"\\?\UNC\");
        let wide: Vec<u16> = std::os::windows::ffi::OsStrExt::encode_wide(text).skip(2).collect();
        out.push(<std::ffi::OsString as std::os::windows::ffi::OsStringExt>::from_wide(&wide));
    } else {
        out.push(r"\\?\");
        out.push(text);
    }
    PathBuf::from(out)
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

    #[test]
    fn full_paths_fix_slashes_dots_and_relative_folders() {
        assert_eq!(full(Path::new("D:/Music/./Rock/../Pop")), PathBuf::from(r"D:\Music\Pop"));
        assert_eq!(full(Path::new(r"\\server\share/Music")), PathBuf::from(r"\\server\share\Music"));
    }

    #[test]
    fn extended_prefixes_once_and_handles_unc() {
        assert_eq!(extended(Path::new("D:/Music/x/../Song.flac")), PathBuf::from(r"\\?\D:\Music\Song.flac"));
        assert_eq!(extended(Path::new(r"\\server\share\a")), PathBuf::from(r"\\?\UNC\server\share\a"));
        assert_eq!(extended(Path::new(r"\\?\D:\Music")), PathBuf::from(r"\\?\D:\Music"));
        assert_eq!(extended(Path::new(r"\\?\UNC\server\share")), PathBuf::from(r"\\?\UNC\server\share"));
        assert_eq!(extended(&extended(Path::new("D:/a"))), PathBuf::from(r"\\?\D:\a"));
    }

    #[test]
    fn relative_and_slash_folders_can_be_read() {
        let root = std::env::temp_dir().join(format!("opus-paths-{}", std::process::id()));
        std::fs::create_dir_all(root.join("In")).unwrap();
        std::fs::write(root.join("In").join("a.txt"), b"x").unwrap();
        let slashed = root.to_string_lossy().replace('\\', "/") + "/In/../In";
        assert!(std::fs::read_dir(extended(Path::new(&slashed))).unwrap().count() == 1);
        // The only test that changes the current folder, so nothing else races it.
        let old = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();
        let here = std::env::current_dir().unwrap();
        let named = (full(Path::new("out")), full(Path::new(r"In\..\out")), display_path(&extended(Path::new("out"))));
        let relative = std::fs::read_dir(extended(Path::new("In"))).map(|d| d.count());
        std::fs::create_dir_all(extended(Path::new("Out/Sub"))).unwrap();
        let made = root.join("Out").join("Sub").is_dir();
        std::env::set_current_dir(old).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(named.0, here.join("out"));
        assert_eq!(named.1, here.join("out"));
        assert_eq!(named.2, here.join("out").to_string_lossy());
        assert_eq!(relative.unwrap(), 1);
        assert!(made);
    }

    #[test]
    fn pick_dest_reuses_finished_files_and_never_doubles_up() {
        let dir = std::env::temp_dir().join(format!("opus-pick-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut claimed = std::collections::HashSet::new();
        // First run: nothing there yet.
        assert_eq!(pick_dest(&dir, "Song", "m4a", &mut claimed, |_| true), (dir.join("Song.m4a"), false));
        std::fs::write(dir.join("Song.m4a"), b"done").unwrap();
        // A second song with the same name in the same run gets "(2)".
        assert_eq!(pick_dest(&dir, "Song", "m4a", &mut claimed, |_| true), (dir.join("Song (2).m4a"), false));
        std::fs::write(dir.join("Song (2).m4a"), b"done").unwrap();
        // Second run: both finished files are found again and skipped.
        let mut again = std::collections::HashSet::new();
        assert_eq!(pick_dest(&dir, "Song", "m4a", &mut again, |_| true), (dir.join("Song.m4a"), true));
        assert_eq!(pick_dest(&dir, "song", "m4a", &mut again, |_| true), (dir.join("song (2).m4a"), true));
        // A file that is not a finished copy is left alone, and a new name is used.
        let mut third = std::collections::HashSet::new();
        assert_eq!(pick_dest(&dir, "Song", "m4a", &mut third, |_| false), (dir.join("Song (3).m4a"), false));
        assert!(dir.join("Song.m4a").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
