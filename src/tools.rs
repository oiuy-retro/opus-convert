//! ffmpeg and ffprobe live in the same folder as Opus Convert.exe.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Bundle {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

pub fn exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "The program folder could not be found.".into())
}

/// ffmpeg and ffprobe are the copies sitting next to this program.
pub fn load_bundle() -> Result<Bundle, String> {
    let dir = exe_dir()?;
    let ffmpeg = dir.join("ffmpeg.exe");
    let ffprobe = dir.join("ffprobe.exe");
    let mut missing = Vec::new();
    if !ffmpeg.is_file() {
        missing.push("ffmpeg.exe");
    }
    if !ffprobe.is_file() {
        missing.push("ffprobe.exe");
    }
    if !missing.is_empty() {
        return Err(format!(
            "Put these files in the same folder as Opus Convert.exe: {}.",
            missing.join(", ")
        ));
    }
    Ok(Bundle { ffmpeg, ffprobe })
}
