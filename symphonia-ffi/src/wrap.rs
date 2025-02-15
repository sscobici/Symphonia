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
    /// The packet buffer.
    pub data_len: usize,
}
