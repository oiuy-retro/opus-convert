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
    if let Some(cover) = cover {
        args.extend(["-i".into(), cover.display().to_string()]);
    }
    // Output is audio only. Video files keep their first audio track (the one
    // the bitrate was picked from); their video and subtitles are dropped.
    let audio = if item.kind == Kind::Video { "0:a:0?" } else { "0:a?" };
    args.extend(["-map".into(), audio.into()]);
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
    if cover.is_some() {
        args.extend([
            "-map".into(),
            "1:v:0".into(),
            "-c:v".into(),
            "copy".into(),
            "-disposition:v:0".into(),
            "attached_pic".into(),
        ]);
    }
    // MP4 keeps only file-level tags. Ogg/Opus sources keep theirs on the
    // audio stream, so pull those in too; file-level tags win.
    args.extend([
        "-map_metadata".into(),
        "0:s:a:0".into(),
        "-map_metadata".into(),
        "0".into(),
    ]);
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
    args.extend(["-f".into(), "mp4".into()]);
    args.push("-progress".into());
    args.push("pipe:1".into());
    args.push("-nostats".into());
    args.push(dest.display().to_string());
    args
}

pub fn genre_args(source: &Path, dest: &Path, genre: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![
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
    ];
    // ffmpeg picks the iPod muxer for .m4a, which cannot hold Opus.
    if is_m4a(dest) {
        args.extend(["-f".into(), "mp4".into()]);
    }
    args.push(dest.display().to_string());
    args
}

fn is_m4a(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("m4a"))
        .unwrap_or(false)
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

    fn pair(args: &[String], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| w[0] == a && w[1] == b)
    }

    #[test]
    fn encode_args_set_libopus_and_tags() {
        let args = ffmpeg_args(
            Path::new("in.mp3"),
            Path::new("out.m4a"),
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
        assert!(pair(&args, "-f", "mp4"));
        assert!(pair(&args, "-map", "0:a?"));
        assert!(!args.iter().any(|arg| arg == "attached_pic"));
        assert!(!args.iter().any(|arg| arg == "-attach"));
        assert!(!args.iter().any(|arg| arg.starts_with("track=")), "track comes from the source tags");
    }

    #[test]
    fn m4a_cover_is_an_attached_picture() {
        let args = ffmpeg_args(
            Path::new("in.mp3"),
            Path::new("out.m4a"),
            &item(Kind::Audio, false, Some(128)),
            "Song",
            &[],
            "",
            Some(Path::new(r"C:\tmp\art.jpg")),
        );
        assert!(pair(&args, "-i", r"C:\tmp\art.jpg"));
        assert!(pair(&args, "-map", "1:v:0"));
        assert!(pair(&args, "-c:v", "copy"));
        assert!(pair(&args, "-disposition:v:0", "attached_pic"));
        assert!(!args.iter().any(|arg| arg == "-attach"));
    }

    #[test]
    fn video_keeps_only_the_first_audio_track() {
        let args = ffmpeg_args(
            Path::new("in.mp4"),
            Path::new("out.m4a"),
            &item(Kind::Video, false, Some(128)),
            "Song",
            &[],
            "",
            None,
        );
        assert!(pair(&args, "-map", "0:a:0?"));
        assert!(!args.iter().any(|arg| arg == "0:v?" || arg == "0:s?"));
        assert!(!pair(&args, "-c:v", "copy"));
    }

    #[test]
    fn genre_write_forces_mp4_for_m4a_only() {
        let m4a = genre_args(Path::new("a.m4a"), Path::new("a.genre-partial.m4a"), "Jazz");
        assert!(pair(&m4a, "-f", "mp4"));
        assert!(m4a.iter().any(|arg| arg == "genre=Jazz"));
        let mp3 = genre_args(Path::new("a.mp3"), Path::new("a.genre-partial.mp3"), "Jazz");
        assert!(!mp3.iter().any(|arg| arg == "-f"));
    }

    #[test]
    fn already_opus_is_copied() {
        let args = ffmpeg_args(
            Path::new("in.opus"),
            Path::new("out.m4a"),
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
