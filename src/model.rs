//! Folders, files, and convert options.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Audio,
    Video,
    Junk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenrePolicy {
    Replace,
    KeepAudio,
    KeepAudioExceptYoutube,
}

impl Default for GenrePolicy {
    fn default() -> Self {
        Self::Replace
    }
}

impl GenrePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::KeepAudio => "keep_audio",
            Self::KeepAudioExceptYoutube => "keep_audio_except_youtube",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "keep_audio" => Self::KeepAudio,
            "keep_audio_except_youtube" => Self::KeepAudioExceptYoutube,
            _ => Self::Replace,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub input: String,
    pub output: String,
    pub convert_audio: bool,
    pub convert_video: bool,
    pub lookup_genre: bool,
    pub genre_policy: GenrePolicy,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            convert_audio: false,
            convert_video: false,
            lookup_genre: false,
            genre_policy: GenrePolicy::Replace,
        }
    }
}

impl Options {
    /// Convert this kind, or copy it as-is.
    pub fn converts(&self, kind: Kind) -> bool {
        match kind {
            Kind::Junk => false,
            Kind::Audio => self.convert_audio || (!self.convert_audio && !self.convert_video),
            Kind::Video => self.convert_video || (!self.convert_audio && !self.convert_video),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub source: PathBuf,
    pub stem: String,
    pub original_stem: String,
    pub extension: String,
    pub meta_title: String,
    pub original_title: String,
    pub artists: Vec<String>,
    pub original_artists: Vec<String>,
    pub kind: Kind,
    pub size: u64,
    pub duration: f64,
    pub bitrate_kbps: Option<u32>,
    pub codec: String,
    pub is_opus: bool,
    pub is_lossless: bool,
    pub has_video: bool,
    pub attached_pic: bool,
    pub track_number: Option<u32>,
    pub opus_kbps: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunkFile {
    pub source: PathBuf,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// Relative folder from the input root. Empty for files in the root.
    pub rel: String,
    pub name: String,
    pub original_name: String,
    pub album: String,
    pub original_album: String,
    pub items: Vec<Item>,
    pub junk: Vec<JunkFile>,
}

pub fn can_advance(opts: &Options) -> bool {
    problems(opts).is_empty()
}

pub fn problems(opts: &Options) -> Vec<String> {
    let mut out = Vec::new();
    let input = opts.input.trim();
    let output = opts.output.trim();
    if input.is_empty() {
        out.push("Choose an input folder.".into());
    } else if !crate::paths::extended(Path::new(input)).is_dir() {
        out.push("The input folder could not be found.".into());
    }
    if output.is_empty() {
        out.push("Choose an output folder.".into());
    }
    if !input.is_empty() && !output.is_empty() {
        if let Some(message) = nested_message(input, output) {
            out.push(message);
        }
    }
    out
}

fn nested_message(input: &str, output: &str) -> Option<String> {
    let a = normalize(input);
    let b = normalize(output);
    if a.eq_ignore_ascii_case(&b) {
        return Some("The output folder has to be different from the input folder.".into());
    }
    if is_inside(&b, &a) {
        return Some("The output folder cannot sit inside the input folder.".into());
    }
    if is_inside(&a, &b) {
        return Some("The input folder cannot sit inside the output folder.".into());
    }
    None
}

fn normalize(path: &str) -> String {
    // The same full path the scan and the conversion will use.
    let full = crate::paths::full(Path::new(path));
    let mut text = full.to_string_lossy().replace('/', "\\");
    while text.ends_with('\\') && text.len() > 3 {
        text.pop();
    }
    text
}

fn is_inside(inner: &str, outer: &str) -> bool {
    let inner = inner.to_ascii_lowercase();
    let outer = outer.to_ascii_lowercase();
    inner.starts_with(&format!("{outer}\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neither_box_converts_both_kinds() {
        let opts = Options::default();
        assert!(opts.converts(Kind::Audio));
        assert!(opts.converts(Kind::Video));
        assert!(!opts.converts(Kind::Junk));
    }

    #[test]
    fn audio_box_copies_video() {
        let opts = Options {
            convert_audio: true,
            ..Options::default()
        };
        assert!(opts.converts(Kind::Audio));
        assert!(!opts.converts(Kind::Video));
    }

    #[test]
    fn video_box_copies_audio() {
        let opts = Options {
            convert_video: true,
            ..Options::default()
        };
        assert!(!opts.converts(Kind::Audio));
        assert!(opts.converts(Kind::Video));
    }

    #[test]
    fn both_boxes_convert_both() {
        let opts = Options {
            convert_audio: true,
            convert_video: true,
            ..Options::default()
        };
        assert!(opts.converts(Kind::Audio));
        assert!(opts.converts(Kind::Video));
    }

    #[test]
    fn refuses_the_same_or_nested_folder() {
        let opts = Options {
            input: r"C:\Music".into(),
            output: r"C:\Music".into(),
            ..Options::default()
        };
        assert!(!can_advance(&opts));
        let nested = Options {
            input: r"C:\Music".into(),
            output: r"C:\Music\Out".into(),
            ..Options::default()
        };
        assert!(problems(&nested).iter().any(|line| line.contains("inside")));
    }
}
