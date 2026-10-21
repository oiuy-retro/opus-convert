//! Still images for Matroska. MKV does not use MP4 attached_pic;
//! the cover is an attached file named cover.jpg.

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
    if let Some(sidecar) = sidecar(item) {
        return jpeg(ffmpeg, &sidecar, work_dir);
    }
    if item.attached_pic {
        return extract(ffmpeg, &item.source, work_dir);
    }
    None
}

pub fn sidecar(item: &Item) -> Option<PathBuf> {
    let dir = item.source.parent()?;
    let stem = item.source.file_stem()?.to_string_lossy();
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
    for name in names {
        let path = dir.join(&name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
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
    if encode_jpeg(ffmpeg, source, &dest) {
        Some(Cover { path: dest, temp: true })
    } else {
        let _ = std::fs::remove_file(&dest);
        None
    }
}

fn extract(ffmpeg: &Path, source: &Path, work_dir: &Path) -> Option<Cover> {
    let dest = work_dir.join(format!(
        ".cover-{}.jpg",
        source.file_stem().unwrap_or_default().to_string_lossy()
    ));
    if encode_jpeg(ffmpeg, source, &dest) {
        Some(Cover { path: dest, temp: true })
    } else {
        let _ = std::fs::remove_file(&dest);
        None
    }
}

fn encode_jpeg(ffmpeg: &Path, source: &Path, dest: &Path) -> bool {
    let mut command = Command::new(ffmpeg);
    command
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
        ])
        .arg(source)
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

    #[test]
    fn prefers_a_same_name_picture() {
        let dir = std::env::temp_dir().join(format!("opus-cover-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let song = dir.join("Song.mp3");
        let pic = dir.join("Song.webp");
        std::fs::write(&song, b"x").unwrap();
        std::fs::write(&pic, b"x").unwrap();
        let item = Item {
            source: song,
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
        };
        assert_eq!(sidecar(&item), Some(pic));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
