// Symphonia Check Tool
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![forbid(unsafe_code)]
// Justification: Fields on DecoderOptions and FormatOptions may change at any time, but
// symphonia-check doesn't want to be updated every time those fields change, therefore always fill
// in the remaining fields with default values.
#![allow(clippy::needless_update)]

use crate::{RefProcess, TestOptions, TestResult};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use extra_data::VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG;
use symphonia::core::codecs::audio::well_known::*;
use symphonia::core::codecs::subtitle::well_known::*;
use symphonia::core::codecs::video::well_known::*;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Result;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia_common::mpeg::video::DOVIDecoderConfigurationRecord;

pub fn build_mediainfo_command(path: &str) -> Command {
    let mut cmd = Command::new("mediainfo");
    cmd.arg("--Output=file://./symphonia-check/format.txt")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    cmd
}

pub fn run_test_mediainfo(path: &Path, opts: &TestOptions, result: &mut TestResult) -> Result<()> {
    if path.is_file() {
        // If it's a file, process it
        if is_video_file(path) {
            // 4. Begin check.
            let path_str = path.to_str().unwrap();
            println!("File: {}", path_str);
            result.n_files += 1;
            if let Err(err) = run_check_mediainfo(path_str, opts, result) {
                result.n_failed_files += 1;
                result.has_failed = true;
                println!("*** Error creating format: {}", err);
            };
        }
    }
    else if path.is_dir() {
        // If it's a directory, iterate over its contents
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            // Recursively process the entry
            run_test_mediainfo(&entry_path, opts, result)?;
        }
    }
    Ok(())
}

fn run_check_mediainfo(path: &str, opts: &TestOptions, result: &mut TestResult) -> Result<()> {
    let expected_lines = get_mediainfo_output(path, opts)?;
    let actual_lines = get_symphonia_mediainfo_output(path, opts)?;

    // Compare the actual and expected lines line by line
    let mut is_different = false;
    for (expected, actual) in expected_lines.iter().zip(actual_lines.iter()) {
        if expected != actual {
            // Mediainfo calculates the duration from the file atom, while Symphonia uses the duration from the longest track.
            // They may differ by a few milliseconds, so we compare the durations only up to whole seconds.
            // Mediainfo always includes milliseconds in the output, so we remove the last .999 milliseconds for comparison.
            if expected.starts_with("General") {
                if &expected[..expected.len() - 4] != actual {
                    is_different = true;
                    break;
                }
            }
            else {
                is_different = true;
                break;
            }
        }
    }

    // print differences
    if is_different {
        println!("    {:<50}\t{:<50}", "Expected:", "Actual:");
        for (expected, actual) in expected_lines.iter().zip(actual_lines.iter()) {
            if expected != actual {
                // cut duration milliseconds before comparison, duration should be at the end
                if expected.starts_with("General") {
                    if &expected[..expected.len() - 4] != actual {
                        println!("*** {:<50}\t{:<50}", expected, actual);
                    }
                }
                else {
                    println!("*** {:<50}\t{:<50}", expected, actual);
                }
            }
        }
        result.n_failed_files += 1;
        result.has_failed = true;
    }

    Ok(())
}

fn get_mediainfo_output(path: &str, opts: &TestOptions) -> Result<Vec<String>> {
    // Start the mediainfo process.
    let mut ref_process = RefProcess::try_spawn(opts.ref_decoder, opts.gapless, path)?;

    // Instantiate a reader for the mediainfo process output.
    let ref_reader = BufReader::new(ref_process.child.stdout.take().unwrap());

    // Read all output
    let lines: Vec<String> = ref_reader
        .lines()
        .take_while(|line| line.is_ok()) // Stop at the first error
        .filter_map(std::result::Result::ok) // Extract Ok values
        .collect();

    Ok(lines)
}

