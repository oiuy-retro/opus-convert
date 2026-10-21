# Opus Convert

Windows Vista–style wizard that converts a folder tree to Opus audio inside `.mkv`.

A built copy is in [`dist/Opus Convert.exe`](dist/Opus%20Convert.exe). Put `ffmpeg.exe` and `ffprobe.exe` in the same folder as the program.

## What it does

- Picks an input folder and an output folder, then walks every subfolder.
- Encodes audio and video to Opus in Matroska. Video pictures are copied. Audio that is already Opus is remuxed, not re-encoded.
- Copies junk files (covers, text, and anything that is not audio or video) as they are.
- Wizard pages: Options, Names, Titles, Metadata, Convert.
- Profiles are stored in `%AppData%\Opus Convert\profiles.json`.

On Options you choose the profile, folders, whether to convert audio and/or video, and **Look up genre**. Names, Titles, and Metadata edit folder names, file names, titles, album, and artists.

## Bitrate

| Source | Mapping |
| --- | --- |
| Already Opus | copied |
| Lossless, DTS | 192k |
| MP3/WMA | ≤128 → 96k, 129–192 → 128k, 193–256 → 160k, 257+ → 192k |
| AAC/Vorbis | ≤128 → 96k, 129–192 → 128k, 193+ → 160k |
| AC-3 | ≤192 → 128k, 193–384 → 160k, 385+ → 192k |

## Cover art

Album art is a Matroska attachment named `cover.jpg` (`image/jpeg`). Sources: an embedded still, then sidecar `Song.jpg`/`png`/`webp`, then `cover.*`, then `folder.*`. WebP and PNG are converted to JPEG first.

## Genre lookup (optional)

When **Look up genre** is on, lookup runs after convert on the output files.

1. Last.fm `track.getTopTags` with autocorrect off, if a key is present.
2. Files Last.fm does not match go to [musicnn](https://github.com/jordipons/musicnn).

Provide a Last.fm API key as `lastfm.key` next to the exe (see `lastfm.key.example`) or the `LASTFM_API_KEY` environment variable. Without a key, Last.fm is skipped and musicnn still runs if Python is present.

musicnn needs a portable Python with the packages listed in [THIRD_PARTY.md](THIRD_PARTY.md), plus `musicnn-one.py` next to the exe.

Genre radios: replace existing genre; keep existing genre on audio; keep existing genre on audio except YouTube categories (Music, People & Blogs, and the rest).

## Run

1. Copy `dist/Opus Convert.exe` to a folder.
2. Place [FFmpeg](https://www.gyan.dev/ffmpeg/builds/) `ffmpeg.exe` and `ffprobe.exe` beside it.
3. Optional: `lastfm.key`, `python.exe` (with musicnn), and `musicnn-one.py`.
4. Start `Opus Convert.exe`. The working directory must be that folder.

Windows 10 or later. The window uses Vista Aero chrome and the Explorer folder picker.

## Build

This crate targets `x86_64-pc-windows-gnu` (`rust-toolchain.toml`).

```bat
cargo build --release
copy target\release\opus-convert.exe dist\"Opus Convert.exe"
```

Static CRT:

```bat
set RUSTFLAGS=-C target-feature=+crt-static
```

```bat
cargo test
```

## License

[MIT](LICENSE) for this program. Bundled and companion tools keep their own licenses; see [THIRD_PARTY.md](THIRD_PARTY.md).
