# Opus Convert

Windows 2.03–style wizard that converts a folder tree to Opus audio inside `.mkv`.

A built copy is [`Opus.Convert.exe`](https://github.com/oiuy-retro/opus-convert/releases/latest/download/Opus.Convert.exe) on the [latest release](https://github.com/oiuy-retro/opus-convert/releases/latest).

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

Album art is saved inside each output file as `cover.jpg`. Opus Convert uses the first image it finds, in this order:

1. An image with the same name as the song, like `Song.jpg`, `Song.jpeg`, `Song.png` or `Song.webp`
2. `cover.jpg`, `cover.jpeg`, `cover.png` or `cover.webp` in the same folder
3. `folder.jpg`, `folder.jpeg`, `folder.png` or `folder.webp` in the same folder
4. Art already embedded in the source file

PNG and WebP images are converted to JPEG.

## Genre lookup

When **Look up genre** is ticked, genre lookup runs after every file has been converted and the tags you entered have been written. Each file is looked up in this order:

1. Last.fm `track.getTopTags` for the exact artist and title, if you have an API key.
2. [musicnn](https://github.com/jordipons/musicnn) for every file Last.fm doesn't match.

Up to two genres are written per file.

Enter your Last.fm API key in the **Last.fm key** field on the Options page. It's saved to `%AppData%\Opus Convert\lastfm.key`. You can also put it in `lastfm.key` next to the exe (see `lastfm.key.example`) or set `LASTFM_API_KEY`. Without a key, every file goes to musicnn.

musicnn needs [Python](https://www.python.org/downloads/release/python-379/) (`python.exe`) with the packages listed in [THIRD_PARTY.md](THIRD_PARTY.md), plus `musicnn-one.py`, next to the exe. With **Look up genre** ticked, Convert won't start if either is missing.

Choose what happens to a genre a file already has:

- **Replace existing genre**: look up every file and replace its genre tags with what's found.
- **Keep existing genre on audio**: keep existing genre tags and only fill in empty ones.
- **Keep existing genre on audio, except YouTube categories**: keep existing genre tags, but remove YouTube categories (as yt-dlp writes them) and fill in empty ones. If only YouTube categories were there, the file is looked up. These count as YouTube categories, in any letter case: Autos & Vehicles, Comedy, Education, Entertainment, Film & Animation, Game, Gaming, Howto & Style, Movies, Music, News & Politics, Nonprofits & Activism, People & Blogs, Pets & Animals, Science & Technology, Shows, Sports, Travel & Events.

Video files are always looked up, whichever option you choose.

## Run

1. Copy `Opus.Convert.exe` from the [latest release](https://github.com/oiuy-retro/opus-convert/releases/latest) to a folder.
2. Place [FFmpeg](https://www.gyan.dev/ffmpeg/builds/) `ffmpeg.exe` and `ffprobe.exe` beside it.
3. For genre lookup, put [Python 3.7.9](https://www.python.org/downloads/release/python-379/) `python.exe` with [musicnn](https://github.com/jordipons/musicnn) installed (packages in [THIRD_PARTY.md](THIRD_PARTY.md)), plus `musicnn-one.py`, next to the exe.
4. Start `Opus Convert.exe`. The working directory must be that folder.
5. For Last.fm genres, enter your API key in the **Last.fm key** field on the Options page.

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