fn get_symphonia_mediainfo_output(path: &str, opts: &TestOptions) -> Result<Vec<String>> {
    // example of the mediainfo output can be obtained by running the following command from the project root folder
    // mediainfo --Output=file://./symphonia-check/format.txt  file_path

    // Instantiate a Symphonia format for the test target.
    let tgt_ms = Box::new(File::open(Path::new(path))?);
    let tgt_mss = MediaSourceStream::new(tgt_ms, Default::default());
    let tgt_fmt_opts = FormatOptions { enable_gapless: opts.gapless, ..Default::default() };
    let meta_opts: MetadataOptions = Default::default();
    let hint = Hint::new();
    let format = symphonia::default::get_probe().probe(&hint, tgt_mss, tgt_fmt_opts, meta_opts)?;

    // use io::Cursor for writing the output into the Vec<u8> using writeln!
    let mut output_in_bytes = Vec::new();
    let mut output = std::io::Cursor::new(&mut output_in_bytes);

    let gen_format = match format.format_info().short_name {
        "isomp4" => "MPEG-4",
        "matroska" => "Matroska",
        _ => "Unknown",
    };

    let mut time = None;
    for track in format.tracks() {
        if let (Some(tb), Some(duration)) = (track.time_base, track.num_frames) {
            let track_time = tb.calc_time(duration);
            if time.is_none() {
                time = Some(track_time);
            }
            else if let Some(t) = time {
                if track_time > t {
                    time = Some(track_time);
                }
            }
        }
    }

    let mut duration = String::from("Unknown");
    if let Some(t) = time {
        duration =
            format!("{:02}:{:02}:{:02}", t.seconds / 3600, t.seconds % 3600 / 60, t.seconds % 60);
    }
    // Write the general section
    writeln!(output, "General,Format:{},Duration:{}", gen_format, duration)?;

    let mut track_nr = 0;

    // Write video tracks
    for track in format.tracks() {
        if let Some(CodecParameters::Video(params)) = &track.codec_params {
            let format = match params.codec {
                CODEC_ID_H264 => "AVC",
                CODEC_ID_HEVC => "HEVC",
                CODEC_ID_AV1 => "AV1",
                CODEC_ID_VP9 => "VP9",
                _ => "Unknown",
            };
            let mut hdr = "";
            for extra_data in &params.extra_data {
                if extra_data.id == VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG {
                    if let Ok(config) = DOVIDecoderConfigurationRecord::read(&extra_data.data) {
                        hdr = match config.dv_bl_signal_compatibility_id {
                            1 | 6 => "Dolby Vision / SMPTE ST 2086",
                            _ => "Dolby Vision",
                        };
                    }
                }
            }
            writeln!(
                output,
                "Video {},Format:{},HDR:{},Width:{},Height:{}",
                track_nr,
                format,
                hdr,
                params.width.unwrap(),
                params.height.unwrap()
            )?;
            track_nr += 1;
        }
    }

    // Write audio tracks
    for track in format.tracks() {
        if let Some(CodecParameters::Audio(params)) = &track.codec_params {
            let format = match params.codec {
                CODEC_ID_AC3 => "AC-3",
                CODEC_ID_EAC3 => "E-AC-3",
                CODEC_ID_DCA => "DTS",
                CODEC_ID_TRUEHD => "MLP FBA",
                CODEC_ID_FLAC => "FLAC",
                CODEC_ID_OPUS => "Opus",
                CODEC_ID_MP3 => "MPEG Audio",
                _ => "Unknown",
            };
            writeln!(output, "Audio {},Format:{}", track_nr, format)?;
            track_nr += 1;
        }
    }

    // Write subtitle tracks
    for track in format.tracks() {
        if let Some(CodecParameters::Subtitle(params)) = &track.codec_params {
            let format = match params.codec {
                CODEC_ID_MOV_TEXT => "Timed Text",
                CODEC_ID_TEXT_UTF8 => "UTF-8",
                CODEC_ID_ASS => "ASS",
                CODEC_ID_HDMV_PGS => "PGS",
                _ => "Unknown",
            };
            writeln!(output, "Text {},Format:{}", track_nr, format)?;
            track_nr += 1;
        }
    }

    // Convert the Vec<u8> (which contains bytes) into Vec<String>
    let lines = String::from_utf8(output_in_bytes).unwrap().lines().map(String::from).collect();

    Ok(lines)
}

// Function to check if a file has a video extension
fn is_video_file(file_path: &Path) -> bool {
    let video_extensions = ["mp4", "mkv"];
    file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| video_extensions.contains(&ext))
        .unwrap_or(false)
}
