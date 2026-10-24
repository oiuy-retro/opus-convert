//! Scan the tree, then copy or convert each file.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::convert;
use crate::cover;
use crate::genre;
use crate::model::{Group, Item, Kind, Options};
use crate::paths;
use crate::probe;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Queued,
    Active,
    Waiting,
    Pinned,
    Failed,
}

#[derive(Clone, Debug)]
pub struct WorkRow {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub copied: u64,
    pub fraction: f64,
    pub kind: RowKind,
    pub note: String,
}

#[derive(Clone, Debug)]
pub struct Live {
    pub status: String,
    pub rows: Vec<WorkRow>,
    pub banked_bytes: u64,
    pub current_bytes: u64,
    pub ok: u32,
    pub bad: u32,
    pub total: u32,
    pub lastfm_done: u32,
    pub lastfm_total: u32,
    pub musicnn_done: u32,
    pub musicnn_total: u32,
    pub done: bool,
    pub error: String,
}

impl Live {
    pub fn new() -> Self {
        Self {
            status: String::new(),
            rows: Vec::new(),
            banked_bytes: 0,
            current_bytes: 0,
            ok: 0,
            bad: 0,
            total: 0,
            lastfm_done: 0,
            lastfm_total: 0,
            musicnn_done: 0,
            musicnn_total: 0,
            done: false,
            error: String::new(),
        }
    }
}

pub struct Shared {
    pub live: Mutex<Live>,
    pub groups: Mutex<Option<Vec<Group>>>,
    pub child: Mutex<Option<Child>>,
    pub cancel: AtomicBool,
}

impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            live: Mutex::new(Live::new()),
            groups: Mutex::new(None),
            child: Mutex::new(None),
            cancel: AtomicBool::new(false),
        })
    }
}

struct RunOut {
    code: i32,
    log: String,
}

pub fn reset(shared: &Shared) {
    shared.cancel.store(false, Ordering::Relaxed);
    *shared.groups.lock().expect("groups") = None;
    *shared.live.lock().expect("live") = Live::new();
}

pub fn set_status(shared: &Shared, text: &str) {
    shared.live.lock().expect("live").status = text.to_string();
}

pub fn finish(shared: &Shared, groups: Vec<Group>, error: String) {
    let mut live = shared.live.lock().expect("live");
    live.done = true;
    live.status = if error.is_empty() {
        if shared.cancel.load(Ordering::Relaxed) {
            "Stopped.".into()
        } else {
            "Done.".into()
        }
    } else {
        error.clone()
    };
    if !error.is_empty() {
        live.error = error;
    }
    *shared.groups.lock().expect("groups") = Some(groups);
}

pub fn kill_child(shared: &Shared) {
    shared.cancel.store(true, Ordering::Relaxed);
    if let Ok(mut slot) = shared.child.lock() {
        if let Some(child) = slot.as_mut() {
            let _ = child.kill();
        }
    }
}

pub fn convert(shared: Arc<Shared>, ffmpeg: PathBuf, ffprobe: PathBuf, opts: Options, groups: Vec<Group>) {
    reset(&shared);
    let output = paths::full(Path::new(opts.output.trim()));
    if let Err(err) = fs::create_dir_all(paths::extended(&output)) {
        finish(&shared, groups, format!("Could not create the output folder: {err}"));
        return;
    }
    let map = paths::folder_map(&groups);
    let (rows, jobs) = rows_for(&groups);
    {
        let mut live = shared.live.lock().expect("live");
        live.total = jobs;
        live.rows = rows;
    }
    let mut ready: Vec<ReadyFile> = Vec::new();
    let mut claimed = std::collections::HashSet::new();
    for group in &groups {
        let dir = paths::output_dir(&output, &group.rel, &map);
        let _ = fs::create_dir_all(paths::extended(&dir));
    }
    for group_index in 0..groups.len() {
        if cancelled(&shared) {
            finish(&shared, groups, String::new());
            return;
        }
        let dir = paths::output_dir(&output, &groups[group_index].rel, &map);
        let junk = groups[group_index].junk.clone();
        for file in junk {
            if cancelled(&shared) {
                finish(&shared, groups, String::new());
                return;
            }
            let row_id = row_id(&file.source);
            set_kind(&shared, row_id, RowKind::Active, "");
            set_status(&shared, &file.name);
            let dest = dir.join(&file.name);
            match copy_file(&shared, row_id, &file.source, &dest, file.size) {
                Ok(()) => remove_done(&shared, row_id, file.size),
                Err(err) => fail(&shared, row_id, &err),
            }
        }
        let count = groups[group_index].items.len();
        for item_index in 0..count {
            if cancelled(&shared) {
                finish(&shared, groups, String::new());
                return;
            }
            let item = groups[group_index].items[item_index].clone();
            let album = groups[group_index].album.clone();
            let row_id = row_id(&item.source);
            set_kind(&shared, row_id, RowKind::Active, "");
            set_status(&shared, &item.stem);
            let (stem, extension) = paths::output_name(&item, &opts);
            let copy = convert::copy_media(&item, &opts);
            // A finished file from an earlier run is kept instead of making
            // "Song (2).m4a" next to it.
            let (dest, done) = paths::pick_dest(&dir, &stem, &extension, &mut claimed, |path| {
                if copy {
                    file_len(path) == item.size
                } else {
                    finished(&ffprobe, &item, path)
                }
            });
            if done {
                let size = file_len(&dest);
                set_fraction(&shared, row_id, 1.0);
                remember(&mut ready, &item, dest, size);
                remove_done(&shared, row_id, size);
                continue;
            }
            if copy {
                match copy_file(&shared, row_id, &item.source, &dest, item.size) {
                    Ok(()) => {
                        remember(&mut ready, &item, dest, item.size);
                        remove_done(&shared, row_id, item.size);
                    }
                    Err(err) => fail(&shared, row_id, &err),
                }
                continue;
            }
            match convert_one(&shared, &ffmpeg, &item, &dest, &album) {
                Ok(size) => {
                    remember(&mut ready, &item, dest, size);
                    remove_done(&shared, row_id, size);
                }
                Err(err) => {
                    let _ = fs::remove_file(paths::extended(&dest));
                    let fallback = fallback_path(&dir, &item);
                    match copy_file(&shared, row_id, &item.source, &fallback, item.size) {
                        Ok(()) => {
                            remember(&mut ready, &item, fallback, item.size);
                            pin(&shared, row_id, &format!("Copied the original. {err}"));
                        }
                        Err(_) => fail(&shared, row_id, &err),
                    }
                }
            }
        }
    }
    if opts.lookup_genre && !cancelled(&shared) {
        lookup_genres(&shared, &ffmpeg, &ffprobe, ready, opts.genre_policy);
    }
    finish(&shared, groups, String::new());
}

