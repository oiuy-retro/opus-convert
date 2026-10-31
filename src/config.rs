//! `configuration.conf`: a small INI file next to the exe.
//!
//! `[section]`, `key = value`, `#` or `;` comment lines, UTF-8 with or without
//! a BOM. Keys and sections are case-insensitive; unknown ones are ignored. A
//! missing file or key means the default. When the file is missing, a
//! commented template is written. If the exe folder can't be written to, the
//! copy in `%APPDATA%\Opus Convert` is used instead.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "configuration.conf";
pub const TEMPLATE: &str = include_str!("../configuration.conf.example");

/// Parsed settings plus the file they came from (None = defaults only).
#[derive(Clone, Debug, Default)]
pub struct Conf {
    pub path: Option<PathBuf>,
    entries: Vec<(String, String, String)>,
}

impl Conf {
    pub fn parse(text: &str, path: Option<PathBuf>) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut section = String::new();
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
                section = name.trim().to_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim().to_lowercase();
            if key.is_empty() {
                continue;
            }
            let mut value = value.trim();
            if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                value = &value[1..value.len() - 1];
            }
            entries.push((section.clone(), key, value.to_string()));
        }
        Conf { path, entries }
    }

    /// The value, or None when the key is missing or empty. First one wins.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let section = section.to_lowercase();
        let key = key.to_lowercase();
        self.entries
            .iter()
            .find(|(s, k, _)| *s == section && *k == key)
            .map(|(_, _, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }

    /// How the file is named in messages.
    pub fn describe(&self) -> String {
        match &self.path {
            Some(path) => crate::paths::display_path(path),
            None => format!("{FILE_NAME} (not found; using defaults)"),
        }
    }
}

/// Rewrite one `key = value` line in `section`, keeping every other line,
/// comment and the order. A missing key goes after the last setting in the
/// section; a missing section is added at the end.
pub fn set_in_text(text: &str, section: &str, key: &str, value: &str) -> String {
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(rest) => ("\u{feff}", rest),
        None => ("", text),
    };
    let newline = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let value: String = value.chars().filter(|c| *c != '\r' && *c != '\n').collect();
    let mut lines: Vec<String> = body.split_inclusive('\n').map(str::to_string).collect();
    let want_section = section.to_lowercase();
    let want_key = key.to_lowercase();
    let mut current = String::new();
    let mut header_at: Option<usize> = None;
    let mut last_setting: Option<usize> = None;
    for (index, line) in lines.iter_mut().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.is_empty() {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            current = name.trim().to_lowercase();
            if current == want_section && header_at.is_none() {
                header_at = Some(index);
            }
            continue;
        }
        if current != want_section {
            continue;
        }
        let Some((name, _)) = trimmed.split_once('=') else {
            continue;
        };
        last_setting = Some(index);
        if name.trim().to_lowercase() == want_key {
            let indent: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
            let ending = if line.ends_with("\r\n") {
                "\r\n"
            } else if line.ends_with('\n') {
                "\n"
            } else {
                ""
            };
            *line = format!("{indent}{} = {value}{ending}", name.trim());
            return format!("{bom}{}", lines.concat());
        }
    }
    let new_line = format!("{key} = {value}{newline}");
    match last_setting.or(header_at) {
        Some(index) => {
            if !lines[index].ends_with('\n') {
                lines[index].push_str(newline);
            }
            lines.insert(index + 1, new_line);
        }
        None => {
            if let Some(last) = lines.last_mut() {
                if !last.ends_with('\n') {
                    last.push_str(newline);
                }
            }
            if !lines.is_empty() {
                lines.push(newline.to_string());
            }
            lines.push(format!("[{section}]{newline}"));
            lines.push(new_line);
        }
    }
    format!("{bom}{}", lines.concat())
}

pub fn appdata_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join("Opus Convert"))
}

fn writable(path: &Path) -> bool {
    fs::OpenOptions::new().append(true).open(path).is_ok()
}

