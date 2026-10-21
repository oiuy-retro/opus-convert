//! Last.fm exact track tags, then musicnn for the rest.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::model::{GenrePolicy, Kind};
use crate::net;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const LASTFM_DELAY_MS: u64 = 300;
const MAX_GENRES: usize = 2;

const YOUTUBE_CATEGORIES: &[&str] = &[
    "game",
    "gaming",
    "music",
    "people & blogs",
    "entertainment",
    "film & animation",
    "comedy",
    "education",
    "howto & style",
    "news & politics",
    "science & technology",
    "pets & animals",
    "travel & events",
    "autos & vehicles",
    "nonprofits & activism",
    "sports",
    "movies",
    "shows",
];

const ALLOWED: &[&str] = &[
    "rock", "pop", "alternative", "indie", "electronic", "dance",
    "alternative rock", "jazz", "metal", "classic rock", "soul",
    "indie rock", "electronica", "folk", "punk", "blues", "hard rock",
    "ambient", "experimental", "hip-hop", "hip hop", "country", "funk",
    "electro", "heavy metal", "progressive rock", "rnb", "r&b", "indie pop",
    "house", "techno", "trance", "drum and bass", "dnb", "dubstep",
    "garage", "breaks", "breakbeat", "idm", "downtempo", "trip-hop",
    "synthwave", "synthpop", "new wave", "post-punk", "gothic",
    "industrial", "ebm", "darkwave", "shoegaze", "post-rock",
    "math rock", "emo", "screamo", "hardcore", "metalcore",
    "death metal", "black metal", "thrash", "doom", "sludge",
    "stoner", "psychedelic", "prog", "progressive", "fusion",
    "latin", "reggae", "ska", "dub", "dancehall", "afrobeat",
    "world", "classical", "opera", "soundtrack", "score",
    "chillout", "chill", "lounge", "acid", "acid house",
    "deep house", "tech house", "minimal", "minimal techno",
    "disco", "nu-disco", "italo", "eurodance", "hardstyle",
    "gabber", "jungle", "breakcore", "glitch", "noise",
    "ambient techno", "intelligent dance music",
];

pub fn api_key() -> String {
    if let Ok(key) = std::env::var("LASTFM_API_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return key;
        }
    }
    if let Ok(dir) = crate::tools::exe_dir() {
        let path = dir.join("lastfm.key");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let key = text.trim();
            if !key.is_empty() {
                return key.to_string();
            }
        }
    }
    String::new()
}

pub fn is_allowed(tag: &str) -> bool {
    let key = tag.trim().to_ascii_lowercase();
    ALLOWED.iter().any(|allowed| *allowed == key)
}

pub fn pretty(tag: &str) -> String {
    let lower = tag.trim().to_ascii_lowercase();
    match lower.as_str() {
        "r&b" | "rnb" => "R&B".into(),
        "idm" | "intelligent dance music" => "IDM".into(),
        "ebm" => "EBM".into(),
        "dnb" | "drum and bass" => "Drum and Bass".into(),
        "hip hop" | "hip-hop" => "Hip-Hop".into(),
        _ => {
            let mut out = String::new();
            for (index, word) in lower.split_whitespace().enumerate() {
                if index > 0 {
                    out.push(' ');
                }
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    out.extend(first.to_uppercase());
                    out.push_str(&chars.as_str());
                }
            }
            out
        }
    }
}

pub fn pick_genres(tags: &[String], max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tag in tags {
        if !is_allowed(tag) {
            continue;
        }
        let pretty = pretty(tag);
        let key = pretty.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(pretty);
        }
        if out.len() >= max {
            break;
        }
    }
    out
}

pub fn join_genres(genres: &[String]) -> String {
    genres.join("; ")
}

pub fn is_youtube_category(tag: &str) -> bool {
    let key = tag.trim().to_ascii_lowercase();
    YOUTUBE_CATEGORIES.iter().any(|name| *name == key)
}

pub fn split_genre_parts(text: &str) -> Vec<String> {
    text.split([';', '/', ',', '\n', '\r'])
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

/// What to do with the genre already on a converted file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExistingGenre {
    Lookup,
    Skip,
    Write(String),
    LookupOrClear,
}