#[derive(Clone)]
struct ReadyFile {
    dest: PathBuf,
    name: String,
    size: u64,
    clear_if_missing: bool,
}

fn remember(ready: &mut Vec<ReadyFile>, item: &Item, dest: PathBuf, size: u64) {
    if item.kind == Kind::Junk {
        return;
    }
    ready.push(ReadyFile {
        dest,
        name: item.stem.clone(),
        size,
        clear_if_missing: false,
    });
}

fn convert_one(
    shared: &Arc<Shared>,
    ffmpeg: &Path,
    item: &Item,
    dest: &Path,
    album: &str,
) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(paths::extended(parent)).map_err(|err| err.to_string())?;
    }
    let work_dir = dest.parent().unwrap_or(Path::new("."));
    let cover = cover::prepare(ffmpeg, item, work_dir);
    let cover_path = cover.as_ref().map(|cover| cover.path.clone());
    let partial = dest.with_extension("partial.m4a");
    let _ = fs::remove_file(paths::extended(&partial));
    let args = convert::ffmpeg_args(
        &item.source,
        &partial,
        item,
        &item.meta_title,
        &item.artists,
        album,
        cover_path.as_deref(),
    );
    let row_id = row_id(&item.source);
    let duration = item.duration;
    let shared_progress = Arc::clone(shared);
    let result = run_command(shared, ffmpeg, &args, move |line| {
        if let Some(seconds) = convert::parse_out_time(line) {
            if duration > 0.0 {
                set_fraction(&shared_progress, row_id, (seconds / duration).clamp(0.0, 0.99));
            }
        }
    });
    if let Some(cover) = cover {
        cover.cleanup();
    }
    match result {
        Ok(output) if output.code == 0 && file_len(&partial) > 0 => {
            if dest.exists() {
                let _ = fs::remove_file(paths::extended(dest));
            }
            fs::rename(paths::extended(&partial), paths::extended(dest)).map_err(|err| err.to_string())?;
            set_fraction(shared, row_id, 1.0);
            Ok(file_len(dest))
        }
        Ok(output) => {
            let _ = fs::remove_file(paths::extended(&partial));
            Err(tail(&output.log))
        }
        Err(err) => {
            let _ = fs::remove_file(paths::extended(&partial));
            Err(err)
        }
    }
}

/// True when `dest` is a finished conversion of `item` from an earlier run:
/// it plays, is at least 90% as long as the source, and has the cover when
/// one is expected. Nothing is deleted here.
fn finished(ffprobe: &Path, item: &Item, dest: &Path) -> bool {
    if !dest.is_file() {
        return false;
    }
    let Ok(info) = probe::probe(ffprobe, dest) else {
        return false;
    };
    if info.duration <= 0.0 {
        return false;
    }
    if item.duration > 1.0 && info.duration < item.duration * 0.90 {
        return false;
    }
    let wants_cover = item.attached_pic || cover::sidecar(item).is_some();
    !(wants_cover && !info.has_cover)
}

fn fallback_path(dir: &Path, item: &Item) -> PathBuf {
    paths::unique_path(dir, &paths::sanitize(&item.stem), &item.extension)
}

fn filter_genre_files(
    shared: &Arc<Shared>,
    ffmpeg: &Path,
    ffprobe: &Path,
    files: Vec<ReadyFile>,
    policy: crate::model::GenrePolicy,
) -> Option<Vec<ReadyFile>> {
    let mut lookup = Vec::new();
    for mut file in files {
        if cancelled(shared) {
            return None;
        }
        match classify_genre(shared, ffmpeg, ffprobe, &mut file, policy) {
            Ok(true) => lookup.push(file),
            Ok(false) => {}
            Err(err) => fail(shared, row_id(&file.dest), &err),
        }
    }
    Some(lookup)
}

