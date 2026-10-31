# Opus Convert

Windows 2.03–style wizard that converts a folder tree to Opus audio in `.m4a`.

A built copy is [`Opus.Convert.exe`](https://github.com/oiuy-retro/opus-convert/releases/latest/download/Opus.Convert.exe) on the [latest release](https://github.com/oiuy-retro/opus-convert/releases/latest).

## What it does

- Picks an input folder and an output folder, then walks every subfolder.
- Encodes audio and video files to Opus in `.m4a`. Video files keep only their first audio track. Audio that is already Opus is remuxed, not re-encoded.
- Copies every other file (cover images, text files, and anything else that isn't music or video) to the output folder unchanged.
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

Album art is embedded in each output file as its cover image. Opus Convert uses the first image it finds, in this order:

1. Art already embedded in the source file
2. An image with the same name as the song, like `Song.jpg`, `Song.jpeg`, `Song.png` or `Song.webp`
3. `cover.jpg`, `cover.jpeg`, `cover.png` or `cover.webp` in the same folder
4. `folder.jpg`, `folder.jpeg`, `folder.png` or `folder.webp` in the same folder

If an image can't be used, it moves on to the next one. PNG and WebP images are converted to JPEG.

## Genre lookup

When **Look up genre** is ticked, genre lookup runs after every file has been converted and the tags you entered have been written. Each file is looked up in this order:

1. Last.fm `track.getTopTags` for the exact artist and title, if you have an API key.
2. [musicnn](https://github.com/jordipons/musicnn) for every file Last.fm doesn't match.

Up to two genres are written per file.

Enter your Last.fm API key in the **Last.fm key** field on the Options page. You can also put it in `lastfm.key` next to the exe (see `lastfm.key.example`; another file can be set with `lastfm_key_file` in [configuration.conf](#configuration)) or set `LASTFM_API_KEY`. The key is taken from `LASTFM_API_KEY` first, then the **Last.fm key** field, then the key file. Without a key, every file goes to musicnn.

musicnn needs [Python](https://www.python.org/downloads/release/python-379/) (`python.exe`) with the packages listed in [THIRD_PARTY.md](THIRD_PARTY.md), plus `musicnn-one.py`, next to the exe. Python can also be somewhere else; see `python` in [configuration.conf](#configuration). With **Look up genre** ticked, Convert won't start if either is missing.

Choose what happens to a genre a file already has:

- **Replace existing genre**: look up every file and replace its genre tags with what's found.
- **Keep existing genre on audio**: keep existing genre tags and only fill in empty ones.
- **Keep existing genre on audio, except YouTube categories**: keep existing genre tags, but remove YouTube categories (as yt-dlp writes them) and fill in empty ones. If only YouTube categories were there, the file is looked up. These count as YouTube categories, in any letter case: Autos & Vehicles, Comedy, Education, Entertainment, Film & Animation, Game, Gaming, Howto & Style, Movies, Music, News & Politics, Nonprofits & Activism, People & Blogs, Pets & Animals, Science & Technology, Shows, Sports, Travel & Events.

Video files are always looked up, whichever option you choose.

## Run

1. Copy `Opus.Convert.exe` from the [latest release](https://github.com/oiuy-retro/opus-convert/releases/latest) to a folder.
2. Place [FFmpeg](https://www.gyan.dev/ffmpeg/builds/) `ffmpeg.exe` and `ffprobe.exe` beside it, or unpack the FFmpeg build into a folder beside it (Opus Convert finds `ffmpeg\bin\ffmpeg.exe`). See [Configuration](#configuration) for other places.
3. For genre lookup, put [Python 3.7.9](https://www.python.org/downloads/release/python-379/) `python.exe` with [musicnn](https://github.com/jordipons/musicnn) installed (packages in [THIRD_PARTY.md](THIRD_PARTY.md)), plus `musicnn-one.py`, next to the exe.
4. Start `Opus Convert.exe`. The working directory must be that folder.
5. For Last.fm genres, enter your API key in the **Last.fm key** field on the Options page.

Tested with FFmpeg `2026-10-19-git-2344518cc5-full_build` (Gyan), Python `3.7.9`, and musicnn `0.1.0`.

## Configuration

Opus Convert reads `configuration.conf` from the folder `Opus Convert.exe` is in. If the file is missing, Opus Convert writes one with every setting at its default and a comment explaining each one. [`configuration.conf.example`](configuration.conf.example) is the same file. If the exe's folder can't be written to (for example under Program Files), Opus Convert uses `%APPDATA%\Opus Convert\configuration.conf` instead, and copies a read-only `configuration.conf` from the exe's folder there the first time. If neither place works, Opus Convert runs with the defaults.

The file is plain text: `[section]` headers, `key = value` lines, and `#` comment lines. Keys and sections don't care about letter case, unknown ones are ignored, and a missing or empty key uses the default.

`[tools]` has one key per program: `ffmpeg`, `ffprobe` and `python`. Each can be:

- `auto` (the default): look next to the exe, then in each folder beside the exe (like `ffmpeg\ffmpeg.exe`), then in their `bin` folders (like `ffmpeg\bin\ffmpeg.exe`), then on `PATH`. For `python`, `venv\Scripts\python.exe` beside the exe is checked before `PATH`.
- `path`: only look on `PATH`.
- A path to the program or to the folder it is in. It can be relative to the exe's folder (not the current folder), use `/` or `\`, and use variables like `%USERPROFILE%`. Only that path is used. If the program isn't there, Opus Convert says so instead of looking anywhere else.

When a program can't be found, the message names the key, the configuration file, and the places that were checked.

`[folders]` has `input` and `output`: the folder each **Browse** button opens at. Picking a folder with **Browse** saves it there; only that one line in the file is changed. If the saved folder no longer exists, the dialog opens at the nearest parent folder that does, or where the box points if none does. These keys only decide where the dialog opens. The input and output folders a job uses still come from the boxes and are saved with your profiles in `%APPDATA%\Opus Convert\profiles.json` as before.

`[opus-convert]` has `lastfm_key_file`, the file the Last.fm key is read from (default `lastfm.key` next to the exe).

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
