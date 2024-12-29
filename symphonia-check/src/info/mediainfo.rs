use std::process::{Command, Stdio};

use serde::Deserialize;
use serde_json::Value;
use symphonia::core::codecs::audio::well_known::*;
use symphonia::core::codecs::audio::{AudioCodecParameters, CODEC_ID_NULL_AUDIO};
use symphonia::core::codecs::subtitle::well_known::*;
use symphonia::core::codecs::subtitle::{SubtitleCodecParameters, CODEC_ID_NULL_SUBTITLE};
use symphonia::core::codecs::video::well_known::*;
use symphonia::core::codecs::video::{VideoCodecParameters, CODEC_ID_NULL_VIDEO};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Result;
use symphonia::core::formats::well_known::*;
use symphonia::core::formats::{
    FormatInfo, FormatReader, Packet, SeekMode, SeekTo, SeekedTo, Track, FORMAT_ID_NULL,
};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::Metadata;

use crate::InfoTestOptions;

use super::get_ref_decoder_output;

/// "media" json node
#[derive(Deserialize, Debug)]
struct Mediainfo {
    #[serde(rename = "track")]
    tracks: Vec<MediainfoTrack>,
}

#[derive(Deserialize, Debug)]
struct MediainfoTrack {
    #[serde(rename = "@type")]
    track_type: String,

    #[serde(rename = "ID")]
    id: Option<String>,

    #[serde(rename = "Format")]
    format: Option<String>,

    #[serde(rename = "Format_Profile")]
    format_profile: Option<String>,

    #[serde(rename = "CodecID")]
    codec_id: Option<String>,
}

struct MediainfoFormatReader {
    format_info: FormatInfo,
    tracks: Vec<Track>,
}

impl FormatReader for MediainfoFormatReader {
    fn format_info(&self) -> &FormatInfo {
        &self.format_info
    }

    fn metadata(&mut self) -> Metadata<'_> {
        todo!()
    }

    fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    fn seek(&mut self, _: SeekMode, _: SeekTo) -> Result<SeekedTo> {
        unreachable!()
    }

    fn next_packet(&mut self) -> Result<Option<Packet>> {
        unreachable!()
    }

    fn into_inner<'s>(self: Box<Self>) -> MediaSourceStream<'s>
    where
        Self: 's,
    {
        unreachable!()
    }
}

pub fn build_mediainfo_command(path: &str) -> Command {
    let mut cmd = Command::new("mediainfo");

    // Pipe errors to null.
    cmd.arg("-f").arg("--Output=JSON").arg(path).stdout(Stdio::piped()).stderr(Stdio::null());

    cmd
}

pub fn get_mediainfo_format(opts: &InfoTestOptions) -> Result<Box<dyn FormatReader>> {
    let mediainfo_output = get_ref_decoder_output(opts)?;
    let value: Value = serde_json::from_str(mediainfo_output.as_str()).unwrap();
    let media_node = value.get("media").unwrap();
    let data: Mediainfo = serde_json::from_value(media_node.clone()).unwrap();

    // contains general information about the file
    let general = &data.tracks[0];

    let first = &data.tracks[1];

    let format_info = get_format_info(general, first);
    let mut tracks = Vec::new();
    add_tracks(&mut tracks, &data)?;

    Ok(Box::new(MediainfoFormatReader { format_info, tracks }))
}

fn add_tracks(tracks: &mut Vec<Track>, data: &Mediainfo) -> Result<()> {
    // first track from the mediainfo contains general information about the file and is skipped for comparison
    for tr in data.tracks.iter().skip(1) {
        add_track(tracks, tr);
    }
    Ok(())
}

fn add_track(tracks: &mut Vec<Track>, tr: &MediainfoTrack) {
    let mut skip_track = false;

    let id = if let Some(id) = &tr.id { id.parse::<u32>().unwrap() } else { 0 };
    let codec_params = match tr.track_type.as_str() {
        "Video" => get_v_codec_params(tr, id),
        "Audio" => get_a_codec_params(tr, id),
        "Text" => get_s_codec_params(tr, id),
        "Menu" | "Image" => {
            skip_track = true;
            None
        }
        _ => None,
    };

    if !skip_track {
        let mut track = Track::new(id);
        if let Some(codec_params) = codec_params {
            track.with_codec_params(codec_params);
        }

        tracks.push(track);
    }
}

fn get_v_codec_params(tr: &MediainfoTrack, id: u32) -> Option<CodecParameters> {
    if let Some(format) = &tr.format {
        let codec = match format.as_str() {
            "HEVC" => CODEC_ID_HEVC,
            "MPEG-4 Visual" => CODEC_ID_MPEG4,
            "AVC" => CODEC_ID_H264,
            "AV1" => CODEC_ID_AV1,
            "VP9" => CODEC_ID_VP9,
            other => {
                println!("mediainfo: symphonia doesn't detect video codec: \"{}\", with codec_id: {:?} for track_id: {}", other, tr.codec_id, id);
                CODEC_ID_NULL_VIDEO
            }
        };
        Some(CodecParameters::Video(VideoCodecParameters { codec, ..Default::default() }))
    }
    else {
        Some(CodecParameters::Video(Default::default()))
    }
}