fn classify_genre(
    shared: &Arc<Shared>,
    ffmpeg: &Path,
    ffprobe: &Path,
    file: &mut ReadyFile,
    policy: crate::model::GenrePolicy,
) -> Result<bool, String> {
    let info = probe::probe(ffprobe, &file.dest)?;
    match genre::plan_for_file(policy, info.kind, &info.genre) {
        genre::ExistingGenre::Skip => Ok(false),
        genre::ExistingGenre::Write(cleaned) => {
            write_genre(shared, ffmpeg, &file.dest, &cleaned)?;
            Ok(false)
        }
        genre::ExistingGenre::Lookup => {
            file.clear_if_missing = false;
            Ok(true)
        }
        genre::ExistingGenre::LookupOrClear => {
            file.clear_if_missing = true;
            Ok(true)
        }
    }
}

fn lookup_genres(
    shared: &Arc<Shared>,
    ffmpeg: &Path,
    ffprobe: &Path,
    files: Vec<ReadyFile>,
    policy: crate::model::GenrePolicy,
) {
    let key = genre::api_key();
    let python = crate::genre::python_next_to_exe();
    let script = crate::genre::script_next_to_exe();
    let files = match filter_genre_files(shared, ffmpeg, ffprobe, files, policy) {
        Some(files) => files,
        None => return,
    };
    {
        let mut live = shared.live.lock().expect("live");
        live.lastfm_total = if key.trim().is_empty() {
            0
        } else {
            files.len() as u32
        };
        live.lastfm_done = 0;
        live.musicnn_total = 0;
        live.musicnn_done = 0;
        live.rows = files
            .iter()
            .map(|file| WorkRow {
                id: row_id(&file.dest),
                name: file.name.clone(),
                size: file.size,
                copied: 0,
                fraction: 0.0,
                kind: RowKind::Queued,
                note: String::new(),
            })
            .collect();
    }
    let leftover = if key.trim().is_empty() {
        files
    } else {
        let mut leftover = Vec::new();
        for file in &files {
            if cancelled(shared) {
                return;
            }
            let row_id = row_id(&file.dest);
            set_kind(shared, row_id, RowKind::Active, "");
            set_status(shared, &format!("Last.fm  {}", file.name));
            let matched = match lastfm_one(shared, ffmpeg, ffprobe, file, &key) {
                Ok(true) => true,
                Ok(false) => false,
                Err(err) => {
                    fail(shared, row_id, &err);
                    bump_lastfm(shared);
                    leftover.push(file.clone());
                    continue;
                }
            };
            bump_lastfm(shared);
            if matched {
                remove_done(shared, row_id, file.size);
            } else {
                leftover.push(file.clone());
                set_kind(shared, row_id, RowKind::Queued, "");
            }
            sleep_cancel(shared, genre::lastfm_delay());
        }
        leftover
    };
    {
        let mut live = shared.live.lock().expect("live");
        live.musicnn_total = leftover.len() as u32;
        live.rows = leftover
            .iter()
            .map(|file| WorkRow {
                id: row_id(&file.dest),
                name: file.name.clone(),
                size: file.size,
                copied: 0,
                fraction: 0.0,
                kind: RowKind::Queued,
                note: String::new(),
            })
            .collect();
    }
    if leftover.is_empty() {
        return;
    }
    let (python, script) = match (python, script) {
        (Ok(python), Ok(script)) => (python, script),
        (Err(err), _) | (_, Err(err)) => {
            set_status(shared, &err);
            for file in leftover {
                fail(shared, row_id(&file.dest), &err);
                bump_musicnn(shared);
            }
            return;
        }
    };
    set_status(shared, "Loading musicnn...");
    let mut child = match genre::prepare_musicnn(&python, &script).spawn() {
        Ok(child) => child,
        Err(err) => {
            let err = format!("Could not start musicnn: {err}");
            set_status(shared, &err);
            for file in leftover {
                fail(shared, row_id(&file.dest), &err);
                bump_musicnn(shared);
            }
            return;
        }
    };
    let mut stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            fail_musicnn_all(shared, leftover, "musicnn has no input pipe.");
            return;
        }
    };
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            fail_musicnn_all(shared, leftover, "musicnn has no output pipe.");
            return;
        }
    };
    if let Some(stderr) = child.stderr.take() {
        thread::spawn(move || {
            let mut log = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut log);
        });
    }
    *shared.child.lock().expect("child") = Some(child);
    let mut reader = BufReader::new(stdout);
    let mut stopped: Option<String> = None;
    for file in leftover {
        if cancelled(shared) {
            drop(stdin);
            if let Some(mut child) = shared.child.lock().expect("child").take() {
                let _ = child.kill();
                let _ = child.wait();
            }
            return;
        }
        let row_id = row_id(&file.dest);
        if let Some(err) = stopped.as_ref() {
            fail(shared, row_id, err);
            bump_musicnn(shared);
            continue;
        }
        set_kind(shared, row_id, RowKind::Active, "");
        set_status(shared, &format!("musicnn  {}", file.name));
        let path = file.dest.to_string_lossy();
        if writeln!(stdin, "{path}").is_err() || stdin.flush().is_err() {
            stopped = Some("musicnn stopped.".to_string());
            fail(shared, row_id, stopped.as_ref().unwrap());
            bump_musicnn(shared);
            continue;
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => {
                stopped = Some("musicnn stopped.".to_string());
                fail(shared, row_id, stopped.as_ref().unwrap());
                bump_musicnn(shared);
                continue;
            }
            Ok(_) => {}
        }
        match apply_musicnn_line(shared, ffmpeg, &file, row_id, &line) {
            Ok(()) => {}
            Err(err) => fail(shared, row_id, &err),
        }
        bump_musicnn(shared);
    }
    drop(stdin);
    if let Some(mut child) = shared.child.lock().expect("child").take() {
        let _ = child.wait();
    }
}

