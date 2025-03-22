use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr::{null, null_mut};

use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::get_probe;
use win::WinIAsyncReader;

mod win;
mod wrap;

/// # Safety
/// This function should receive a pointer to c string.
#[no_mangle]
pub unsafe extern "C" fn sm_io_mss_new_file(path: *mut c_char) -> *mut c_void {
    if path.is_null() {
        return null_mut();
    }

    let path = CStr::from_ptr(path).to_string_lossy().into_owned();
    let src = std::fs::File::open(path).expect("failed to open media");
    // Box MediaSourceStream to put structure on the heap
    Box::into_raw(Box::new(MediaSourceStream::new(Box::new(src), Default::default())))
        as *mut c_void
}

#[no_mangle]
pub extern "C" fn sm_io_mss_new_win_iasyncreader(iasyncreader: *mut c_void) -> *mut c_void {
    if iasyncreader.is_null() {
        return null_mut();
    }
    let reader = WinIAsyncReader::new(iasyncreader);
    let mss = MediaSourceStream::new(Box::new(reader), Default::default());
    Box::into_raw(Box::new(mss)) as *mut c_void
}

#[no_mangle]
pub extern "C" fn sm_probe(media_source_stream: *mut c_void) -> *mut c_void {
    if media_source_stream.is_null() {
        return null_mut();
    }

    let probe = get_probe();
    let hint = Default::default();
    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    unsafe {
        let mss = Box::from_raw(media_source_stream as *mut MediaSourceStream);
        if let Ok(result) = probe.probe(&hint, *mss, fmt_opts, meta_opts) {
            // Box<dyn Trait> pointer has two pointers behind
            // Box it to put two pointers (data and vtable) into a simple box that will have a single pointer behind
            Box::into_raw(Box::new(result)) as *mut c_void
        }
        else {
            null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sm_format_tracks(
    format: *mut c_void,
    tracks_len: *mut usize,
) -> *const wrap::Track {
    if format.is_null() || tracks_len.is_null() {
        return null_mut();
    }

    let format = Box::leak(Box::from_raw(format as *mut Box<dyn FormatReader>));
    let tracks: Vec<wrap::Track> = format
        .tracks()
        .iter()
        .map(|track| {
            let codec_params = match track.codec_params.as_ref() {
                Some(CodecParameters::Audio(_)) => wrap::CodecParameters {
                    codec_type: wrap::CodecType::Audio,
                    ..Default::default()
                },
                Some(CodecParameters::Video(params)) => {
                    let extra_data: Vec<wrap::VideoExtraData> = params
                        .extra_data
                        .iter()
                        .map(|x| wrap::VideoExtraData {
                            id: x.id,
                            data_len: x.data.len(),
                            data: x.data.as_ptr(),
                        })
                        .collect();
                    let ptr = extra_data.as_ptr();
                    std::mem::forget(extra_data);

                    wrap::CodecParameters {
                        codec_type: wrap::CodecType::Video,
                        video_params: wrap::VideoCodecParameters {
                            codec: params.codec,
                            profile: params.profile.map_or(Default::default(), |x| {
                                wrap::CodecProfile::new(x.get() as i32)
                            }),
                            level: params.level.unwrap_or_default(),
                            width: params.width.unwrap_or_default(),
                            height: params.height.unwrap_or_default(),
                            color_space: params
                                .color_space
                                .as_ref()
                                .map_or(null(), |cs| Box::into_raw(Box::new(cs.clone()))),
                            extra_data_len: params.extra_data.len(),
                            extra_data: ptr,
                        },
                    }
                }
                Some(CodecParameters::Subtitle(_)) => wrap::CodecParameters {
                    codec_type: wrap::CodecType::Subtitle,
                    ..Default::default()
                },
                _ => Default::default(),
            };

            let language = track.language.as_ref().map_or(null_mut(), |x| {
                CString::new(x.as_str()).ok().map_or(null_mut(), |c_string| c_string.into_raw())
            });

            wrap::Track {
                id: track.id,
                codec_params,
                language,
                time_base: track.time_base.unwrap(),
                num_frames: track.num_frames.unwrap_or_default(),
                start_ts: track.start_ts,
                delay: track.delay.unwrap_or_default(),
                padding: track.padding.unwrap_or_default(),
                flags: track.flags.bits(),
            }
        })
        .collect();

    *tracks_len = tracks.len();
    let ptr = tracks.as_ptr();
    std::mem::forget(tracks); // Prevent Rust from deallocating
    ptr
}

#[no_mangle]
pub extern "C" fn sm_format_next_packet(format: *mut c_void) -> *mut wrap::Packet {
    if format.is_null() {
        return null_mut();
    }

    unsafe {
        let format = Box::leak(Box::from_raw(format as *mut Box<dyn FormatReader>));
        if let Some(packet) = format.next_packet().expect("Cannot get next packet") {
            let wrap_packet = wrap::Packet {
                track_id: packet.track_id,
                ts: packet.ts,
                dur: packet.dur,
                trim_start: packet.trim_start,
                trim_end: packet.trim_end,
                data_len: packet.data.len(),
                data: Box::into_raw(packet.data) as *const u8,
            };
            Box::into_raw(Box::new(wrap_packet))
        }
        else {
            null_mut()
        }
    }
}
