//! Cover picture for the .m4a output, stored there as an attached picture.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::model::{Item, Kind};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Cover {
    pub path: PathBuf,
    temp: bool,
}

impl Cover {
    pub fn cleanup(self) {
        if self.temp {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub fn prepare(ffmpeg: &Path, item: &Item, work_dir: &Path) -> Option<Cover> {
    if item.kind == Kind::Junk {
        return None;
    }
    if item.attached_pic {
        if let Some(cover) = extract(ffmpeg, &item.source, work_dir) {
            return Some(cover);
        }
    }
    for path in sidecars(item) {
        if let Some(cover) = jpeg(ffmpeg, &path, work_dir) {
            return Some(cover);
        }
    }
    None
}

/// First existing picture beside the source, if any.
pub fn sidecar(item: &Item) -> Option<PathBuf> {
    sidecars(item).into_iter().next()
}

/// Every existing picture beside the source, best first:
/// `<song>.*`, then `cover.*`, then `folder.*`.
pub fn sidecars(item: &Item) -> Vec<PathBuf> {
    let Some(dir) = item.source.parent() else {
        return Vec::new();
    };
    let Some(stem) = item.source.file_stem() else {
        return Vec::new();
    };
    let stem = stem.to_string_lossy();
    let names = [
        format!("{stem}.jpg"),
        format!("{stem}.jpeg"),
        format!("{stem}.png"),
        format!("{stem}.webp"),
        "cover.jpg".into(),
        "cover.jpeg".into(),
        "cover.png".into(),
        "cover.webp".into(),
        "folder.jpg".into(),
        "folder.jpeg".into(),
        "folder.png".into(),
        "folder.webp".into(),
    ];
    names
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .collect()
}

fn jpeg(ffmpeg: &Path, source: &Path, work_dir: &Path) -> Option<Cover> {
    if is_jpeg(source) {
        return Some(Cover {
            path: source.to_path_buf(),
            temp: false,
        });
    }
    let dest = work_dir.join(format!(
        ".cover-{}.jpg",
        source.file_stem().unwrap_or_default().to_string_lossy()
    ));
    if encode_jpeg(ffmpeg, source, None, &dest) {
        Some(Cover { path: dest, temp: true })
    } else {
        let _ = std::fs::remove_file(&dest);
        None
    }
}

/// The attached picture only. A plain `-i video` would grab a frame of the
/// video itself, so the stream is picked by its attached_pic disposition.
const ATTACHED_PIC: &str = "0:v:disp:attached_pic:0";

fn extract(ffmpeg: &Path, source: &Path, work_dir: &Path) -> Option<Cover> {
    let dest = work_dir.join(format!(
        ".cover-{}.jpg",
        source.file_stem().unwrap_or_default().to_string_lossy()
    ));
    if encode_jpeg(ffmpeg, source, Some(ATTACHED_PIC), &dest) {
        Some(Cover { path: dest, temp: true })
    } else {
        let _ = std::fs::remove_file(&dest);
        None
    }
}

fn encode_jpeg(ffmpeg: &Path, source: &Path, map: Option<&str>, dest: &Path) -> bool {
    let mut command = Command::new(ffmpeg);
    command
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
        ])
        .arg(source);
    if let Some(map) = map {
        command.args(["-map", map]);
    }
    command
        .args(["-an", "-frames:v", "1", "-c:v", "mjpeg", "-q:v", "2"])
        .arg(dest)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
        .status()
        .map(|status| status.success() && dest.is_file() && file_len(dest) > 32)
        .unwrap_or(false)
}

fn is_jpeg(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 3];
    file.read_exact(&mut buf).is_ok() && buf == [0xFF, 0xD8, 0xFF]
}

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_for(source: PathBuf) -> Item {
        Item {
            source,
            stem: "Song".into(),
            original_stem: "Song".into(),
            extension: "mp3".into(),
            meta_title: String::new(),
            original_title: String::new(),
            artists: Vec::new(),
            original_artists: Vec::new(),
            kind: Kind::Audio,
            size: 1,
            duration: 1.0,
            bitrate_kbps: None,
            codec: "mp3".into(),
            is_opus: false,
            is_lossless: false,
            has_video: false,
            attached_pic: false,
            track_number: None,
            opus_kbps: Some(96),
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opus-cover-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prefers_a_same_name_picture() {
        let dir = temp_dir("same-name");
        let song = dir.join("Song.mp3");
        let pic = dir.join("Song.webp");
        std::fs::write(&song, b"x").unwrap();
        std::fs::write(&pic, b"x").unwrap();
        assert_eq!(sidecar(&item_for(song)), Some(pic));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lists_every_sidecar_in_order() {
        let dir = temp_dir("order");
        let song = dir.join("Song.mp3");
        std::fs::write(&song, b"x").unwrap();
        let folder = dir.join("folder.png");
        let cover = dir.join("cover.jpeg");
        let same = dir.join("Song.png");
        for path in [&folder, &cover, &same] {
            std::fs::write(path, b"x").unwrap();
        }
        assert_eq!(sidecars(&item_for(song)), vec![same, cover, folder]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_past_a_broken_picture() {
        let dir = temp_dir("fallback");
        let song = dir.join("Song.mp3");
        std::fs::write(&song, b"x").unwrap();
        // Not a JPEG, and ffmpeg cannot run, so it cannot be converted.
        std::fs::write(dir.join("Song.png"), b"not an image").unwrap();
        let cover = dir.join("cover.jpg");
        std::fs::write(&cover, [0xFF, 0xD8, 0xFF, 0xE0, 0, 0]).unwrap();
        let missing_ffmpeg = dir.join("no-ffmpeg.exe");
        let picked = prepare(&missing_ffmpeg, &item_for(song), &dir).expect("cover.jpg is used");
        assert_eq!(picked.path, cover);
        assert!(!picked.temp, "a JPEG sidecar is used as is");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_usable_picture_means_no_cover() {
        let dir = temp_dir("none");
        let song = dir.join("Song.mp3");
        std::fs::write(&song, b"x").unwrap();
        std::fs::write(dir.join("cover.png"), b"not an image").unwrap();
        let mut item = item_for(song);
        item.attached_pic = true;
        assert!(prepare(&dir.join("no-ffmpeg.exe"), &item, &dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
