extern crate cocoa;
extern crate core_foundation;
extern crate core_media;
extern crate dispatch;
extern crate objc;
extern crate block;
extern crate symphonia;

use std::cell::RefCell;
use std::ops::Deref;
use std::thread;

use block::ConcreteBlock;
use cocoa::appkit::NSBackingStoreType::NSBackingStoreBuffered;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyRegular, NSWindow, NSWindowStyleMask,
};
use cocoa::base::{nil, NO};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use core_foundation::base::TCFType;
use core_foundation::data::CFData;
use core_foundation::dictionary::{CFDictionaryRef, CFMutableDictionary};
use core_foundation::string::{CFString, CFStringRef};
use core_media::sample_buffer::CMSampleTimingInfo;
use core_media::time::{kCMTimeInvalid, CMTimeMake};
use dispatch::Queue;
use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};
use symphonia::core::codecs::video::well_known::extra_data::{
    VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG, VIDEO_EXTRA_DATA_ID_HEVC_DECODER_CONFIG,
};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;

#[link(name = "AVFoundation", kind = "framework")]
#[link(name = "AVKit", kind = "framework")]
#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    fn CMVideoFormatDescriptionCreate(
        allocator: *const Object,
        codecType: u32,
        width: i32,
        height: i32,
        extensions: CFDictionaryRef,
        formatDescriptionOut: *mut *const Object,
    ) -> i32;
    fn CMVideoFormatDescriptionCreateFromHEVCParameterSets(
        allocator: *const Object,
        parameterSetCount: usize,
        parameterSetPointers: *const *const u8,
        parameterSetSizes: *const usize,
        nalUnitHeaderLength: i32,
        extensions: CFDictionaryRef,
        formatDescriptionOut: *mut *const Object,
    ) -> i32;
    fn CFCopyDescription(cf: *const Object) -> CFStringRef;
    fn CMBlockBufferCreateWithMemoryBlock(
        allocator: *const Object,
        memoryBlock: *const u8,
        blockLength: usize,
        blockAllocator: *const Object,
        customBlockSource: *const Object,
        offsetToData: usize,
        dataLength: usize,
        flags: u32,
        blockBufferOut: *mut *const Object,
    ) -> i32;
    fn CMSampleBufferCreateReady(
        allocator: *const Object,
        dataBuffer: *const Object,
        formatDescription: *const Object,
        numSamples: usize,
        numSampleTimingEntries: usize,
        sampleTimingArray: *const CMSampleTimingInfo,
        numSampleSizeEntries: usize,
        sampleSizeArray: *const usize,
        sampleBufferOut: *mut *const Object,
    ) -> i32;
}

const kCMMediaType_Video: u32 = 0x31637661;
const kCMVideoCodecType_DolbyVisionHEVC: u32 = 0x64766831; // FourCC for HEVC ('dvh1')

fn main() {
    // mp4 NALU -> CMSampleBuffer -> AVSampleBufferDisplayLayer
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Initialize Cocoa app
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyRegular);

        // Create a window
        let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0)),
            NSWindowStyleMask::NSClosableWindowMask
                | NSWindowStyleMask::NSTitledWindowMask
                | NSWindowStyleMask::NSResizableWindowMask
                | NSWindowStyleMask::NSFullSizeContentViewWindowMask,
            NSBackingStoreBuffered,
            NO,
        );
        window.center();
        window.setTitle_(NSString::alloc(nil).init_str("Rust AVPlayerView"));
        window.makeKeyAndOrderFront_(nil);

        // Get the frame of the window's content view
        let content_view = window.contentView();
        let content_frame = content_view.frame();

        // Create AVSampleBufferDisplayLayer
        let display_layer_class = Class::get("AVSampleBufferDisplayLayer")
            .expect("Unable to find AVSampleBufferDisplayLayer class");
        let display_layer: *mut Object = msg_send![display_layer_class, alloc];
        let _: *mut Object = msg_send![display_layer, init];

        // Add the AVSampleBufferDisplayLayer to the window's content view
        let _: () = msg_send![content_view, setLayer: display_layer];
        let _: () = msg_send![content_view, setWantsLayer: true];

        // Set the autoresizing mask to make the layer resize with the window
        let _: () = msg_send![content_view, setAutoresizingMask: (1 << 1) | (1 << 4)];
        let _: *mut Object = msg_send![display_layer, setFrame: content_frame];

        // Open the media source.
        let src = std::fs::File::open("/Users/sergheiscobici/Downloads/iOS.mp4")
            .expect("failed to open media");

        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        // Create a probe hint using the file's extension. [Optional]
        let hint = Hint::new();

        // Probe the media source.
        let mut format = symphonia::default::get_probe()
            .probe(&hint, mss, Default::default(), Default::default())
            .expect("unsupported format");

        start_loop(display_layer, format);

        // Run the app
        app.run();
    }
}

