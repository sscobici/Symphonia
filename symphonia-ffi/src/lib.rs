use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr::null_mut;

use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::get_probe;
use wrap::Packet;

mod wrap;

// #[no_mangle]
// pub extern "C" fn sm_get_probe() -> *const Probe {
//     get_probe()
// }

/// # Safety
/// This function should receive a pointer to c string.
#[no_mangle]
pub unsafe extern "C" fn sm_io_media_source_stream_new_file(path: *mut c_char) -> *mut c_void {
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
pub extern "C" fn sm_format_next_packet(format: *mut c_void) -> *mut Packet {
    if format.is_null() {
        return null_mut();
    }

    unsafe {
        let mut format = Box::from_raw(format as *mut Box<dyn FormatReader>);
        if let Some(packet) = format.next_packet().expect("Cannot get next packet") {
            let wrap_packet = Packet {
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

// struct MyFormatReader;
// pub trait FormatReader2 {
//     fn read(&self) {
//         println!("Reading FormatReader2...");
//     }
// }

// impl FormatReader2 for MyFormatReader {
//     fn read(&self) {
//         println!("Reading MyFormatReader...");
//     }
// }

// #[no_mangle]
// pub extern "C" fn create_reader() -> *mut c_void {
//     let reader: Box<dyn FormatReader2> = Box::new(MyFormatReader);
//     Box::into_raw(Box::new(reader)) as *mut c_void
// }

// #[no_mangle]
// pub extern "C" fn consume_reader(ptr: *mut c_void) {
//     if ptr.is_null() {
//         return;
//     }
//     unsafe {
//         let reader = Box::from_raw(ptr as *mut Box<dyn FormatReader2>);
//         reader.read();
//     }
// }