fn fail_musicnn_all(shared: &Arc<Shared>, leftover: Vec<ReadyFile>, err: &str) {
    set_status(shared, err);
    for file in leftover {
        fail(shared, row_id(&file.dest), err);
        bump_musicnn(shared);
    }
}

fn apply_musicnn_line(
    shared: &Arc<Shared>,
    ffmpeg: &Path,
    file: &ReadyFile,
    row_id: u64,
    line: &str,
) -> Result<(), String> {
    let genres = genre::genres_from_musicnn(line)?;
    if genres.is_empty() {
        if file.clear_if_missing {
            write_genre(shared, ffmpeg, &file.dest, "")?;
        }
        set_kind(shared, row_id, RowKind::Pinned, "No genre");
        bump_ok(shared);
        return Ok(());
    }
    write_genre(shared, ffmpeg, &file.dest, &genre::join_genres(&genres))?;
    remove_done(shared, row_id, file.size);
    Ok(())
}

fn lastfm_one(
    shared: &Arc<Shared>,
    ffmpeg: &Path,
    ffprobe: &Path,
    file: &ReadyFile,
    key: &str,
) -> Result<bool, String> {
    let info = probe::probe(ffprobe, &file.dest)?;
    let artist = info.artists.first().cloned().unwrap_or_default();
    let title = info.title;
    let Some(genres) = genre::lastfm_genres(&artist, &title, key)? else {
        return Ok(false);
    };
    if genres.is_empty() {
        return Ok(false);
    }
    write_genre(shared, ffmpeg, &file.dest, &genre::join_genres(&genres))?;
    Ok(true)
}

fn write_genre(shared: &Arc<Shared>, ffmpeg: &Path, dest: &Path, genre: &str) -> Result<(), String> {
    let stem = dest.file_stem().unwrap_or_default().to_string_lossy();
    let ext = dest.extension().map(|ext| ext.to_string_lossy().to_string()).unwrap_or_else(|| "m4a".into());
    let partial = dest.with_file_name(format!("{stem}.genre-partial.{ext}"));
    let args = convert::genre_args(dest, &partial, genre);
    let parked = shared.child.lock().expect("child").take();
    let result = run_command(shared, ffmpeg, &args, |_| {});
    *shared.child.lock().expect("child") = parked;
    match result {
        Ok(output) if output.code == 0 && file_len(&partial) > 0 => {
            let _ = fs::remove_file(paths::extended(dest));
            fs::rename(paths::extended(&partial), paths::extended(dest)).map_err(|err| err.to_string())?;
            Ok(())
        }
        Ok(output) => {
            let _ = fs::remove_file(paths::extended(&partial));
            Err(tail(&output.log))
        }
        Err(err) => {
            let _ = fs::remove_file(paths::extended(&partial));
            Err(err)
        }
    }
}

fn bump_lastfm(shared: &Shared) {
    let mut live = shared.live.lock().expect("live");
    live.lastfm_done = live.lastfm_done.saturating_add(1);
}

fn bump_musicnn(shared: &Shared) {
    let mut live = shared.live.lock().expect("live");
    live.musicnn_done = live.musicnn_done.saturating_add(1);
}

fn bump_ok(shared: &Shared) {
    let mut live = shared.live.lock().expect("live");
    live.ok = live.ok.saturating_add(1);
}

fn sleep_cancel(shared: &Shared, total: std::time::Duration) {
    let start = std::time::Instant::now();
    while start.elapsed() < total {
        if cancelled(shared) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn copy_file(shared: &Shared, row_id: u64, source: &Path, dest: &Path, size: u64) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(paths::extended(parent)).map_err(|err| err.to_string())?;
    }
    if source == dest {
        set_fraction(shared, row_id, 1.0);
        return Ok(());
    }
    let mut input = fs::File::open(paths::extended(source)).map_err(|err| err.to_string())?;
    let mut output = fs::File::create(paths::extended(dest)).map_err(|err| err.to_string())?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut copied = 0u64;
    loop {
        if cancelled(shared) {
            drop(output);
            let _ = fs::remove_file(paths::extended(dest));
            return Err("Stopped.".into());
        }
        let n = input.read(&mut buf).map_err(|err| err.to_string())?;
        if n == 0 {
            break;
        }
        output.write_all(&buf[..n]).map_err(|err| err.to_string())?;
        copied += n as u64;
        {
            let mut live = shared.live.lock().expect("live");
            live.current_bytes = copied;
            if let Some(row) = live.rows.iter_mut().find(|row| row.id == row_id) {
                row.copied = copied;
                row.size = size.max(copied);
                row.fraction = if size > 0 {
                    (copied as f64 / size as f64).clamp(0.0, 1.0)
                } else {
                    1.0
                };
            }
        }
    }
    output.flush().map_err(|err| err.to_string())?;
    Ok(())
}

