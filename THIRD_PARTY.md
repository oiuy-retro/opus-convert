# Third-party software

Opus Convert links against or is meant to run next to the following projects. This repository has the Opus Convert sources; the built exe is on the [latest release](https://github.com/oiuy-retro/opus-convert/releases/latest). Neither includes FFmpeg, Python, or musicnn weights.

Tested with FFmpeg `2026-10-19-git-2344518cc5-full_build` (Gyan), Python `3.7.9`, and musicnn `0.1.0`.

## Rust crates (linked into the exe)

| Project | License | Use |
| --- | --- | --- |
| [windows-rs](https://github.com/microsoft/windows-rs) | MIT OR Apache-2.0 | Win32 UI, WinHTTP |
| [serde_json](https://github.com/serde-rs/json) | MIT OR Apache-2.0 | JSON profiles and musicnn output |
| [winres](https://github.com/mxre/winres) | MIT | Icon and manifest at build time |
| [Rust](https://www.rust-lang.org/) / LLVM | MIT / Apache-2.0 / NCSA | Compiler |

## Required beside the exe

| Project | License | Notes |
| --- | --- | --- |
| [FFmpeg](https://ffmpeg.org/) | LGPL / GPL, depending on the build | Encode, remux, embed covers, write genre tags. A Gyan full build (`--enable-gpl --enable-version3`) is GPL-3.0. Tested with `ffmpeg version 2026-09-10-git-fd7c73d01e-full_build` from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/). |

FFmpeg is a trademark of Fabrice Bellard / the FFmpeg project.

## Genre lookup

| Project | License | Notes |
| --- | --- | --- |
| [Last.fm API](https://www.last.fm/api) | Last.fm API terms | `track.getTopTags`. You supply your own API key. |
| [Python](https://www.python.org/) 3.7.9 | PSF License | Portable interpreter next to the exe. |
| [musicnn](https://github.com/jordipons/musicnn) 0.1.0 | MIT | Audio tagging. `MSD_musicnn_big` is not in the PyPI wheel; copy it from the upstream repo into `site-packages/musicnn/MSD_musicnn_big`. |
| [TensorFlow](https://github.com/tensorflow/tensorflow) 1.15.5 | Apache-2.0 | musicnn backend |
| [NumPy](https://numpy.org/) 1.16.6 | BSD | musicnn |
| [librosa](https://librosa.org/) 0.7.2 | ISC | musicnn |
| [SciPy](https://scipy.org/) 1.5.4 | BSD | musicnn |
| [numba](https://numba.pydata.org/) 0.48.0 | BSD | musicnn |
| [llvmlite](https://github.com/numba/llvmlite) 0.31.0 | BSD | musicnn |
| [protobuf](https://github.com/protocolbuffers/protobuf) 3.20.3 | BSD | TensorFlow |

`musicnn-one.py` in this repo is a small launcher for those two MSD models.

## Windows

Common Controls 6, Segoe UI, and the Explorer folder picker are part of Windows. This program requests them through a manifest and does not redistribute Windows.
