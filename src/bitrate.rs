//! Opus bitrate from the source codec and its average bitrate.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecClass {
    Opus,
    Lossless,
    Aac,
    Vorbis,
    Mp3,
    Ac3,
    Dts,
    Other,
}

pub fn classify(codec: &str) -> CodecClass {
    match codec.to_ascii_lowercase().as_str() {
        "opus" => CodecClass::Opus,
        "flac" | "alac" | "ape" | "wavpack" | "tta" | "truehd" | "mlp"
        | "pcm_s16le" | "pcm_s16be" | "pcm_s24le" | "pcm_s24be" | "pcm_s32le" | "pcm_s32be"
        | "pcm_f32le" | "pcm_f64le" | "pcm_u8" | "pcm_s8" | "pcm_s16le_planar" | "pcm_s24le_planar"
        | "dsd_lsbf" | "dsd_msbf" | "dsd_lsbf_planar" | "dsd_msbf_planar" => CodecClass::Lossless,
        "aac" | "mp4a" | "aac_latm" => CodecClass::Aac,
        "vorbis" | "libvorbis" => CodecClass::Vorbis,
        "mp3" | "mp2" | "mp1" | "wma" | "wmav1" | "wmav2" | "wmapro" => CodecClass::Mp3,
        "ac3" | "eac3" | "ac-3" => CodecClass::Ac3,
        "dts" | "dca" | "dts-hd" => CodecClass::Dts,
        _ => CodecClass::Other,
    }
}

pub fn is_lossless(codec: &str) -> bool {
    classify(codec) == CodecClass::Lossless
}

pub fn is_opus(codec: &str) -> bool {
    classify(codec) == CodecClass::Opus
}

/// Target Opus bitrate in kbps, or `None` when the audio should be copied.
pub fn opus_kbps(codec: &str, source_kbps: Option<u32>) -> Option<u32> {
    match classify(codec) {
        CodecClass::Opus => None,
        CodecClass::Lossless => Some(192),
        CodecClass::Aac | CodecClass::Vorbis => Some(aac_ladder(source_kbps.unwrap_or(128))),
        CodecClass::Mp3 | CodecClass::Other => Some(mp3_ladder(source_kbps.unwrap_or(192))),
        CodecClass::Ac3 => Some(ac3_ladder(source_kbps.unwrap_or(384))),
        CodecClass::Dts => Some(192),
    }
}

fn mp3_ladder(kbps: u32) -> u32 {
    if kbps <= 128 {
        96
    } else if kbps <= 192 {
        128
    } else if kbps <= 256 {
        160
    } else {
        192
    }
}

fn aac_ladder(kbps: u32) -> u32 {
    if kbps <= 128 {
        96
    } else if kbps <= 192 {
        128
    } else {
        160
    }
}

fn ac3_ladder(kbps: u32) -> u32 {
    if kbps <= 192 {
        128
    } else if kbps <= 384 {
        160
    } else {
        192
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_is_copied() {
        assert_eq!(opus_kbps("opus", Some(160)), None);
    }

    #[test]
    fn lossless_is_the_top_of_the_ladder() {
        assert_eq!(opus_kbps("flac", Some(900)), Some(192));
        assert_eq!(opus_kbps("pcm_s24le", None), Some(192));
        assert_eq!(opus_kbps("alac", Some(700)), Some(192));
    }

    #[test]
    fn mp3_follows_the_library_ladder() {
        assert_eq!(opus_kbps("mp3", Some(128)), Some(96));
        assert_eq!(opus_kbps("mp3", Some(192)), Some(128));
        assert_eq!(opus_kbps("mp3", Some(256)), Some(160));
        assert_eq!(opus_kbps("mp3", Some(320)), Some(192));
        assert_eq!(opus_kbps("wma", Some(160)), Some(128));
    }

    #[test]
    fn aac_and_vorbis_stop_at_160() {
        assert_eq!(opus_kbps("aac", Some(256)), Some(160));
        assert_eq!(opus_kbps("vorbis", Some(96)), Some(96));
        assert_eq!(opus_kbps("aac", Some(160)), Some(128));
    }

    #[test]
    fn surround_tracks_use_their_own_map() {
        assert_eq!(opus_kbps("ac3", Some(192)), Some(128));
        assert_eq!(opus_kbps("ac3", Some(448)), Some(192));
        assert_eq!(opus_kbps("dts", Some(1509)), Some(192));
    }
}
