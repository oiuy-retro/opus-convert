//! Profiles live in AppData, not beside the program.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::model::Options;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedProfile {
    pub name: String,
    pub options: Options,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Memory {
    pub last_profile: String,
    pub override_profile: String,
    pub override_input: String,
    pub override_output: String,
    pub profiles: Vec<SavedProfile>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            last_profile: "Default".into(),
            override_profile: String::new(),
            override_input: String::new(),
            override_output: String::new(),
            profiles: vec![SavedProfile {
                name: "Default".into(),
                options: Options::default(),
            }],
        }
    }
}

pub fn file_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Opus Convert").join("profiles.json")
}

pub fn load() -> Memory {
    load_from(&file_path())
}

pub fn save(memory: &Memory) {
    let _ = save_to(&file_path(), memory);
}

pub fn load_from(path: &Path) -> Memory {
    let Ok(text) = fs::read_to_string(path) else {
        return Memory::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Memory::default();
    };
    let mut profiles = Vec::new();
    if let Some(list) = value.get("profiles").and_then(Value::as_array) {
        for entry in list {
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if name.is_empty() {
                continue;
            }
            profiles.push(SavedProfile {
                name,
                options: options_from(entry.get("options").unwrap_or(&Value::Null)),
            });
        }
    }
    if profiles.is_empty() {
        profiles.push(SavedProfile {
            name: "Default".into(),
            options: Options::default(),
        });
    }
    let last = value
        .get("last_profile")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let last_profile = if profiles.iter().any(|profile| profile.name == last) {
        last
    } else {
        profiles[0].name.clone()
    };
    Memory {
        last_profile,
        override_profile: text_at(&value, "override_profile"),
        override_input: text_at(&value, "override_input"),
        override_output: text_at(&value, "override_output"),
        profiles,
    }
}

pub fn save_to(path: &Path, memory: &Memory) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let profiles: Vec<Value> = memory
        .profiles
        .iter()
        .map(|profile| json!({ "name": profile.name, "options": options_to_json(&profile.options) }))
        .collect();
    let value = json!({
        "last_profile": memory.last_profile,
        "override_profile": memory.override_profile,
        "override_input": memory.override_input,
        "override_output": memory.override_output,
        "profiles": profiles,
    });
    let text = serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?;
    fs::write(path, text).map_err(|err| err.to_string())
}

fn options_to_json(opts: &Options) -> Value {
    json!({
        "input": opts.input,
        "output": opts.output,
        "convert_audio": opts.convert_audio,
        "convert_video": opts.convert_video,
        "lookup_genre": opts.lookup_genre,
        "genre_policy": opts.genre_policy.as_str(),
    })
}

fn options_from(value: &Value) -> Options {
    Options {
        input: text_at(value, "input"),
        output: text_at(value, "output"),
        convert_audio: value.get("convert_audio").and_then(Value::as_bool).unwrap_or(false),
        convert_video: value.get("convert_video").and_then(Value::as_bool).unwrap_or(false),
        lookup_genre: value.get("lookup_genre").and_then(Value::as_bool).unwrap_or(false),
        genre_policy: crate::model::GenrePolicy::from_name(&text_at(value, "genre_policy")),
    }
}

fn text_at(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_keeps_the_folders_with_the_profile() {
        let path = std::env::temp_dir().join(format!("opus-convert-store-{}.json", std::process::id()));
        let mut memory = Memory::default();
        memory.profiles[0].options.input = r"C:\In".into();
        memory.profiles[0].options.output = r"D:\Out".into();
        memory.profiles[0].options.convert_audio = true;
        memory.profiles[0].options.lookup_genre = true;
        memory.profiles[0].options.genre_policy = crate::model::GenrePolicy::KeepAudioExceptYoutube;
        memory.last_profile = "Default".into();
        memory.override_profile = "Default".into();
        memory.override_input = r"E:\Other".into();
        save_to(&path, &memory).unwrap();
        let loaded = load_from(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(loaded, memory);
    }
}
