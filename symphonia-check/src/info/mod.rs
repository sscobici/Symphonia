// Symphonia Check Tool
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]
// Justification: Fields on DecoderOptions and FormatOptions may change at any time, but
// symphonia-check doesn't want to be updated every time those fields change, therefore always fill
// in the remaining fields with default values.
#![allow(clippy::needless_update)]

use std::cmp::max;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use log::warn;
use mediainfo::{build_mediainfo_command, get_mediainfo_format};
use symphonia::core::codecs::audio::well_known::*;
use symphonia::core::codecs::audio::AudioCodecId;
use symphonia::core::codecs::subtitle::well_known::*;
use symphonia::core::codecs::subtitle::SubtitleCodecId;
use symphonia::core::codecs::video::well_known::*;
use symphonia::core::codecs::video::VideoCodecId;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::{unsupported_error, Result};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatReader, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
mod mediainfo;
use crate::{InfoTestDecoder, InfoTestOptions, RefProcess};

const EMPTY: &str = "---";

struct Line {
    title: String,
    exp: String,
    act: String,
}

impl Line {
    fn new(title: &str, exp: &str, act: &str) -> Self {
        Self { title: title.to_string(), exp: exp.to_string(), act: act.to_string() }
    }

    fn new_line(title: &str) -> Self {
        Self { title: title.to_string(), exp: "".to_string(), act: "".to_string() }
    }
}

fn get_ref_decoder_format(opts: &InfoTestOptions) -> Result<Box<dyn FormatReader>> {
    match opts.ref_decoder {
        InfoTestDecoder::Mediainfo => get_mediainfo_format(opts),
    }
}

/// returns a symphonia FormatReader object for a file
fn get_symphonia_format(opts: &InfoTestOptions) -> Result<Box<dyn FormatReader>> {
    let tgt_ms = Box::new(File::open(Path::new(&opts.input))?);
    let tgt_mss = MediaSourceStream::new(tgt_ms, Default::default());
    let tgt_fmt_opts = Default::default();
    let meta_opts: MetadataOptions = Default::default();
    let hint = Hint::new();
    let format = symphonia::default::get_probe().probe(&hint, tgt_mss, tgt_fmt_opts, meta_opts)?;
    Ok(format)
}

/// returns text output lines from the reference decoder
fn get_ref_decoder_output(opts: &InfoTestOptions) -> Result<String> {
    // Start the mediainfo process.
    let mut ref_process = match opts.ref_decoder {
        InfoTestDecoder::Mediainfo => RefProcess::try_spawn(build_mediainfo_command(&opts.input))?,
    };

    // Instantiate a reader for the mediainfo process output.
    let mut ref_reader = BufReader::new(ref_process.child.stdout.take().unwrap());

    // Read all output to multiline String
    let mut output = String::new();
    ref_reader.read_to_string(&mut output)?;

    Ok(output)
}

pub fn run_info(opts: InfoTestOptions) -> Result<()> {
    // consider ref decoder as expected value.
    // ref decoder output is processed and converted into symphonia FormatReader for comparison.
    let expected = get_ref_decoder_format(&opts)?;

    // consider symphonia detection as actual
    let actual = get_symphonia_format(&opts)?;

    // collect the differencies in lines to display them at the end
    let mut diff_lines = Vec::new();

    if expected.format_info().format != actual.format_info().format {
        // "General" section contains overall information about the file
        diff_lines.push(Line::new_line("General"));
        diff_lines.push(Line::new(
            "Format",
            expected.format_info().short_name,
            actual.format_info().short_name,
        ));
    }

    let expected_tracks = expected.tracks();
    let mut actual_tracks = Vec::new();
    actual_tracks.extend(actual.tracks());
    // sort tracks, before comparison, some files don't have tracks in usual order
    actual_tracks.sort_by_key(|track| match track.codec_params {
        Some(CodecParameters::Video(_)) => 0,    // Video first
        Some(CodecParameters::Audio(_)) => 1,    // Audio second
        Some(CodecParameters::Subtitle(_)) => 2, // Subtitle third
        Some(_) | None => 4,                     // None last
    });
    let max = max(expected_tracks.len(), actual_tracks.len());
    for i in 0..max {
        compare_tracks(
            &mut diff_lines,
            i + 1, // display track indexes, starting from 1
            expected_tracks.get(i),
            actual_tracks.get(i).map(|v| &**v),
        );
    }

    // when there are differences display Expected / Actual
    if !diff_lines.is_empty() {
        let mut lines = Vec::new();
        lines.push(Line::new("", "Expected:", "Actual:"));
        lines.extend(diff_lines);
        print_lines(&lines);
        return unsupported_error("info is different");
    }

    Ok(())
}