pub fn plan_for_file(policy: GenrePolicy, kind: Kind, genre: &str) -> ExistingGenre {
    if kind == Kind::Video {
        return ExistingGenre::Lookup;
    }
    plan_for_audio(policy, genre)
}

pub fn plan_for_audio(policy: GenrePolicy, genre: &str) -> ExistingGenre {
    match policy {
        GenrePolicy::Replace => ExistingGenre::Lookup,
        GenrePolicy::KeepAudio => {
            if genre.trim().is_empty() {
                ExistingGenre::Lookup
            } else {
                ExistingGenre::Skip
            }
        }
        GenrePolicy::KeepAudioExceptYoutube => {
            let parts = split_genre_parts(genre);
            if parts.is_empty() {
                return ExistingGenre::Lookup;
            }
            let kept: Vec<String> = parts
                .iter()
                .filter(|part| !is_youtube_category(part))
                .cloned()
                .collect();
            if kept.is_empty() {
                ExistingGenre::LookupOrClear
            } else if kept.len() == parts.len() {
                ExistingGenre::Skip
            } else {
                ExistingGenre::Write(join_genres(&kept))
            }
        }
    }
}

pub fn lastfm_url(artist: &str, title: &str, key: &str) -> String {
    format!(
        "https://ws.audioscrobbler.com/2.0/?method=track.getTopTags&artist={}&track={}&api_key={}&format=json&autocorrect=0",
        encode(artist.trim()),
        encode(title.trim()),
        encode(key.trim())
    )
}

/// Exact artist+title lookup. `None` means Last.fm does not have that pair.
pub fn lastfm_genres(artist: &str, title: &str, key: &str) -> Result<Option<Vec<String>>, String> {
    if key.trim().is_empty() || artist.trim().is_empty() || title.trim().is_empty() {
        return Ok(None);
    }
    let body = net::get_text(&lastfm_url(artist, title, key))?;
    genres_from_lastfm(&body)
}

pub fn genres_from_lastfm(body: &str) -> Result<Option<Vec<String>>, String> {
    let json: Value = serde_json::from_str(body).map_err(|err| err.to_string())?;
    if json.get("error").is_some() {
        return Ok(None);
    }
    let tags = tag_names(&json);
    if tags.is_empty() {
        return Ok(None);
    }
    Ok(Some(pick_genres(&tags, MAX_GENRES)))
}