fn start_loop(display_layer: *mut Object, format: Box<dyn FormatReader>) {
    unsafe {
        // Find the first video track
        let track = format.default_track(TrackType::Video).expect("no video track");
        let track_id = track.id;
        let mut hvcc_data: &[u8] = &[];
        let mut dvcc_data: &[u8] = &[];
        if let Some(CodecParameters::Video(params)) = &track.codec_params {
            for extra_data in &params.extra_data {
                match extra_data.id {
                    VIDEO_EXTRA_DATA_ID_HEVC_DECODER_CONFIG => hvcc_data = &extra_data.data,
                    VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG => dvcc_data = &extra_data.data,
                    _ => {}
                }
            }
        }

        // Create extentions parameter for CMVideoFormatDescriptionCreate method
        let hvcc = CFData::from_buffer(hvcc_data);
        let dvcc = CFData::from_buffer(dvcc_data);

        let hvcc_key = CFString::new("hvcC");
        let dvcc_key = CFString::new("dvcC");
        let mut extensions_atoms = CFMutableDictionary::new();
        extensions_atoms.add(&hvcc_key, &hvcc);
        extensions_atoms.add(&dvcc_key, &dvcc);

        let extensions_atoms_key: CFString = CFString::new("SampleDescriptionExtensionAtoms");
        let mut extensions = CFMutableDictionary::new();
        extensions.add(&extensions_atoms_key, &extensions_atoms.as_CFType());

        // Call the function
        // Example NAL units
        let parameter_set_1: &[u8] = &[
            64, 1, 12, 1, 255, 255, 2, 32, 0, 0, 3, 0, 176, 0, 0, 3, 0, 0, 3, 0, 153, 152, 144, 48,
            0, 0, 62, 144, 0, 14, 166, 5,
        ];
        /// Example NAL unit
        let parameter_set_2: &[u8] = &[
            66, 1, 1, 2, 32, 0, 0, 3, 0, 176, 0, 0, 3, 0, 0, 3, 0, 153, 160, 1, 224, 32, 2, 28, 77,
            148, 98, 100, 145, 182, 188, 5, 184, 16, 16, 16, 32, 0, 0, 125, 32, 0, 29, 76, 12, 37,
            189, 239, 192, 0, 115, 247, 128, 0, 231, 239, 16,
        ];
        ///
        let parameter_set_3: &[u8] = &[68, 1, 193, 98, 91, 152, 30, 217];
        ///
        let parameter_sets = vec![parameter_set_1, parameter_set_2, parameter_set_3];
        // Convert the NAL unit slices to pointers
        let parameter_set_pointers: Vec<*const u8> =
            parameter_sets.iter().map(|a| a.as_ptr()).collect();

        // Get the sizes of the parameter sets
        let parameter_set_sizes: Vec<usize> = parameter_sets.iter().map(|set| set.len()).collect();

        let mut format_description: *const Object = nil;
        let _status = CMVideoFormatDescriptionCreateFromHEVCParameterSets(
            nil,
            parameter_set_pointers.len(),    // Number of parameter sets
            parameter_set_pointers.as_ptr(), // Pointers to parameter sets
            parameter_set_sizes.as_ptr(),    // Sizes of parameter sets
            4,
            extensions.as_concrete_TypeRef(),
            &mut format_description,
        );

        let format_cell = RefCell::new(format);
        let display_layer_cell = RefCell::new(display_layer);

        let callback = move || {
            println!("Called .... ");
            let display_layer = *display_layer_cell.borrow().deref();
            let mut format = format_cell.borrow_mut();
            // Check if the display layer is ready for more data
            let mut is_ready = true;
            while is_ready {
                is_ready = msg_send![display_layer, isReadyForMoreMediaData];
                let mut packet = None;
                loop {
                    match format.next_packet() {
                        Ok(Some(pack)) => {
                            if pack.track_id() == track_id {
                                packet = Some(pack);
                                break;
                            }
                        }
                        _ => break,
                    }
                };
                if packet.is_none() {
                    break;
                }
                let packet = packet.unwrap();

                let mut block_buffer: *const Object = nil;
                let _status = CMBlockBufferCreateWithMemoryBlock(
                    nil,
                    packet.data.as_ptr(),
                    packet.data.len(),
                    nil,
                    nil,
                    0,
                    packet.data.len(),
                    0,
                    &mut block_buffer,
                );

         //println!("{}", CFString::wrap_under_get_rule(CFCopyDescription(format_description)));
                let timing_info = CMSampleTimingInfo {
                    duration: CMTimeMake(1, 30),  // e.g., 30 fps
                    presentationTimeStamp: CMTimeMake(2, 1),  // Presentation time
                    decodeTimeStamp: kCMTimeInvalid, // Use `kCMTimeInvalid` if no decode timestamp is needed
                };

                let mut sample_buffer: *const Object = nil;
                let _status = CMSampleBufferCreateReady(
                    nil,
                    block_buffer,
                    format_description,
                    1,
                    1,
                    &timing_info as *const CMSampleTimingInfo,
                    0,
                    std::ptr::null_mut(),
                    &mut sample_buffer,
                );

                let _: () = msg_send![display_layer, enqueueSampleBuffer: sample_buffer];
            }
        };

        let callback_block = ConcreteBlock::new(callback);
        // this call is needed to make closure work properly with variables
        let callback_block = callback_block.copy();

        let _: () = msg_send![display_layer, requestMediaDataWhenReadyOnQueue: Queue::main() usingBlock: &*callback_block];
    }
}