fn compare_tracks(
    lines: &mut Vec<Line>,
    index: usize,
    expected: Option<&Track>,
    actual: Option<&Track>,
) {
    let mut diff_lines = Vec::new();
    match (expected, actual) {
        // tracks present on both sides
        (Some(expected), Some(actual)) => {
            if !equal_codec_params_type(expected, actual) {
                diff_lines.push(Line::new(
                    "TrackType",
                    get_codec_type(index, &expected.codec_params),
                    get_codec_type(index, &actual.codec_params),
                ))
            }
            else {
                compare_track(&mut diff_lines, expected, actual);
            }
        }
        // only actual track is present
        (None, Some(actual)) => diff_lines.push(Line::new(
            "TrackType",
            EMPTY,
            get_codec_type(index, &actual.codec_params),
        )),
        // only expected track is present
        (Some(expected), None) => diff_lines.push(Line::new(
            "TrackType",
            get_codec_type(index, &expected.codec_params),
            EMPTY,
        )),
        _ => {}
    }

    if !diff_lines.is_empty() {
        match (expected, actual) {
            (Some(expected), Some(actual)) => {
                if equal_codec_params_type(expected, actual) {
                    lines.push(Line::new_line(
                        format!("{} {}", get_codec_type(index, &expected.codec_params), index)
                            .as_str(),
                    ));
                }
                else {
                    lines.push(Line::new_line(format!("Track {}", index).as_str()));
                }
            }
            _ => lines.push(Line::new_line(format!("Track {}", index).as_str())),
        }

        lines.extend(diff_lines);
    }
}

fn compare_track(diff_lines: &mut Vec<Line>, expected: &Track, actual: &Track) {
    if expected.id != actual.id {
        diff_lines.push(Line::new(
            "Id",
            expected.id.to_string().as_str(),
            actual.id.to_string().as_str(),
        ));
    }

    compare_codec_params(diff_lines, &expected.codec_params, &actual.codec_params);
}

fn compare_codec_params(
    diff_lines: &mut Vec<Line>,
    expected: &Option<CodecParameters>,
    actual: &Option<CodecParameters>,
) {
    match (expected, actual) {
        (Some(CodecParameters::Video(exp)), Some(CodecParameters::Video(act))) => {
            if exp.codec != act.codec {
                diff_lines.push(Line::new(
                    "Format",
                    get_v_codec(exp.codec),
                    get_v_codec(act.codec),
                ));
            }
        }
        (Some(CodecParameters::Audio(exp)), Some(CodecParameters::Audio(act))) => {
            if exp.codec != act.codec {
                diff_lines.push(Line::new(
                    "Format",
                    get_a_codec(exp.codec),
                    get_a_codec(act.codec),
                ));
            }
        }
        (Some(CodecParameters::Subtitle(exp)), Some(CodecParameters::Subtitle(act))) => {
            if exp.codec != act.codec {
                diff_lines.push(Line::new(
                    "Format",
                    get_s_codec(exp.codec),
                    get_s_codec(act.codec),
                ));
            }
        }
        _ => {}
    }
}

fn get_v_codec(codec: VideoCodecId) -> &'static str {
    match codec {
        CODEC_ID_MPEG4 => "MPEG4",
        CODEC_ID_H264 => "H264",
        CODEC_ID_HEVC => "HEVC",
        CODEC_ID_AV1 => "AV1",
        CODEC_ID_VP9 => "VP9",
        other => {
            println!("info: cannot detect VideoCodecId: {}", other);
            "Unknown"
        }
    }
}

fn get_a_codec(codec: AudioCodecId) -> &'static str {
    match codec {
        CODEC_ID_AAC => "AAC",
        CODEC_ID_AC3 => "AC3",
        CODEC_ID_EAC3 => "EAC3",
        CODEC_ID_DCA => "DCA",
        CODEC_ID_TRUEHD => "TRUEHD",
        CODEC_ID_FLAC => "FLAC",
        CODEC_ID_OPUS => "OPUS",
        CODEC_ID_MP3 => "MP3",
        other => {
            println!("info: cannot detect AudioCodecId: {}", other);
            "Unknown"
        }
    }
}

fn get_s_codec(codec: SubtitleCodecId) -> &'static str {
    match codec {
        CODEC_ID_MOV_TEXT => "MOV_TEXT",
        CODEC_ID_TEXT_UTF8 => "TEXT_UTF8",
        CODEC_ID_ASS => "ASS",
        CODEC_ID_HDMV_PGS => "HDMV_PGS",
        other => {
            println!("info: cannot detect SubtitleCodecId: {}", other);
            "Unknown"
        }
    }
}

fn equal_codec_params_type(expected: &Track, actual: &Track) -> bool {
    matches!(
        (&expected.codec_params, &actual.codec_params),
        (Some(CodecParameters::Video(_)), Some(CodecParameters::Video(_)))
            | (Some(CodecParameters::Audio(_)), Some(CodecParameters::Audio(_)))
            | (Some(CodecParameters::Subtitle(_)), Some(CodecParameters::Subtitle(_)))
            | (None, None)
    )
}

fn get_codec_type(index: usize, codec_params: &Option<CodecParameters>) -> &str {
    match codec_params {
        Some(CodecParameters::Video(_)) => "Video",
        Some(CodecParameters::Audio(_)) => "Audio",
        Some(CodecParameters::Subtitle(_)) => "Text",
        _ => {
            println!("info: cannot detect CodecParameters type, for track_id: {}", index);
            "Unknown"
        }
    }
}

fn print_lines(lines: &Vec<Line>) {
    for line in lines {
        if line.title.is_empty() {
            println!("                {:<20}\t{:<20}", line.exp, line.act);
        }
        else if line.exp.is_empty() && line.act.is_empty() {
            println!("{}", line.title);
        }
        else {
            println!("{:>14}: {:<20}\t{:<20}", line.title, line.exp, line.act);
        }
    }
}
