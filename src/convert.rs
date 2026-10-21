//! ffmpeg argument lists and progress lines.

use std::path::Path;

use crate::model::{Item, Kind, Options};

pub fn copy_media(item: &Item, opts: &Options) -> bool {
    !opts.converts(item.kind) || item.kind == Kind::Junk
}

pub fn ffmpeg_args(
    source: &Path,
    dest: &Path,
    item: &Item,
    title: &str,
    artists: &[String],
    album: &str,
    cover: Option<&Path>,
) -> Vec<String> {
    let mut args = vec![
        "-hide_banner".into(),
        "-y".into(),
        "-fflags".into(),
        "+discardcorrupt".into(),
        "-err_detect".into(),
        "ignore_err".into(),
        "-i".into(),
        source.display().to_string(),
    ];
    if item.kind == Kind::Video {
        args.extend(["-map".into(), "0:v?".into(), "-c:v".into(), "copy".into()]);
        args.extend(["-map".into(), "0:s?".into(), "-c:s".into(), "copy".into()]);
    }
    args.extend(["-map".into(), "0:a?".into()]);
    if item.is_opus || item.opus_kbps.is_none() {
        args.extend(["-c:a".into(), "copy".into()]);
    } else {
        let kbps = item.opus_kbps.unwrap_or(128);
        args.extend([
            "-c:a".into(),
            "libopus".into(),
            "-b:a".into(),
            format!("{kbps}k"),
            "-vbr".into(),
            "on".into(),
            "-compression_level".into(),
            "10".into(),
            "-application".into(),
            "audio".into(),
        ]);
    }
    args.extend(["-map_metadata".into(), "0".into()]);
    if !title.is_empty() {
        args.extend(["-metadata".into(), format!("title={title}")]);
    }
    let artist = artists.join("; ");
    if !artist.is_empty() {
        args.extend(["-metadata".into(), format!("artist={artist}")]);
    }
    if !album.is_empty() {
        args.extend(["-metadata".into(), format!("album={album}")]);
    }
    if let Some(track) = item.track_number {
        args.extend(["-metadata".into(), format!("track={track}")]);
    }
    if let Some(cover) = cover {
        args.extend([
            "-attach".into(),
            cover.display().to_string(),
            "-metadata:s:t:0".into(),
            "mimetype=image/jpeg".into(),
            "-metadata:s:t:0".into(),
            "filename=cover.jpg".into(),
        ]);
    }
    args.push("-progress".into());
    args.push("pipe:1".into());
    args.push("-nostats".into());
    args.push(dest.display().to_string());
    args
}

pub fn genre_args(source: &Path, dest: &Path, genre: &str) -> Vec<String> {
    vec![
        "-hide_banner".into(),
        "-y".into(),
        "-i".into(),
        source.display().to_string(),
        "-map".into(),
        "0".into(),
        "-c".into(),
        "copy".into(),
        "-map_metadata".into(),
        "0".into(),
        "-metadata".into(),
        format!("genre={genre}"),
        dest.display().to_string(),
    ]
}

pub fn parse_out_time(line: &str) -> Option<f64> {
    if let Some(rest) = line.strip_prefix("out_time=") {
        return parse_clock(rest.trim());
    }
    if let Some(rest) = line.strip_prefix("out_time_ms=") {
        let value: f64 = rest.trim().parse().ok()?;
        if value > 10_000_000.0 {
            return Some(value / 1_000_000.0);
        }
        return Some(value / 1_000.0);
    }
    if let Some(rest) = line.strip_prefix("out_time_us=") {
        let value: f64 = rest.trim().parse().ok()?;
        return Some(value / 1_000_000.0);
    }
    None
}

fn parse_clock(text: &str) -> Option<f64> {
    let mut parts = text.split(':');
    let hours: f64 = parts.next()?.parse().ok()?;
    let minutes: f64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Item;
    use std::path::PathBuf;

    fn item(kind: Kind, opus: bool, kbps: Option<u32>) -> Item {
        Item {
            source: PathBuf::from("in.mp3"),
            stem: "Song".into(),
            original_stem: "Song".into(),
            extension: "mp3".into(),
            meta_title: "Song".into(),
            original_title: "Song".into(),
            artists: vec!["Queen".into()],
            original_artists: vec!["Queen".into()],
            kind,
            size: 1,
            duration: 10.0,
            bitrate_kbps: Some(192),
            codec: if opus { "opus".into() } else { "mp3".into() },
            is_opus: opus,
            is_lossless: false,
            has_video: kind == Kind::Video,
            attached_pic: false,
            track_number: Some(1),
            opus_kbps: kbps,
        }
    }

    #[test]
    fn encode_args_set_libopus_and_tags() {
        let args = ffmpeg_args(
            Path::new("in.mp3"),
            Path::new("out.mkv"),
            &item(Kind::Audio, false, Some(128)),
            "Song",
            &["Queen".into()],
            "News",
            None,
        );
        assert!(args.contains(&"libopus".into()));
        assert!(args.contains(&"128k".into()));
        assert!(args.iter().any(|arg| arg == "title=Song"));
        assert!(args.iter().any(|arg| arg == "artist=Queen"));
        assert!(args.iter().any(|arg| arg == "album=News"));
        assert!(!args.iter().any(|arg| arg == "attached_pic"));
        assert!(!args.iter().any(|arg| arg == "-attach"));
    }

    #[test]
    fn mkv_cover_is_an_attachment_named_cover_jpg() {
        let args = ffmpeg_args(
            Path::new("in.mp3"),
            Path::new("out.mkv"),
            &item(Kind::Audio, false, Some(128)),
            "Song",
            &[],
            "",
            Some(Path::new(r"C:\tmp\art.jpg")),
        );
        assert!(args.windows(2).any(|pair| pair[0] == "-attach" && pair[1].ends_with("art.jpg")));
        assert!(args.iter().any(|arg| arg == "filename=cover.jpg"));
        assert!(args.iter().any(|arg| arg == "mimetype=image/jpeg"));
        assert!(!args.iter().any(|arg| arg == "attached_pic"));
    }

    #[test]
    fn already_opus_is_copied() {
        let args = ffmpeg_args(
            Path::new("in.opus"),
            Path::new("out.mkv"),
            &item(Kind::Audio, true, None),
            "Song",
            &[],
            "",
            None,
        );
        let audio = args.windows(2).any(|pair| pair[0] == "-c:a" && pair[1] == "copy");
        assert!(audio);
        assert!(!args.iter().any(|arg| arg == "libopus"));
    }

    #[test]
    fn clock_progress_parses() {
        assert_eq!(parse_out_time("out_time=00:01:30.500000"), Some(90.5));
    }
}