fn rows_for(groups: &[Group]) -> (Vec<WorkRow>, u32) {
    let mut rows = Vec::new();
    for group in groups {
        for file in &group.junk {
            rows.push(WorkRow {
                id: row_id(&file.source),
                name: file.name.clone(),
                size: file.size,
                copied: 0,
                fraction: 0.0,
                kind: RowKind::Queued,
                note: String::new(),
            });
        }
        for item in &group.items {
            rows.push(WorkRow {
                id: row_id(&item.source),
                name: format!("{}.{}", item.stem, item.extension),
                size: item.size,
                copied: 0,
                fraction: 0.0,
                kind: RowKind::Queued,
                note: String::new(),
            });
        }
    }
    let jobs = rows.len() as u32;
    (rows, jobs)
}

fn row_id(path: &Path) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for byte in path.to_string_lossy().bytes() {
        value ^= byte as u64;
        value = value.wrapping_mul(0x100_0000_01b3);
    }
    value
}

fn set_kind(shared: &Shared, id: u64, kind: RowKind, note: &str) {
    let mut live = shared.live.lock().expect("live");
    if let Some(row) = live.rows.iter_mut().find(|row| row.id == id) {
        row.kind = kind;
        if !note.is_empty() {
            row.note = note.to_string();
        }
    }
}

fn set_fraction(shared: &Shared, id: u64, fraction: f64) {
    let mut live = shared.live.lock().expect("live");
    if let Some(row) = live.rows.iter_mut().find(|row| row.id == id) {
        row.fraction = fraction.clamp(0.0, 1.0);
        live.current_bytes = (row.size as f64 * row.fraction) as u64;
    }
}

fn remove_done(shared: &Shared, id: u64, size: u64) {
    let mut live = shared.live.lock().expect("live");
    if let Some(index) = live.rows.iter().position(|row| row.id == id) {
        live.rows.remove(index);
    }
    live.ok = live.ok.saturating_add(1);
    live.banked_bytes = live.banked_bytes.saturating_add(size);
    live.current_bytes = 0;
}

fn fail(shared: &Shared, id: u64, message: &str) {
    let mut live = shared.live.lock().expect("live");
    if let Some(row) = live.rows.iter_mut().find(|row| row.id == id) {
        row.kind = RowKind::Failed;
        row.note = message.to_string();
    }
    live.bad = live.bad.saturating_add(1);
    live.current_bytes = 0;
}

fn pin(shared: &Shared, id: u64, message: &str) {
    let mut live = shared.live.lock().expect("live");
    if let Some(row) = live.rows.iter_mut().find(|row| row.id == id) {
        row.kind = RowKind::Pinned;
        row.note = message.to_string();
        let index = live.rows.iter().position(|row| row.id == id);
        let insert = live.rows.iter().position(|row| row.kind != RowKind::Pinned).unwrap_or(live.rows.len());
        if let Some(index) = index {
            if index != insert && index > insert {
                let row = live.rows.remove(index);
                let insert = insert.min(live.rows.len());
                live.rows.insert(insert, row);
            }
        }
    }
    live.ok = live.ok.saturating_add(1);
    live.current_bytes = 0;
}

fn cancelled(shared: &Shared) -> bool {
    shared.cancel.load(Ordering::Relaxed)
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(paths::extended(path)).map(|meta| meta.len()).unwrap_or(0)
}

fn tail(log: &str) -> String {
    let line = log
        .lines()
        .rev()
        .find(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with("out_time") && !t.starts_with("progress=") && !t.starts_with("bitrate=")
        })
        .unwrap_or("The conversion failed.");
    line.trim().to_string()
}

