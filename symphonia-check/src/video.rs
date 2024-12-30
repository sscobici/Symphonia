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

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use log::warn;
use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::{decode_error, unsupported_error, Error, Result};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;

use crate::{get_codec_type, AudioTestDecoder, AudioTestOptions, RefProcess, VideoTestDecoder, VideoTestOptions};

const AUDIO: &str = "audio";
const VIDEO: &str = "video";

#[derive(Default)]
struct VideoTestResult {
    n_packets: u64,
    n_failed_packets: u64,
}

fn build_ffprobe_command(path: &str) -> Command {
    let mut cmd = Command::new("ffprobe");

    cmd.arg("-hide_banner")
        .arg("-show_packets")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null()); // Pipe errors to null.

    cmd
}

#[derive(Debug)]
struct FfprobePacket {
    codec_type: String,
    pts: u64,
    dts: u64,
}

struct PacketIterator<R: BufRead> {
    reader: R,
    buffer: String,
}

impl<R: BufRead> PacketIterator<R> {
    fn new(reader: R) -> Self {
        PacketIterator { reader, buffer: String::new() }
    }
}

impl<R: BufRead> Iterator for PacketIterator<R> {
    type Item = FfprobePacket;

    fn next(&mut self) -> Option<Self::Item> {
        let mut pts: u64 = 0;
        let mut dts: u64 = 0;
        let mut codec_type = String::new();
        loop {
            self.buffer.clear();
            if self.reader.read_line(&mut self.buffer).unwrap() == 0 {
                break;
            }

            let line = self.buffer.trim();

            if line.starts_with("[PACKET]") {
                pts = 0;
                dts = 0;
                codec_type.clear();
            } else if line.starts_with("[/PACKET]") {
                if codec_type == VIDEO {
                    return Some(FfprobePacket { codec_type, pts, dts });
                }
            } else if let Some((key, value)) = line.split_once('=') {
                match key {
                    "codec_type" => { codec_type = value.to_string(); }
                    "pts" => { pts = value.parse::<u64>().unwrap_or_default(); },
                    "dts" => { dts = value.parse::<u64>().unwrap_or_default(); },
                    _ => {},
                }
            }
        }
        None
    }
}

#[derive(Default)]
struct FlushStats {
    n_packets: u64,
    n_samples: u64,
}

struct DecoderInstance {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
}

fn run_test(path: &str, opts: &VideoTestOptions, result: &mut VideoTestResult) -> Result<()> {
    // open file with Symphonia
    let tgt_ms = Box::new(File::open(Path::new(&opts.input))?);
    let tgt_mss = MediaSourceStream::new(tgt_ms, Default::default());
    let tgt_fmt_opts = Default::default();
    let meta_opts: MetadataOptions = Default::default();
    let hint = Hint::new();
    let mut format = symphonia::default::get_probe().probe(&hint, tgt_mss, tgt_fmt_opts, meta_opts)?;


    let command = match opts.ref_decoder {
        VideoTestDecoder::Ffprobe => build_ffprobe_command(path),
    };

    // Start the ref decoder process.
    let mut ref_process = RefProcess::try_spawn(command)?;

    // Instantiate a iterator reader for the ref decoder process output.
    let packet_iterator = PacketIterator::new(BufReader::new(ref_process.child.stdout.take().unwrap()));

    let audio_track_id = format.first_track(TrackType::Audio).unwrap().id;

    for exp in packet_iterator {
        let act = loop {
            match format.next_packet() {
                Ok(Some(packet)) => {
                    if packet.track_id() == audio_track_id {
                        continue;
                    }
                    break packet;
                }
                Ok(None) => {
                    // Reached the end of the stream.
                    return decode_error("video: Symphonia reached end of file but reference decoder still have packets");
                }
                Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // WavReader will always return an UnexpectedEof when it ends because the
                    // reference decoder is piping the decoded audio and cannot write out the
                    // actual length of the media. Treat UnexpectedEof as the end of the stream.
                    return decode_error("video: Symphonia reached end of file wav, but reference decoder still have packets");
                }
                Err(err) => {
                    // A unrecoverable error occurred, halt decoding.
                    return Err(err);
                }
            }
        };

        result.n_packets += 1;

        let codec_param = match format.tracks().get(act.track_id() as usize - 1) {
            Some(tr) => &tr.codec_params,
            _ => &None,
        };

        let different = match (exp.codec_type.as_str(), codec_param) {
            (AUDIO, Some(CodecParameters::Audio(_))) |
            (VIDEO, Some(CodecParameters::Video(_))) => { 
                // valid conbinations, compare packet data
                exp.pts != act.pts || exp.dts != act.dts
            }
            _ => true,
        };

        if different {
            result.n_failed_packets += 1;
            println!("FAIL");
            println!("\tExpected: codec_type: {}, dts: {:<10} pts: {:<10}", exp.codec_type, exp.dts, exp.pts);
            println!("\t  Actual: codec_type: {}, dts: {:<10} pts: {:<10}", get_codec_type(act.track_id() as usize, codec_param), act.dts, act.pts);
        }
    }

    Ok(())
}

pub fn run_video(opts: VideoTestOptions) -> Result<()> {
    let mut res: VideoTestResult = Default::default();

    run_test(&opts.input, &opts, &mut res)?;

    if !opts.is_quiet {
        println!();
    }

    println!("Test Results");
    println!("=================================================");
    println!();
    println!("  Failed/Total Packets: {:>12}/{:>12}", res.n_failed_packets, res.n_packets);
    println!();

    if res.n_failed_packets == 0 {
        Ok(())
    }
    else {
        unsupported_error("Some packet didn't pass validation")
    }
}
