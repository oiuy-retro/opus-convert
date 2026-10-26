//! Walk the input tree and group files by folder.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::bitrate;
use crate::model::{Group, Item, JunkFile, Kind, Options};
use crate::paths;
use crate::probe;
use crate::run::Shared;

pub fn scan(shared: Arc<Shared>, ffprobe: PathBuf, opts: Options) {
    crate::run::reset(&shared);
    crate::run::set_status(&shared, "Reading the folder…");
    let input = paths::full(Path::new(opts.input.trim()));
    let files = match collect_files(&input) {
        Ok(files) => files,
        Err(err) => {
            crate::run::finish(&shared, Vec::new(), err);
            return;
        }
    };
    let total = files.len() as u32;
    {
        let mut live = shared.live.lock().expect("live");
        live.total = total;
    }
    let mut folders: BTreeMap<String, Group> = BTreeMap::new();
    for (index, path) in files.iter().enumerate() {
        if shared.cancel.load(Ordering::Relaxed) {
            crate::run::finish(&shared, Vec::new(), String::new());
            return;
        }
        crate::run::set_status(&shared, &format!("Reading {} of {}", index + 1, files.len()));
        shared.live.lock().expect("live").ok = index as u32;
        let rel_file = relative(&input, path);
        let (folder_rel, file_name) = split_rel(&rel_file);
        let group = folders.entry(folder_rel.to_ascii_lowercase()).or_insert_with(|| {
            let name = folder_name(&folder_rel);
            Group {
                rel: folder_rel.clone(),
                name: name.clone(),
                original_name: name.clone(),
                album: name.clone(),
                original_album: name,
                items: Vec::new(),
                junk: Vec::new(),
            }
        });
        let size = fs::metadata(paths::extended(path)).map(|meta| meta.len()).unwrap_or(0);
        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .unwrap_or_default();
        let guessed = probe::guess_kind(&extension);
        if guessed.is_none() {
            group.junk.push(JunkFile {
                source: path.clone(),
                name: file_name,
                size,
            });
            continue;
        }
        match probe::probe(&ffprobe, path) {
            Ok(info) if info.kind != Kind::Junk => {
                let stem = stem_of(&file_name);
                let title = if info.title.is_empty() {
                    stem.clone()
                } else {
                    info.title.clone()
                };
                if group.album == group.original_name && !info.album.is_empty() {
                    group.album = info.album.clone();
                    group.original_album = info.album.clone();
                }
                group.items.push(Item {
                    source: path.clone(),
                    stem: stem.clone(),
                    original_stem: stem,
                    extension,
                    meta_title: title.clone(),
                    original_title: title,
                    artists: info.artists.clone(),
                    original_artists: info.artists,
                    kind: info.kind,
                    size,
                    duration: info.duration,
                    bitrate_kbps: info.bitrate_kbps,
                    codec: info.codec.clone(),
                    is_opus: info.is_opus,
                    is_lossless: info.is_lossless,
                    has_video: info.has_video,
                    attached_pic: info.attached_pic,
                    track_number: info.track_number,
                    opus_kbps: bitrate::opus_kbps(&info.codec, info.bitrate_kbps),
                });
            }
            _ => {
                group.junk.push(JunkFile {
                    source: path.clone(),
                    name: file_name,
                    size,
                });
            }
        }
    }
    let mut groups: Vec<Group> = folders.into_values().collect();
    groups.sort_by(|a, b| a.rel.to_ascii_lowercase().cmp(&b.rel.to_ascii_lowercase()));
    for group in &mut groups {
        group.items.sort_by(|a, b| a.stem.to_ascii_lowercase().cmp(&b.stem.to_ascii_lowercase()));
        group.junk.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    }
    crate::run::finish(&shared, groups, String::new());
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.is_dir() {
        return Err("The input folder could not be found.".into());
    }
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(paths::extended(&dir)) {
            Ok(entries) => entries,
            Err(err) => return Err(format!("Could not read {}: {err}", dir.display())),
        };
        for entry in entries.flatten() {
            let path = strip_extended(&entry.path());
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }
    files.sort_by(|a, b| a.to_string_lossy().to_ascii_lowercase().cmp(&b.to_string_lossy().to_ascii_lowercase()));
    Ok(files)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('/', "\\"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('/', "\\"))
}

fn split_rel(rel: &str) -> (String, String) {
    match rel.rfind('\\') {
        Some(index) => (rel[..index].to_string(), rel[index + 1..].to_string()),
        None => (String::new(), rel.to_string()),
    }
}

fn folder_name(rel: &str) -> String {
    rel.rsplit('\\').next().unwrap_or(rel).to_string()
}

fn stem_of(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| name.to_string())
}

fn strip_extended(path: &Path) -> PathBuf {
    PathBuf::from(paths::display_path(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_nested_relative_path() {
        let (folder, name) = split_rel(r"Queen\News\Song.mp3");
        assert_eq!(folder, r"Queen\News");
        assert_eq!(name, "Song.mp3");
        assert_eq!(folder_name(&folder), "News");
    }
}