fn run_command<F>(shared: &Shared, exe: &Path, args: &[String], on_line: F) -> Result<RunOut, String>
where
    F: FnMut(&str) + Send + 'static,
{
    if cancelled(shared) {
        return Err("Stopped.".into());
    }
    let mut command = Command::new(exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command
        .spawn()
        .map_err(|err| format!("Could not start {}: {err}", exe.display()))?;
    let stdout = child.stdout.take().ok_or_else(|| "No output pipe.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "No error pipe.".to_string())?;
    *shared.child.lock().expect("child") = Some(child);
    let log = Arc::new(Mutex::new(String::new()));
    let log_out = log.clone();
    let log_err = log.clone();
    let callback = Arc::new(Mutex::new(on_line));
    let callback_out = callback.clone();
    let stdout_thread = thread::spawn(move || {
        read_lines(stdout, move |line| {
            callback_out.lock().expect("progress")(&line);
            push_log(&log_out, &line);
        });
    });
    let stderr_thread = thread::spawn(move || {
        read_lines(stderr, move |line| {
            push_log(&log_err, &line);
        });
    });
    let status = wait_child(shared);
    *shared.child.lock().expect("child") = None;
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    if cancelled(shared) {
        return Err("Stopped.".into());
    }
    let code = status.and_then(|status| status.code()).unwrap_or(1);
    let log_text = log.lock().expect("log").clone();
    Ok(RunOut { code, log: log_text })
}

/// Wait for the running child without holding the lock the whole time, so
/// `kill_child` (Cancel) can get in and stop it.
fn wait_child(shared: &Shared) -> Option<std::process::ExitStatus> {
    loop {
        {
            let mut slot = shared.child.lock().expect("child");
            let child = slot.as_mut()?;
            match child.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => {}
                Err(_) => return child.wait().ok(),
            }
        }
        thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Read lines until the pipe closes. Lines that are not valid UTF-8 are kept
/// (with replacement characters) instead of ending the read, so progress keeps
/// updating and the log keeps ffmpeg's error text.
fn read_lines<R, F>(reader: R, mut on_line: F)
where
    R: Read,
    F: FnMut(String),
{
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => on_line(String::from_utf8_lossy(&line).trim_end_matches(['\r', '\n']).to_string()),
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

fn push_log(log: &Mutex<String>, line: &str) {
    let mut text = log.lock().expect("log");
    if text.len() < 256 * 1024 {
        text.push_str(line);
        text.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Options;
    use crate::tools;

    fn test_bundle() -> Option<crate::tools::Bundle> {
        if let Ok(bundle) = tools::load_bundle() {
            return Some(bundle);
        }
        let dir = PathBuf::from(r"C:\ffmpeg\bin");
        let ffmpeg = dir.join("ffmpeg.exe");
        let ffprobe = dir.join("ffprobe.exe");
        if ffmpeg.is_file() && ffprobe.is_file() {
            Some(crate::tools::Bundle { ffmpeg, ffprobe })
        } else {
            None
        }
    }

    fn ff(bundle: &crate::tools::Bundle, args: &[&str], out: &Path) {
        let status = Command::new(&bundle.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(args)
            .arg(out)
            .status()
            .expect("ffmpeg");
        assert!(status.success(), "could not make {}", out.display());
    }

    /// Streams of a file as ffprobe JSON.
    fn streams(bundle: &crate::tools::Bundle, file: &Path) -> Vec<serde_json::Value> {
        let out = Command::new(&bundle.ffprobe)
            .args(["-v", "quiet", "-print_format", "json", "-show_streams", "-show_format"])
            .arg(file)
            .output()
            .expect("ffprobe");
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        json["streams"].as_array().cloned().unwrap_or_default()
    }

    fn format_tags(bundle: &crate::tools::Bundle, file: &Path) -> serde_json::Value {
        let out = Command::new(&bundle.ffprobe)
            .args(["-v", "quiet", "-print_format", "json", "-show_format"])
            .arg(file)
            .output()
            .expect("ffprobe");
        let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        json["format"]["tags"].clone()
    }

    /// Width of the attached picture, and the number of other video streams.
    /// `Folder/File` under the output root, with Windows separators.
    fn out_file(root: &Path, name: &str) -> PathBuf {
        name.split('/').fold(root.to_path_buf(), |path, part| path.join(part))
    }

    fn cover_and_video(bundle: &crate::tools::Bundle, file: &Path) -> (Option<i64>, usize) {
        let mut cover = None;
        let mut video = 0;
        for stream in streams(bundle, file) {
            if stream["codec_type"] != "video" {
                continue;
            }
            if stream["disposition"]["attached_pic"] == 1 {
                assert_eq!(stream["codec_name"], "mjpeg", "cover must be JPEG");
                cover = stream["width"].as_i64();
            } else {
                video += 1;
            }
        }
        (cover, video)
    }

    #[test]
    fn converts_audio_and_video_to_opus_m4a() {
        let Some(bundle) = test_bundle() else {
            return;
        };
        let keep = std::env::var_os("OPUS_CONVERT_KEEP_TEST").is_some();
        let root = std::env::temp_dir().join("opus-convert-smoke");
        let input = root.join("in");
        let album = input.join("Album");
        let solo = input.join("Solo");
        let output = root.join("out");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&album).unwrap();
        fs::create_dir_all(&solo).unwrap();
        fs::write(album.join("notes.txt"), "cover notes").unwrap();
        // Sizes tell the pictures apart: embedded 300, folder cover 200,
        // sidecar 120, a frame of the video would be 640.
        let blue = root.join("blue.jpg");
        let green = root.join("green.png");
        ff(&bundle, &["-f", "lavfi", "-i", "color=c=blue:s=300x300:d=1", "-frames:v", "1"], &blue);
        ff(&bundle, &["-f", "lavfi", "-i", "color=c=green:s=300x300:d=1", "-frames:v", "1"], &green);
        ff(&bundle, &["-f", "lavfi", "-i", "color=c=yellow:s=200x200:d=1", "-frames:v", "1"], &album.join("cover.jpg"));
        ff(&bundle, &["-f", "lavfi", "-i", "color=c=red:s=120x120:d=1", "-frames:v", "1"], &solo.join("Clip.jpg"));
        let blue_s = blue.display().to_string();
        let green_s = green.display().to_string();
        ff(&bundle, &["-f", "lavfi", "-i", "sine=frequency=440:duration=2", "-i", &blue_s,
            "-map", "0:a", "-map", "1:v", "-c:a", "libmp3lame", "-b:a", "192k", "-c:v", "copy",
            "-id3v2_version", "3", "-disposition:v", "attached_pic",
            "-metadata", "title=MP3 Song", "-metadata", "artist=Artist A; Artist B",
            "-metadata", "album=Test Album", "-metadata", "album_artist=Album Artist",
            "-metadata", "track=3/12", "-metadata", "disc=1/2", "-metadata", "date=2024",
            "-metadata", "genre=Techno", "-metadata", "comment=mp3 comment", "-metadata", "composer=Comp"],
            &album.join("Mp3.mp3"));
        ff(&bundle, &["-f", "lavfi", "-i", "sine=frequency=550:duration=2", "-i", &green_s,
            "-map", "0:a", "-map", "1:v", "-c:a", "flac", "-c:v", "copy", "-disposition:v", "attached_pic",
            "-metadata", "TITLE=FLAC Song", "-metadata", "ARTIST=Flac Artist", "-metadata", "ALBUM=Test Album",
            "-metadata", "ALBUMARTIST=Album Artist", "-metadata", "TRACKNUMBER=4", "-metadata", "DISCNUMBER=1",
            "-metadata", "DATE=2023-05-01", "-metadata", "GENRE=Jazz", "-metadata", "COMMENT=flac comment"],
            &album.join("Flac.flac"));
        ff(&bundle, &["-f", "lavfi", "-i", "sine=frequency=660:duration=2", "-c:a", "pcm_s16le",
            "-metadata", "title=WAV Song", "-metadata", "artist=Wav Artist"], &album.join("Wav.wav"));
        ff(&bundle, &["-f", "lavfi", "-i", "sine=frequency=770:duration=2", "-c:a", "libopus", "-b:a", "96k",
            "-metadata", "title=Opus Song", "-metadata", "artist=Opus Artist", "-metadata", "album=Test Album",
            "-metadata", "track=5", "-metadata", "date=2022", "-metadata", "genre=Music"], &album.join("Opus.opus"));
        ff(&bundle, &["-f", "lavfi", "-i", "testsrc=s=640x360:d=2:r=25", "-f", "lavfi", "-i", "sine=frequency=330:duration=2",
            "-i", &blue_s, "-map", "0:v", "-map", "1:a", "-map", "2:v", "-c:v", "libx264", "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "128k", "-c:v:1", "mjpeg", "-disposition:v:1", "attached_pic",
            "-metadata", "title=Video With Cover", "-metadata", "artist=Vid Artist", "-metadata", "album=Vid Album",
            "-metadata", "date=2021"], &album.join("Video.mp4"));
        ff(&bundle, &["-f", "lavfi", "-i", "testsrc=s=640x360:d=2:r=25", "-f", "lavfi", "-i", "sine=frequency=220:duration=2",
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac",
            "-metadata", "title=Video No Cover", "-metadata", "artist=Vid Artist 2"], &solo.join("Clip.mp4"));

        let opts = Options {
            input: input.display().to_string(),
            output: output.display().to_string(),
            ..Options::default()
        };
        let shared = Shared::new();
        crate::scan::scan(shared.clone(), bundle.ffprobe.clone(), opts.clone());
        let groups = shared.groups.lock().unwrap().take().unwrap();
        let shared = Shared::new();
        convert(shared.clone(), bundle.ffmpeg.clone(), bundle.ffprobe.clone(), opts, groups);
        assert_eq!(
            fs::read_to_string(output.join("Album").join("notes.txt")).unwrap(),
            "cover notes"
        );

        // (file, cover width, title, artist)
        let expect: [(&str, i64, &str, &str); 6] = [
            ("Album/Mp3.m4a", 300, "MP3 Song", "Artist A; Artist B"),
            ("Album/Flac.m4a", 300, "FLAC Song", "Flac Artist"),
            ("Album/Wav.m4a", 200, "WAV Song", "Wav Artist"),
            ("Album/Opus.m4a", 200, "Opus Song", "Opus Artist"),
            ("Album/Video.m4a", 300, "Video With Cover", "Vid Artist"),
            ("Solo/Clip.m4a", 120, "Video No Cover", "Vid Artist 2"),
        ];
        for (name, width, title, artist) in expect {
            let file = out_file(&output, name);
            assert!(file.is_file(), "missing {}", file.display());
            let info = crate::probe::probe(&bundle.ffprobe, &file).unwrap();
            assert_eq!(info.codec, "opus", "{name}");
            assert!(info.has_cover, "{name} has no cover");
            assert_eq!(info.kind, crate::model::Kind::Audio, "{name} still has video");
            assert_eq!(info.title, title, "{name}");
            assert_eq!(info.artists.join("; "), artist, "{name}");
            assert_eq!(cover_and_video(&bundle, &file), (Some(width), 0), "{name}");
            let tags = format_tags(&bundle, &file);
            assert_eq!(tags["major_brand"], "isom", "{name}");
        }
        let mp3 = format_tags(&bundle, &output.join("Album").join("Mp3.m4a"));
        for (key, value) in [("album", "Test Album"), ("album_artist", "Album Artist"), ("track", "3/12"),
            ("disc", "1/2"), ("date", "2024"), ("genre", "Techno"), ("comment", "mp3 comment"), ("composer", "Comp")] {
            assert_eq!(mp3[key], value, "mp3 {key}");
        }
        let flac = format_tags(&bundle, &output.join("Album").join("Flac.m4a"));
        for (key, value) in [("album", "Test Album"), ("album_artist", "Album Artist"), ("track", "4"),
            ("disc", "1"), ("date", "2023-05-01"), ("genre", "Jazz"), ("comment", "flac comment")] {
            assert_eq!(flac[key], value, "flac {key}");
        }
        let opus = format_tags(&bundle, &output.join("Album").join("Opus.m4a"));
        for (key, value) in [("album", "Test Album"), ("track", "5"), ("date", "2022"), ("genre", "Music")] {
            assert_eq!(opus[key], value, "opus {key}");
        }
        let video = format_tags(&bundle, &output.join("Album").join("Video.m4a"));
        // Album is the folder album the app writes for the whole group.
        assert_eq!(video["album"], "Test Album");
        assert_eq!(video["date"], "2021");

        // The genre write keeps the other tags and the cover.
        let shared = Shared::new();
        for (name, width, title, _) in expect {
            let file = out_file(&output, name);
            write_genre(&shared, &bundle.ffmpeg, &file, "Electronic").unwrap();
            let info = crate::probe::probe(&bundle.ffprobe, &file).unwrap();
            assert_eq!(info.genre, "Electronic", "{name}");
            assert_eq!(info.title, title, "{name}");
            assert_eq!(info.codec, "opus", "{name}");
            assert_eq!(cover_and_video(&bundle, &file), (Some(width), 0), "{name} after genre");
        }
        let mp3 = format_tags(&bundle, &output.join("Album").join("Mp3.m4a"));
        assert_eq!(mp3["track"], "3/12");
        assert_eq!(mp3["album_artist"], "Album Artist");

        // A second run keeps the finished files.
        assert!(finished(&bundle.ffprobe, &crate::model::Item {
            source: album.join("Wav.wav"),
            stem: "Wav".into(),
            original_stem: "Wav".into(),
            extension: "wav".into(),
            meta_title: String::new(),
            original_title: String::new(),
            artists: Vec::new(),
            original_artists: Vec::new(),
            kind: crate::model::Kind::Audio,
            size: 1,
            duration: 2.0,
            bitrate_kbps: None,
            codec: "pcm_s16le".into(),
            is_opus: false,
            is_lossless: true,
            has_video: false,
            attached_pic: false,
            track_number: None,
            opus_kbps: Some(192),
        }, &output.join("Album").join("Wav.m4a")));
        if !keep {
            let _ = fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn read_lines_keeps_reading_past_invalid_utf8() {
        let mut data = Vec::new();
        data.extend_from_slice(b"out_time_us=1000000\n");
        data.extend_from_slice(b"title=\xff\xfe bad \xc3\n");
        for n in 0..20_000 {
            data.extend_from_slice(format!("line {n}\r\n").as_bytes());
        }
        data.extend_from_slice(b"progress=end");
        let mut lines = Vec::new();
        read_lines(std::io::Cursor::new(data), |line| lines.push(line));
        assert_eq!(lines.len(), 20_003);
        assert_eq!(lines[0], "out_time_us=1000000");
        assert_eq!(lines[1], "title=\u{fffd}\u{fffd} bad \u{fffd}");
        assert_eq!(lines[2], "line 0");
        assert_eq!(lines.last().unwrap(), "progress=end");
    }

    /// Not a real test: the slow child for `cancel_stops_a_running_child`.
    #[test]
    #[ignore]
    fn slow_child_helper() {
        if std::env::var_os("OPUS_SLOW_CHILD").is_some() {
            thread::sleep(std::time::Duration::from_secs(30));
        }
    }

    #[test]
    fn cancel_stops_a_running_child() {
        let shared = Shared::new();
        let exe = std::env::current_exe().unwrap();
        std::env::set_var("OPUS_SLOW_CHILD", "1");
        let args: Vec<String> = ["slow_child_helper", "--ignored", "--exact", "run::tests::slow_child_helper"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let killer = Arc::clone(&shared);
        let started = std::time::Instant::now();
        let cancel = thread::spawn(move || {
            // Wait until the child is running, then press "Cancel".
            while killer.child.lock().unwrap().is_none() {
                thread::sleep(std::time::Duration::from_millis(20));
            }
            thread::sleep(std::time::Duration::from_millis(300));
            kill_child(&killer);
        });
        let result = run_command(&shared, &exe, &args, |_| {});
        cancel.join().unwrap();
        assert_eq!(result.err().as_deref(), Some("Stopped."));
        assert!(started.elapsed() < std::time::Duration::from_secs(10), "took {:?}", started.elapsed());
        assert!(shared.child.lock().unwrap().is_none());
    }
}