fn get_a_codec_params(tr: &MediainfoTrack, tr_id: u32) -> Option<CodecParameters> {
    if let Some(format) = &tr.format {
        let codec = match format.as_str() {
            "AAC" => CODEC_ID_AAC,
            "AC-3" => CODEC_ID_AC3,
            "E-AC-3" => CODEC_ID_EAC3,
            "DTS" => CODEC_ID_DCA,
            "MLP FBA" => CODEC_ID_TRUEHD,
            "FLAC" => CODEC_ID_FLAC,
            "Opus" => CODEC_ID_OPUS,
            "MPEG Audio" => {
                if let Some(format_profile) = &tr.format_profile {
                    match format_profile.as_str() {
                        "Layer 1" => CODEC_ID_MP1,
                        "Layer 2" => CODEC_ID_MP2,
                        "Layer 3" => CODEC_ID_MP3,
                        other => {
                            println!(
                                "mediainfo: symphonia doesn't detect layer for MPEG Audio: {} for track_id: {}",
                                other,
                                tr_id
                            );
                            CODEC_ID_NULL_AUDIO
                        }
                    }
                }
                else {
                    println!("mediainfo: cannot find \"Format_Profile\" for \"MPEG Audio\" for track_id: {}", tr_id);
                    CODEC_ID_NULL_AUDIO
                }
            }
            other => {
                println!("mediainfo: symphonia doesn't detect audio codec: {}, with codec_id: {:?} for track_id: {}", other, tr.codec_id, tr_id);
                CODEC_ID_NULL_AUDIO
            }
        };
        Some(CodecParameters::Audio(AudioCodecParameters { codec, ..Default::default() }))
    }
    else {
        Some(CodecParameters::Video(Default::default()))
    }
}

fn get_s_codec_params(tr: &MediainfoTrack, id: u32) -> Option<CodecParameters> {
    if let Some(format) = &tr.format {
        let codec = match format.as_str() {
            "Timed Text" => CODEC_ID_MOV_TEXT,
            "UTF-8" => CODEC_ID_TEXT_UTF8,
            "ASS" => CODEC_ID_ASS,
            "PGS" => CODEC_ID_HDMV_PGS,
            other => {
                println!("mediainfo: symphonia doesn't detect subtitle codec: {}, with codec_id: {:?} for track_id: {}", other, tr.codec_id, id);
                CODEC_ID_NULL_SUBTITLE
            }
        };
        Some(CodecParameters::Subtitle(SubtitleCodecParameters { codec, ..Default::default() }))
    }
    else {
        Some(CodecParameters::Video(Default::default()))
    }
}

fn get_format_info(general: &MediainfoTrack, first: &MediainfoTrack) -> FormatInfo {
    match &general.format {
        Some(format) => {
            let (format, short_name) = match format.as_str() {
                "MPEG-4" => (FORMAT_ID_ISOMP4, "isomp4"),
                "Matroska" => (FORMAT_ID_MKV, "matroska"),
                "Ogg" => (FORMAT_ID_OGG, "ogg"),
                "FLAC" => (FORMAT_ID_FLAC, "flac"),
                "ADTS" => (FORMAT_ID_ADTS, "aac"),
                "CAF" => (FORMAT_ID_CAF, "caf"),
                "AIFF" => (FORMAT_ID_AIFF, "aiff"),
                "Wave" => (FORMAT_ID_WAVE, "wave"),
                "MPEG Audio" => {
                    // general track doesn't contain detailed information, assume that it is present in the first track
                    if let Some(format_profile) = &first.format_profile {
                        match format_profile.as_str() {
                            "Layer 1" => (FORMAT_ID_MP1, "mp1"),
                            "Layer 2" => (FORMAT_ID_MP2, "mp2"),
                            "Layer 3" => (FORMAT_ID_MP3, "mp3"),
                            other => {
                                println!(
                                    "mediainfo: symphonia doesn't detect layer for MPEG Audio: {}",
                                    other
                                );
                                (FORMAT_ID_NULL, "Unknown")
                            }
                        }
                    }
                    else {
                        println!("mediainfo: first track don't have \"Format_Profile\" for \"MPEG Audio\"");
                        (FORMAT_ID_NULL, "Unknown")
                    }
                }
                other => {
                    println!("mediainfo: symphonia doesn't detect track format: {}", other);
                    (FORMAT_ID_NULL, "Unknown")
                }
            };
            FormatInfo { format, short_name, long_name: "" }
        }
        _ => {
            println!("mediainfo: cannot find General \"Format\"");
            FormatInfo { format: FORMAT_ID_NULL, short_name: "Unknown", long_name: "" }
        }
    }
}
