use std::os::raw::c_char;
use std::ptr::null;

use symphonia::core::codecs::video::{VideoCodecId, VideoExtraDataId};
use symphonia::core::units::TimeBase;

/// A `Packet` contains a discrete amount of encoded data for a single codec bitstream. The exact
/// amount of data is bounded, but not defined, and is dependant on the container and/or the
/// encapsulated codec.
#[repr(C)]
pub struct Packet {
    /// The track ID.
    pub track_id: u32,
    /// The timestamp of the packet. When gapless support is enabled, this timestamp is relative to
    /// the end of the encoder delay.
    ///
    /// This timestamp is in `TimeBase` units.
    pub ts: u64,
    /// The duration of the packet. When gapless support is enabled, the duration does not include
    /// the encoder delay or padding.
    ///
    /// The duration is in `TimeBase` units.
    pub dur: u64,
    /// When gapless support is enabled, this is the number of decoded frames that should be trimmed
    /// from the start of the packet to remove the encoder delay. Must be 0 in all other cases.
    pub trim_start: u32,
    /// When gapless support is enabled, this is the number of decoded frames that should be trimmed
    /// from the end of the packet to remove the encoder padding. Must be 0 in all other cases.
    pub trim_end: u32,
    /// The packet buffer.
    pub data: *const u8,
    /// The packet buffer's length
    pub data_len: usize,
}

/// A `Track` is an independently coded media bitstream. A media format may contain multiple tracks
/// in one container. Each of those tracks are represented by one `Track`.
#[repr(C)]
pub struct Track {
    /// A unique identifier for the track.
    ///
    /// For most formats this is usually the zero-based index of the track, however, some more
    /// complex formats set this differently.
    pub id: u32,
    /// The codec parameters for the track.
    ///
    /// If `None`, the format reader was unable to determine the codec parameters and the track will
    /// be unplayable.
    pub codec_params: CodecParameters,
    /// The language of the track. May be unknown or not set.
    pub language: *const c_char,
    /// The timebase of the track.
    ///
    /// The timebase is the length of time in seconds of a single tick of a timestamp or duration.
    /// It can be used to convert any timestamp or duration related to the track into seconds.
    pub time_base: TimeBase,
    /// The length of the track in number of frames.
    ///
    /// If a timebase is available, this field can be used to calculate the total duration of the
    /// track in seconds by using [`TimeBase::calc_time`] and passing the number of frames as the
    /// timestamp.
    pub num_frames: u64,
    /// The timestamp of the first frame.
    pub start_ts: u64,
    /// The number of leading frames inserted by the encoder that should be skipped during playback.
    pub delay: u32,
    /// The number of trailing frames inserted by the encoder for padding that should be skipped
    /// during playback.
    pub padding: u32,
    /// Flags indicating track attributes.
    pub flags: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct CodecParameters {
    pub codec_type: CodecType,
//    pub audio_params: AudioCodecParameters,
    pub video_params: VideoCodecParameters,
//    pub subtitle_params: SubtitleCodecParameters,
}

#[repr(u8)]
#[derive(Default)]
pub enum CodecType {
    Audio,
    Video,
    Subtitle,
    #[default]
    Unknown, // Add an "Unknown" variant for future extensions or errors
}

/// Codec parameters for audio codecs.
#[repr(C)]
#[derive(Default)]
pub struct AudioCodecParameters {
    //pub sample_rate: c_uint,
    //pub channels: c_uint,
    // Add other audio-specific fields
}

/// Codec parameters for video codecs.
#[repr(C)]
pub struct VideoCodecParameters {
    /// The codec ID.
    pub codec: VideoCodecId,
    /// The codec-defined profile.
    pub profile: CodecProfile,
    /// The codec-defined level.
    pub level: u32,
    /// Video width.
    pub width: u16,
    /// Video height.
    pub height: u16,
    // Extra data (defined by the codec).
    pub extra_data: *const VideoExtraData,
    // Extra data length
    pub extra_data_len: usize,
}

impl Default for VideoCodecParameters {
    fn default() -> Self {
        VideoCodecParameters {
            codec: Default::default(),
            profile: Default::default(),
            level: Default::default(),
            width: Default::default(),
            height: Default::default(),
            extra_data: null(),
            extra_data_len: Default::default(),
        }
    }
}

/// Codec parameters for subtitle codecs.
#[repr(C)]
#[derive(Default)]
pub struct SubtitleCodecParameters {
    // Add subtitle-specific fields
}

/// A codec-specific identification code for a profile.
///
/// In general, codec profiles are designed to target specific applications, and define a set of
/// minimum capabilities a decoder must implement to successfully decode a bitstream. For an
/// encoder, a profile imposes a set of constraints upon the bitstream it produces.
#[repr(C)]
pub struct CodecProfile(i32);

/// Null codec profile
pub const CODEC_PROFILE_NULL: CodecProfile = CodecProfile(-1);

impl CodecProfile {
    pub fn new(value: i32) -> Self {
        CodecProfile(value)
    }
}

impl Default for CodecProfile {
    fn default() -> Self {
        CODEC_PROFILE_NULL
    }
}

/// Extra data for a video codec.
#[repr(C)]
pub struct VideoExtraData {
    /// The extra data ID.
    pub id: VideoExtraDataId,
    /// Extra data (defined by codec)
    pub data: *const u8,
    /// Extra data's length
    pub data_len: usize,
}