fn tag_names(json: &Value) -> Vec<String> {
    match json.pointer("/toptags/tag") {
        Some(Value::Array(list)) => list
            .iter()
            .filter_map(|tag| tag.get("name").and_then(Value::as_str).map(|s| s.to_string()))
            .collect(),
        Some(Value::Object(tag)) => tag
            .get("name")
            .and_then(Value::as_str)
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

pub fn prepare_musicnn(python: &Path, script: &Path) -> Command {
    let mut command = Command::new(python);
    command
        .arg(script)
        .arg("--batch")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Ok(dir) = crate::tools::exe_dir() {
        let mut path = dir.to_string_lossy().into_owned();
        if let Some(old) = std::env::var_os("PATH") {
            path.push(';');
            path.push_str(&old.to_string_lossy());
        }
        command.env("PATH", path);
        command.env("PYTHONHOME", &dir);
        command.env_remove("PYTHONPATH");
        command.env("TF_CPP_MIN_LOG_LEVEL", "3");
        command.env("PYTHONIOENCODING", "utf-8");
        command.env("PYTHONUNBUFFERED", "1");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

pub fn genres_from_musicnn(text: &str) -> Result<Vec<String>, String> {
    let line = text
        .lines()
        .rev()
        .find(|line| line.trim_start().starts_with('{'))
        .unwrap_or("{}");
    let json: Value = serde_json::from_str(line).map_err(|err| err.to_string())?;
    let big = string_list(&json, "big");
    let reg = string_list(&json, "reg");
    Ok(intersect(&pick_genres(&big, 3), &pick_genres(&reg, 3), MAX_GENRES))
}

pub fn intersect(big: &[String], reg: &[String], max: usize) -> Vec<String> {
    let reg_set: std::collections::HashSet<String> = reg.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut out = Vec::new();
    for tag in big {
        if reg_set.contains(&tag.to_ascii_lowercase()) {
            out.push(tag.clone());
        }
        if out.len() >= max {
            break;
        }
    }
    out
}

fn string_list(json: &Value, key: &str) -> Vec<String> {
    json.get(key)
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|value| value.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn lastfm_delay() -> std::time::Duration {
    std::time::Duration::from_millis(LASTFM_DELAY_MS)
}

pub fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*byte as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn python_next_to_exe() -> Result<PathBuf, String> {
    let dir = crate::tools::exe_dir()?;
    for candidate in [dir.join("python.exe"), dir.join("venv").join("Scripts").join("python.exe")] {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err("Put python.exe (with musicnn) in the same folder as Opus Convert.exe.".into())
}

pub fn script_next_to_exe() -> Result<PathBuf, String> {
    let path = crate::tools::exe_dir()?.join("musicnn-one.py");
    if path.is_file() {
        Ok(path)
    } else {
        Err("Put musicnn-one.py in the same folder as Opus Convert.exe.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lastfm_empty_tags_means_no_track() {
        assert_eq!(genres_from_lastfm(r#"{"toptags":{"tag":[]}}"#).unwrap(), None);
        assert_eq!(genres_from_lastfm(r#"{"error":6,"message":"Track not found"}"#).unwrap(), None);
    }

    #[test]
    fn lastfm_skips_empty_key() {
        assert_eq!(lastfm_genres("Artist", "Title", "").unwrap(), None);
    }

    #[test]
    fn lastfm_keeps_two_allowed_tags() {
        let body = r#"{"toptags":{"tag":[{"name":"seen live"},{"name":"techno"},{"name":"electronic"},{"name":"awesome"}]}}"#;
        assert_eq!(
            genres_from_lastfm(body).unwrap(),
            Some(vec!["Techno".into(), "Electronic".into()])
        );
    }

    #[test]
    fn musicnn_needs_both_models() {
        assert_eq!(
            intersect(&["Techno".into(), "Pop".into()], &["Pop".into(), "Rock".into()], 2),
            vec!["Pop".to_string()]
        );
        assert!(intersect(&["Techno".into()], &["House".into()], 2).is_empty());
    }

    #[test]
    fn musicnn_json_intersects_models() {
        let line = r#"{"path":"a.m4a","big":["Techno","Pop","Rock"],"reg":["Pop","Jazz"]}"#;
        assert_eq!(genres_from_musicnn(line).unwrap(), vec!["Pop".to_string()]);
    }

    #[test]
    fn encodes_artist_and_title() {
        let url = lastfm_url("2 Mello", "Song Name", "key");
        assert!(url.contains("artist=2%20Mello"));
        assert!(url.contains("track=Song%20Name"));
        assert!(url.contains("autocorrect=0"));
    }

    #[test]
    fn youtube_categories_are_not_real_genres() {
        assert!(is_youtube_category("Music"));
        assert!(is_youtube_category("people & blogs"));
        assert!(is_youtube_category("Gaming"));
        assert!(!is_youtube_category("Electronic"));
        assert!(!is_youtube_category("Soundtrack"));
    }

    #[test]
    fn keep_audio_skips_a_filled_genre() {
        assert_eq!(
            plan_for_audio(GenrePolicy::KeepAudio, "Techno"),
            ExistingGenre::Skip
        );
        assert_eq!(
            plan_for_audio(GenrePolicy::KeepAudio, ""),
            ExistingGenre::Lookup
        );
        assert_eq!(
            plan_for_file(GenrePolicy::KeepAudio, Kind::Video, "Entertainment"),
            ExistingGenre::Lookup
        );
    }

    #[test]
    fn youtube_genre_is_stripped_then_looked_up() {
        assert_eq!(
            plan_for_audio(GenrePolicy::KeepAudioExceptYoutube, "Music"),
            ExistingGenre::LookupOrClear
        );
        assert_eq!(
            plan_for_audio(GenrePolicy::KeepAudioExceptYoutube, "Music; Electronic"),
            ExistingGenre::Write("Electronic".into())
        );
        assert_eq!(
            plan_for_audio(GenrePolicy::KeepAudioExceptYoutube, "Jazz"),
            ExistingGenre::Skip
        );
        assert_eq!(
            plan_for_audio(GenrePolicy::Replace, "Music"),
            ExistingGenre::Lookup
        );
    }
}