// fn main() {            // URL -> AVPlayer -> AVPlayerView
//     unsafe {
//         // Initialize Cocoa app
//         let app = NSApp();
//         app.setActivationPolicy_(NSApplicationActivationPolicyRegular);

//         // Create a window
//         let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
//             NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(800.0, 600.0)),
//             NSWindowStyleMask::NSClosableWindowMask
//                 | NSWindowStyleMask::NSTitledWindowMask
//                 | NSWindowStyleMask::NSResizableWindowMask
//                 | NSWindowStyleMask::NSFullSizeContentViewWindowMask,
//             NSBackingStoreBuffered,
//             NO,
//         );
//         window.center();
//         window.setTitle_(NSString::alloc(nil).init_str("Rust AVPlayerView"));
//         window.makeKeyAndOrderFront_(nil);

//         // Create the AVPlayerView class reference
//         let av_player_view_class = Class::get("AVPlayerView").expect("Unable to find AVPlayerView class");
//         let av_player_view: *mut Object = msg_send![av_player_view_class, alloc];

//         // Get the frame of the window's content view
//         let content_view: *mut Object = window.contentView();
//         let content_frame: NSRect = msg_send![content_view, frame];

//         // Initialize AVPlayerView with the content frame (to fill the window)
//         let av_player_view: *mut Object = msg_send![av_player_view, initWithFrame: content_frame];
//         let _: () = msg_send![av_player_view, setAutoresizingMask: (1 << 1) | (1 << 4)];

//         // Add the AVPlayerView to the window's content view
//         let content_view: *mut Object = window.contentView();
//         let _: () = msg_send![content_view, addSubview: av_player_view];

//         // Create NSURL from file path
//         let file_path_nsstring = NSString::alloc(nil).init_str("/Users/sergheiscobici/Downloads/iOS.mp4");
//         let file_url = NSURL::fileURLWithPath_(nil, file_path_nsstring);

//         // Create an AVPlayer instance and assign it to the AVPlayerView
//         let av_player_class = Class::get("AVPlayer").expect("Unable to find AVPlayer class");
//         let av_player: *mut Object = msg_send![av_player_class, playerWithURL: file_url];
//         let _: () = msg_send![av_player_view, setPlayer: av_player];

//         // Play the video
//         let _: () = msg_send![av_player, play];

//         // Run the app
//         app.run();
//     }
// }
