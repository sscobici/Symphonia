extern crate cocoa;
extern crate objc;

use cocoa::appkit::NSBackingStoreType::NSBackingStoreBuffered;
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyRegular, NSWindow, NSWindowStyleMask};
use cocoa::base::{nil, NO};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString, NSURL};
use objc::runtime::{Class, Object};
use objc::{msg_send, sel, sel_impl};

#[link(name = "AVFoundation", kind = "framework")]
#[link(name = "AVKit", kind = "framework")]
extern "C" {}

fn main() {                     // mp4 NALU -> CMSampleBuffer -> AVSampleBufferDisplayLayer
    unsafe {
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
        let content_view= window.contentView();
        let content_frame = content_view.frame();

        // Create AVSampleBufferDisplayLayer
        let display_layer_class = Class::get("AVSampleBufferDisplayLayer")
            .expect("Unable to find AVSampleBufferDisplayLayer class");
        let display_layer: *mut Object = msg_send![display_layer_class, alloc];
        let display_layer: *mut Object = msg_send![display_layer, init];

        // Add the AVSampleBufferDisplayLayer to the window's content view
        let _: () = msg_send![content_view, setLayer: display_layer];
        let _: () = msg_send![content_view, setWantsLayer: true];

        // Set the autoresizing mask to make the layer resize with the window
        let _: () = msg_send![content_view, setAutoresizingMask: (1 << 1) | (1 << 4)];
        
        let display_layer: *mut Object = msg_send![display_layer, setFrame: content_frame];
        
        // Run the app
        app.run();
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