/// Which file to use. Beside the exe when it is there and writable, or when a
/// template can be written there. Otherwise the AppData copy (seeded from a
/// read-only file beside the exe, or from the template). None = defaults.
pub fn choose(exe_dir: &Path, appdata: Option<&Path>) -> Option<PathBuf> {
    let local = exe_dir.join(FILE_NAME);
    let local_exists = local.is_file();
    if local_exists && writable(&local) {
        return Some(local);
    }
    if !local_exists && fs::write(&local, TEMPLATE).is_ok() {
        return Some(local);
    }
    if let Some(dir) = appdata {
        let roaming = dir.join(FILE_NAME);
        if roaming.is_file() {
            return Some(roaming);
        }
        let seed = if local_exists {
            fs::read_to_string(&local).unwrap_or_else(|_| TEMPLATE.to_string())
        } else {
            TEMPLATE.to_string()
        };
        if fs::create_dir_all(dir).is_ok() && fs::write(&roaming, seed).is_ok() {
            return Some(roaming);
        }
    }
    if local_exists {
        return Some(local);
    }
    None
}

pub fn load_from(exe_dir: &Path, appdata: Option<&Path>) -> Conf {
    match choose(exe_dir, appdata) {
        Some(path) => {
            let text = fs::read(&path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default();
            Conf::parse(&text, Some(path))
        }
        None => Conf::default(),
    }
}

/// Settings for this run of the program.
pub fn load() -> Conf {
    match crate::tools::exe_dir() {
        Ok(dir) => load_from(&dir, appdata_dir().as_deref()),
        Err(_) => Conf::default(),
    }
}

/// Change one value in the file `conf` came from.
pub fn save_value(conf: &Conf, section: &str, key: &str, value: &str) -> Result<(), String> {
    let path = conf.path.as_ref().ok_or_else(|| "No configuration file.".to_string())?;
    let text = fs::read(path).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default();
    fs::write(path, set_in_text(&text, section, key, value)).map_err(|err| err.to_string())
}

/// `%NAME%` replaced by the environment variable (left as is when unset).
pub fn expand_vars(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(value) => out.push_str(&value),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A path from the file: `%VAR%` expanded, `/` turned into `\`, and relative
/// paths taken from the exe's folder (not the current folder).
pub fn resolve(value: &str, exe_dir: &Path) -> PathBuf {
    let text = expand_vars(value.trim()).replace('/', "\\");
    let path = PathBuf::from(&text);
    let path = if path.is_absolute() || text.starts_with('\\') { path } else { exe_dir.join(path) };
    crate::paths::full(&path)
}

fn listed(paths: &[PathBuf]) -> String {
    const SHOW: usize = 12;
    let mut parts: Vec<String> = paths.iter().take(SHOW).map(|p| crate::paths::display_path(p)).collect();
    if paths.len() > SHOW {
        parts.push(format!("and {} more", paths.len() - SHOW));
    }
    parts.join("\n")
}

/// Find one program, following `[tools] <key>`.
///
/// `auto`: beside the exe, each folder beside it, each of their `bin`
/// folders, then `extra` (older places the app used to look), then PATH.
/// `path`: PATH only. Anything else: that file, or `<folder>\<file_name>`,
/// and nothing else.
pub fn find_tool(
    conf: &Conf,
    key: &str,
    file_name: &str,
    exe_dir: &Path,
    path_var: Option<&OsStr>,
    extra: &[PathBuf],
) -> Result<PathBuf, String> {
    let value = conf.get("tools", key).unwrap_or("auto");
    let mode = value.to_ascii_lowercase();
    let mut tried: Vec<PathBuf> = Vec::new();
    let path_dirs = || -> Vec<PathBuf> {
        path_var
            .map(|var| std::env::split_paths(var).map(|dir| PathBuf::from(dir.to_string_lossy().trim_matches('"'))).collect())
            .unwrap_or_default()
    };
    if mode == "auto" || mode == "path" {
        let mut candidates = Vec::new();
        if mode == "auto" {
            candidates.push(exe_dir.join(file_name));
            let mut subs: Vec<PathBuf> = fs::read_dir(exe_dir)
                .map(|entries| entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
                .unwrap_or_default();
            subs.sort_by_key(|p| p.to_string_lossy().to_lowercase());
            candidates.extend(subs.iter().map(|sub| sub.join(file_name)));
            candidates.extend(subs.iter().map(|sub| sub.join("bin").join(file_name)));
            candidates.extend(extra.iter().cloned());
        }
        candidates.extend(path_dirs().into_iter().filter(|d| !d.as_os_str().is_empty()).map(|dir| dir.join(file_name)));
        for candidate in candidates {
            if candidate.is_file() {
                return Ok(candidate);
            }
            tried.push(candidate);
        }
        let how = if mode == "auto" { "auto" } else { "path" };
        return Err(format!(
            "Could not find {file_name}.\n\n[tools] {key} = {how} in {}\n\nLooked in:\n{}",
            conf.describe(),
            if tried.is_empty() { "(nothing to look in)".into() } else { listed(&tried) }
        ));
    }
    let mut path = resolve(value, exe_dir);
    if path.is_dir() {
        path = path.join(file_name);
    }
    if path.is_file() {
        return Ok(path);
    }
    Err(format!(
        "Could not find {file_name}.\n\n[tools] {key} = {value} in {}\n\nThat points to {}, which does not exist.",
        conf.describe(),
        crate::paths::display_path(&path)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opus-conf-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parser_handles_bom_comments_case_and_unknowns() {
        let text = "\u{feff}# top\r\n[Tools]\r\n FFmpeg = C:\\x\\ffmpeg.exe \r\n; other\r\nweird line\r\n[unknown]\r\nffmpeg = no\r\n[tools]\r\nffmpeg = second\r\nffprobe =\r\npython = \"C:\\Py 3\\python.exe\"\r\n";
        let conf = Conf::parse(text, None);
        assert_eq!(conf.get("tools", "ffmpeg"), Some("C:\\x\\ffmpeg.exe"));
        assert_eq!(conf.get("TOOLS", "FFMPEG"), Some("C:\\x\\ffmpeg.exe"));
        assert_eq!(conf.get("tools", "ffprobe"), None);
        assert_eq!(conf.get("tools", "python"), Some("C:\\Py 3\\python.exe"));
        assert_eq!(conf.get("tools", "missing"), None);
        assert_eq!(conf.get("unknown", "ffmpeg"), Some("no"));
    }

    #[test]
    fn template_parses_to_defaults() {
        let conf = Conf::parse(TEMPLATE, None);
        assert_eq!(conf.get("tools", "ffmpeg"), Some("auto"));
        assert_eq!(conf.get("tools", "python"), Some("auto"));
        assert_eq!(conf.get("opus-convert", "lastfm_key_file"), Some("lastfm.key"));
    }

    #[test]
    fn writer_changes_one_line_and_keeps_the_rest() {
        let text = "\u{feff}# keep me\r\n[tools]\r\nffmpeg = auto # not a comment\r\n\r\n[folders]\r\n# where Browse opens\r\nInput = C:\\Old\r\noutput =\r\n\r\n[opus-convert]\r\nx = 1\r\n";
        let out = set_in_text(text, "folders", "input", "D:\\New");
        assert_eq!(out, text.replace("Input = C:\\Old", "Input = D:\\New"));
        let out = set_in_text(text, "FOLDERS", "output", "E:\\Out");
        assert_eq!(out, text.replace("output =\r\n", "output = E:\\Out\r\n"));
        // Missing key: added after the section's last setting.
        let out = set_in_text("[folders]\n# c\ninput = a\n\n[other]\nk = v\n", "folders", "output", "b");
        assert_eq!(out, "[folders]\n# c\ninput = a\noutput = b\n\n[other]\nk = v\n");
        // Missing key in an empty section: right under the header.
        let out = set_in_text("[folders]\n# only a comment\n[x]\n", "folders", "input", "a");
        assert_eq!(out, "[folders]\ninput = a\n# only a comment\n[x]\n");
        // Missing section: appended.
        let out = set_in_text("# c\n[tools]\nffmpeg = auto", "folders", "input", "a");
        assert_eq!(out, "# c\n[tools]\nffmpeg = auto\n\n[folders]\ninput = a\n");
        assert_eq!(set_in_text("", "folders", "input", "a"), "[folders]\ninput = a\n");
        // A key in another section with the same name is not touched.
        let out = set_in_text("[a]\ninput = 1\n[folders]\ninput = 2\n", "folders", "input", "3");
        assert_eq!(out, "[a]\ninput = 1\n[folders]\ninput = 3\n");
    }

    #[test]
    fn missing_file_gets_the_template() {
        let dir = temp("tpl");
        let conf = load_from(&dir, None);
        assert_eq!(conf.path, Some(dir.join(FILE_NAME)));
        assert_eq!(fs::read_to_string(dir.join(FILE_NAME)).unwrap(), TEMPLATE);
        // An existing file is not overwritten.
        fs::write(dir.join(FILE_NAME), "[tools]\nffmpeg = path\n").unwrap();
        assert_eq!(load_from(&dir, None).get("tools", "ffmpeg"), Some("path"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unwritable_exe_folder_falls_back_to_appdata() {
        let root = temp("ro");
        // A "folder" under a plain file can't be created or written to.
        fs::write(root.join("file"), b"x").unwrap();
        let exe_dir = root.join("file").join("app");
        let appdata = root.join("AppData").join("Opus Convert");
        let conf = load_from(&exe_dir, Some(&appdata));
        assert_eq!(conf.path, Some(appdata.join(FILE_NAME)));
        assert_eq!(fs::read_to_string(appdata.join(FILE_NAME)).unwrap(), TEMPLATE);
        save_value(&conf, "tools", "ffmpeg", "path").unwrap();
        assert_eq!(load_from(&exe_dir, Some(&appdata)).get("tools", "ffmpeg"), Some("path"));
        // No AppData either: defaults, no crash.
        let none = load_from(&exe_dir, Some(&root.join("file").join("nope")));
        assert_eq!(none.path, None);
        assert_eq!(none.get("tools", "ffmpeg"), None);

        // A read-only file beside the exe: copied to AppData, which is used from then on.
        let exe2 = root.join("app2");
        fs::create_dir_all(&exe2).unwrap();
        let local = exe2.join(FILE_NAME);
        fs::write(&local, "[tools]\nffprobe = D:\\x\n").unwrap();
        let mut perms = fs::metadata(&local).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&local, perms.clone()).unwrap();
        let appdata2 = root.join("AppData2");
        let conf = load_from(&exe2, Some(&appdata2));
        assert_eq!(conf.path, Some(appdata2.join(FILE_NAME)));
        assert_eq!(conf.get("tools", "ffprobe"), Some("D:\\x"));
        perms.set_readonly(false);
        fs::set_permissions(&local, perms).unwrap();
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn auto_search_order_and_bin_folders() {
        let root = temp("auto");
        let exe = root.join("app");
        let elsewhere = root.join("onpath");
        fs::create_dir_all(exe.join("ffmpeg").join("bin")).unwrap();
        fs::create_dir_all(exe.join("Alpha")).unwrap();
        fs::create_dir_all(&elsewhere).unwrap();
        let conf = Conf::default();
        let path_var = std::env::join_paths([&elsewhere]).unwrap();
        let find = || find_tool(&conf, "ffmpeg", "ffmpeg.exe", &exe, Some(&path_var), &[]);
        assert!(find().unwrap_err().contains("Looked in"));
        fs::write(elsewhere.join("ffmpeg.exe"), b"").unwrap();
        assert_eq!(find().unwrap(), elsewhere.join("ffmpeg.exe"));
        fs::write(exe.join("ffmpeg").join("bin").join("ffmpeg.exe"), b"").unwrap();
        assert_eq!(find().unwrap(), exe.join("ffmpeg").join("bin").join("ffmpeg.exe"));
        fs::write(exe.join("Alpha").join("ffmpeg.exe"), b"").unwrap();
        assert_eq!(find().unwrap(), exe.join("Alpha").join("ffmpeg.exe"));
        fs::write(exe.join("ffmpeg.exe"), b"").unwrap();
        assert_eq!(find().unwrap(), exe.join("ffmpeg.exe"));
        // "path" skips everything beside the exe.
        let conf = Conf::parse("[tools]\nffmpeg = PATH\n", None);
        assert_eq!(find_tool(&conf, "ffmpeg", "ffmpeg.exe", &exe, Some(&path_var), &[]).unwrap(), elsewhere.join("ffmpeg.exe"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extra_places_come_before_path() {
        let root = temp("extra");
        let exe = root.join("app");
        let scripts = exe.join("venv").join("Scripts");
        let onpath = root.join("p");
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&onpath).unwrap();
        fs::write(scripts.join("python.exe"), b"").unwrap();
        fs::write(onpath.join("python.exe"), b"").unwrap();
        let path_var = std::env::join_paths([&onpath]).unwrap();
        let got = find_tool(&Conf::default(), "python", "python.exe", &exe, Some(&path_var), &[scripts.join("python.exe")]);
        assert_eq!(got.unwrap(), scripts.join("python.exe"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_paths_relative_vars_slashes_and_folders() {
        let root = temp("explicit");
        let exe = root.join("app");
        fs::create_dir_all(exe.join("tools").join("ff")).unwrap();
        fs::write(exe.join("tools").join("ff").join("ffmpeg.exe"), b"").unwrap();
        let want = exe.join("tools").join("ff").join("ffmpeg.exe");
        let find = |value: &str| {
            let conf = Conf::parse(&format!("[tools]\nffmpeg = {value}\n"), Some(exe.join(FILE_NAME)));
            find_tool(&conf, "ffmpeg", "ffmpeg.exe", &exe, None, &[])
        };
        assert_eq!(find("tools/ff/ffmpeg.exe").unwrap(), want);
        assert_eq!(find("tools\\ff").unwrap(), want);
        assert_eq!(find(".\\tools/x/../ff/").unwrap(), want);
        std::env::set_var("OPUS_CONF_TEST_DIR", exe.join("tools").as_os_str());
        assert_eq!(find("%OPUS_CONF_TEST_DIR%/ff/ffmpeg.exe").unwrap(), want);
        let absolute = want.to_string_lossy().replace('\\', "/");
        assert_eq!(find(&absolute).unwrap(), want);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_path_that_is_missing_is_an_error_without_fallback() {
        let root = temp("missing");
        let exe = root.join("app");
        fs::create_dir_all(&exe).unwrap();
        fs::write(exe.join("ffmpeg.exe"), b"").unwrap();
        let conf = Conf::parse("[tools]\nffmpeg = C:/nope/ffmpeg.exe\n", Some(exe.join(FILE_NAME)));
        let err = find_tool(&conf, "ffmpeg", "ffmpeg.exe", &exe, None, &[]).unwrap_err();
        assert!(err.contains("[tools] ffmpeg = C:/nope/ffmpeg.exe"), "{err}");
        assert!(err.contains(&crate::paths::display_path(&exe.join(FILE_NAME))), "{err}");
        assert!(err.contains(r"C:\nope\ffmpeg.exe"), "{err}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn expand_vars_leaves_unknown_and_lone_percent() {
        std::env::set_var("OPUS_CONF_X", "abc");
        assert_eq!(expand_vars("%OPUS_CONF_X%\\y"), "abc\\y");
        assert_eq!(expand_vars("%OPUS_CONF_NOT_SET_1%\\y"), "%OPUS_CONF_NOT_SET_1%\\y");
        assert_eq!(expand_vars("50% off"), "50% off");
    }
}
