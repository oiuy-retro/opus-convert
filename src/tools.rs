//! Finding ffmpeg, ffprobe and Python, following `[tools]` in configuration.conf.

use std::path::{Path, PathBuf};

use crate::config::{self, Conf};

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

fn find(conf: &Conf, key: &str, file_name: &str, extra: &[PathBuf]) -> Result<PathBuf, String> {
    let dir = exe_dir()?;
    let path_var = std::env::var_os("PATH");
    config::find_tool(conf, key, file_name, &dir, path_var.as_deref(), extra)
}

/// ffmpeg and ffprobe, as `[tools] ffmpeg` and `[tools] ffprobe` say.
pub fn load_bundle() -> Result<Bundle, String> {
    let conf = config::load();
    let ffmpeg = find(&conf, "ffmpeg", "ffmpeg.exe", &[]);
    let ffprobe = find(&conf, "ffprobe", "ffprobe.exe", &[]);
    match (ffmpeg, ffprobe) {
        (Ok(ffmpeg), Ok(ffprobe)) => Ok(Bundle { ffmpeg, ffprobe }),
        (Err(a), Err(b)) => Err(format!("{a}\n\n{b}")),
        (Err(err), _) | (_, Err(err)) => Err(err),
    }
}

/// Python for musicnn, as `[tools] python` says. `auto` also checks
/// `venv\Scripts\python.exe` beside the exe, where older versions looked.
pub fn find_python() -> Result<PathBuf, String> {
    let conf = config::load();
    let legacy = exe_dir().map(|dir| vec![dir.join("venv").join("Scripts").join("python.exe")]).unwrap_or_default();
    find(&conf, "python", "python.exe", &legacy)
}

/// `[opus-convert] lastfm_key_file`, default `lastfm.key` beside the exe.
pub fn lastfm_key_file() -> Option<PathBuf> {
    let dir = exe_dir().ok()?;
    let conf = config::load();
    let value = conf.get("opus-convert", "lastfm_key_file").unwrap_or("lastfm.key");
    Some(config::resolve(value, &dir))
}
