//! Probe a file with ffprobe.

use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::bitrate;
use crate::model::Kind;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug)]
pub struct Probe {
    pub kind: Kind,
    pub codec: String,
    pub bitrate_kbps: Option<u32>,
    pub duration: f64,
    pub is_opus: bool,
    pub is_lossless: bool,
    pub has_video: bool,
    pub attached_pic: bool,
    pub has_cover: bool,
    pub title: String,
    pub album: String,
    pub artists: Vec<String>,
    pub genre: String,
    pub track_number: Option<u32>,
}

pub fn probe(ffprobe: &Path, file: &Path) -> Result<Probe, String> {
    let mut command = Command::new(ffprobe);
    command
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            "-show_entries",
            "format=duration,bit_rate,tags:stream=codec_type,codec_name,bit_rate,duration,disposition,tags",
        ])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .output()
        .map_err(|err| format!("Could not start ffprobe: {err}"))?;
    if !output.status.success() {
        return Err("ffprobe could not read that file.".into());
    }
    let json: Value =
        serde_json::from_slice(&output.stdout).map_err(|_| "ffprobe did not return JSON.".to_string())?;
    Ok(from_json(&json, file))
}

fn from_json(json: &Value, file: &Path) -> Probe {
    let mut probe = Probe {
        kind: Kind::Junk,
        codec: String::new(),
        bitrate_kbps: None,
        duration: 0.0,
        is_opus: false,
        is_lossless: false,
        has_video: false,
        attached_pic: false,
        has_cover: false,
        title: String::new(),
        album: String::new(),
        artists: Vec::new(),
        genre: String::new(),
        track_number: None,
    };
    let format = json.get("format").cloned().unwrap_or(Value::Null);
    let tags = format.get("tags").cloned().unwrap_or(Value::Null);
    probe.duration = number(&format, "duration");
    probe.title = tag(&tags, &["title", "TITLE"]);
    probe.album = tag(&tags, &["album", "ALBUM"]);
    probe.artists = split_artists(&tag(
        &tags,
        &["artist", "ARTIST", "album_artist", "ALBUM_ARTIST"],
    ));
    probe.genre = tag(&tags, &["genre", "GENRE"]);
    probe.track_number = parse_track(&tag(&tags, &["track", "TRACK", "tracknumber"]));

    let streams = json.get("streams").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut audio: Option<&Value> = None;
    for stream in &streams {
        let kind = stream.get("codec_type").and_then(Value::as_str).unwrap_or("");
        let disposition = stream.get("disposition").cloned().unwrap_or(Value::Null);
        let attached = flag(&disposition, "attached_pic");
        if kind == "video" {
            if attached {
                probe.attached_pic = true;
                probe.has_cover = true;
            } else {
                probe.has_video = true;
            }
        }
        if kind == "attachment" {
            let stream_tags = stream.get("tags").cloned().unwrap_or(Value::Null);
            let filename = tag(&stream_tags, &["filename", "FILENAME"]).to_ascii_lowercase();
            if filename.starts_with("cover.") || filename == "cover" {
                probe.has_cover = true;
            }
        }
        if kind == "audio" && audio.is_none() {
            audio = Some(stream);
        }
        if probe.title.is_empty() {
            if let Some(stream_tags) = stream.get("tags") {
                probe.title = tag(stream_tags, &["title", "TITLE"]);
            }
        }
        if probe.genre.is_empty() {
            if let Some(stream_tags) = stream.get("tags") {
                probe.genre = tag(stream_tags, &["genre", "GENRE"]);
            }
        }
    }

    if let Some(stream) = audio {
        probe.codec = stream
            .get("codec_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        probe.is_opus = bitrate::is_opus(&probe.codec);
        probe.is_lossless = bitrate::is_lossless(&probe.codec);
        if probe.duration <= 0.0 {
            probe.duration = number(stream, "duration");
        }
        let stream_rate = integer(stream, "bit_rate");
        let format_rate = integer(&format, "bit_rate");
        probe.bitrate_kbps = stream_rate
            .or(if probe.has_video { None } else { format_rate })
            .map(|bps| ((bps as f64) / 1000.0).round() as u32);
        if probe.bitrate_kbps.is_none() && probe.duration > 0.0 && !probe.has_video {
            if let Ok(meta) = std::fs::metadata(file) {
                if meta.len() > 0 {
                    let bps = (meta.len() as f64 * 8.0) / probe.duration;
                    probe.bitrate_kbps = Some((bps / 1000.0).round() as u32);
                }
            }
        }
        if probe.artists.is_empty() {
            if let Some(stream_tags) = stream.get("tags") {
                probe.artists = split_artists(&tag(stream_tags, &["artist", "ARTIST"]));
            }
        }
    }

    probe.kind = if probe.has_video {
        Kind::Video
    } else if audio.is_some() {
        Kind::Audio
    } else {
        Kind::Junk
    };
    probe
}

fn number(value: &Value, key: &str) -> f64 {
    match value.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn integer(value: &Value, key: &str) -> Option<u64> {
    match value.get(key) {
        Some(Value::Number(n)) => n.as_u64().or_else(|| n.as_f64().map(|v| v as u64)).filter(|v| *v > 0),
        Some(Value::String(s)) if s != "N/A" => s.parse().ok().filter(|v: &u64| *v > 0),
        _ => None,
    }
}

fn flag(value: &Value, key: &str) -> bool {
    match value.get(key) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::Bool(v)) => *v,
        Some(Value::String(s)) => s == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

fn tag(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(Value::as_str) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    String::new()
}

fn parse_track(text: &str) -> Option<u32> {
    let first = text.split(['/', '-']).next().unwrap_or(text).trim();
    first.parse().ok().filter(|n| *n > 0)
}

pub fn split_artists(text: &str) -> Vec<String> {
    text.split([';', '\n', '\r'])
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

pub fn guess_kind(extension: &str) -> Option<Kind> {
    match extension.to_ascii_lowercase().as_str() {
        "mp3" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wav" | "wma" | "aiff" | "aif" | "alac"
        | "ape" | "wv" | "tta" | "dsf" | "dff" | "mka" | "m4b" | "ac3" | "dts" | "eac3" | "mp2"
        | "amr" | "ra" | "oga" | "caf" => Some(Kind::Audio),
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv" | "m4v" | "mpg" | "mpeg" | "ts"
        | "m2ts" | "vob" | "3gp" | "ogv" | "asf" | "f4v" | "mts" | "divx" | "m2v" | "mpe" => {
            Some(Kind::Video)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_numbers_drop_the_total() {
        assert_eq!(parse_track("3/12"), Some(3));
        assert_eq!(parse_track("07"), Some(7));
        assert_eq!(parse_track(""), None);
    }

    #[test]
    fn artists_split_on_semicolon() {
        assert_eq!(
            split_artists("2 Mello; bop it!; Matthew Hopkins"),
            vec!["2 Mello", "bop it!", "Matthew Hopkins"]
        );
    }
}
