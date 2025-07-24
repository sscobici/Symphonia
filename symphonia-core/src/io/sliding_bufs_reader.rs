use std::io;

/// A reader that operates over a sliding window of immutable byte slices.
///
/// This structure is designed for high-performance, zero-copy reading from a sequence
/// of buffers, such as those received from a network socket or read from a file in chunks.
/// It uses a fixed-size ring buffer to manage the slices and does not perform any
/// heap allocations during its read operations.
pub struct SlidingBufsReader<'a> {
    /// Total logical size in bytes of the content being read.
    size: usize,
    /// Current global read position, relative to `size`.
    r_pos: usize,
    /// Ring buffer holding the byte slices.
    bufs_ring: [&'a [u8]; 4],
    /// The head of the ring buffer, pointing to the oldest slice, where the reads will happen first
    bufs_head: usize,
    /// The tail of the ring buffer, pointing to where the next slice will be inserted
    bufs_tail: usize,
    /// The index of the buffer in `bufs_ring` from which we are currently reading.
    r_buf_idx: usize,
    /// The read position within the buffer at `r_buf_idx`.
    r_buf_pos: usize,
}

/// Creates an `io::Result<T>` with an `UnexpectedEof` error.
#[inline(always)]
fn underrun_error<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::UnexpectedEof, "buffer underrun"))
}

/// Creates an `io::Result<T>` for errors when a read spans too many buffers.
#[inline(always)]
fn span_error<T>() -> io::Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, "read spans too many buffers"))
}

impl<'a> SlidingBufsReader<'a> {
    /// Instantiate a new `SlidingBufsReader`.
    ///
    /// # Arguments
    ///
    /// * `size` - The total expected size of the content to be read. Must be non zero and less than usize::MAX
    pub fn new(size: usize) -> Self {
        // TODO
        assert!(size != 0);
        assert!(size != usize::MAX);
        SlidingBufsReader {
            size,
            r_pos: 0,
            bufs_ring: [&[]; 4],
            bufs_head: 0,
            bufs_tail: 0,
            r_buf_idx: 0,
            r_buf_pos: 0,
        }
    }

    // --- Methods for adding and removing slices ---

    /// Adds a buffer slice to the reader. This is an alias for `push_buf`.
    #[inline]
    pub fn add_slice(&mut self, buf: &'a [u8]) -> Result<(), &'static str> {
        self.push_buf(buf)
    }

    /// Removes the oldest buffer slice from the reader. This is an alias for `pop_buf`.
    #[inline]
    pub fn remove_slice(&mut self) -> Result<&'a [u8], &'static str> {
        self.pop_buf()
    }

    /// Adds a buffer slice to the tail of the ring buffer.
    pub fn push_buf(&mut self, buf: &'a [u8]) -> Result<(), &'static str> {
        assert!(buf.len() > 0);
        let next_tail = (self.bufs_tail + 1) % self.bufs_ring.len();
        if next_tail == self.bufs_head {
            return Err("ring buffer is full");
        }
        self.bufs_ring[self.bufs_tail] = buf;
        self.bufs_tail = next_tail;

        Ok(())
    }

    /// Removes the oldest buffer slice from the head of the ring buffer.
    pub fn pop_buf(&mut self) -> Result<&'a [u8], &'static str> {
        if self.bufs_head == self.bufs_tail {
            return Err("empty ring buffer");
        }
        // Ensure the read cursor does not point to the buffer being popped.
        if self.r_buf_idx == self.bufs_head {
            return Err("extracting the buffer that is currently read");
        }

        let buf = self.bufs_ring[self.bufs_head];
        self.bufs_ring[self.bufs_head] = &[];
        self.bufs_head = (self.bufs_head + 1) % self.bufs_ring.len();

        Ok(buf)
    }

    // --- Zero-copy read method ---

    /// Returns an array of up to 4 buffer slices that constitute the next `len` bytes.
    ///
    /// This method is zero-copy. It returns slices that point directly into the managed
    /// buffers. The state of the reader is advanced by `len` bytes upon success.
    ///
    /// # Returns
    ///
    /// A `Result` containing:
    /// * `Ok(([&'a [u8]; 4], usize))` - A tuple where the first element is a fixed-size
    ///   array of slices, and the second element is the number of valid slices in the array.
    /// * `Err(io::Error)` - An `UnexpectedEof` if `len` exceeds the remaining data, or
    ///   an `Other` error if the read would span more than 4 buffers.
    pub fn read_slices(&mut self, len: usize) -> io::Result<([&'a [u8]; 4], usize)> {
        if self.r_pos.saturating_add(len) > self.size {
            return underrun_error();
        }

        let mut slices: [&[u8]; 4] = [&[]; 4];
        let mut slice_count = 0;
        let mut remaining_len = len;

        let mut current_buf_idx = self.r_buf_idx;
        let mut current_buf_pos = self.r_buf_pos;

        while remaining_len > 0 {
            if current_buf_idx == self.bufs_tail {
                return underrun_error();
            }
            if slice_count >= self.bufs_ring.len() {
                return span_error();
            }

            let current_buf = self.bufs_ring[current_buf_idx];
            let available_in_buf = current_buf.len().saturating_sub(current_buf_pos);
            let to_take = available_in_buf.min(remaining_len);

            if to_take > 0 {
                slices[slice_count] = &current_buf[current_buf_pos..current_buf_pos + to_take];
                slice_count += 1;
                remaining_len -= to_take;
                current_buf_pos += to_take;
            }

            if remaining_len > 0 {
                current_buf_idx = (current_buf_idx + 1) % self.bufs_ring.len();
                current_buf_pos = 0;
            }
        }

        // Advance the reader's state
        self.r_pos += len;
        self.r_buf_idx = current_buf_idx;
        self.r_buf_pos = current_buf_pos;

        Ok((slices, slice_count))
    }

    // --- Primitive read methods ---

    /// Reads a single byte (`u8`) from the buffer.
    #[inline]
    pub fn read_u8(&mut self) -> io::Result<u8> {
        if self.r_pos.saturating_add(1) > self.size {
            return underrun_error();
        }

        // Fast path: byte is in the current buffer
        let current_buf = self.bufs_ring[self.r_buf_idx];
        if self.r_buf_pos < current_buf.len() {
            let byte = current_buf[self.r_buf_pos];
            self.r_pos += 1;
            self.r_buf_pos += 1;
            return Ok(byte);
        }

        // Slow path: byte is in the next buffer
        let next_r_buf_idx = (self.r_buf_idx + 1) % self.bufs_ring.len();
        // Check if we have lapped the tail, meaning no more buffers are available
        if next_r_buf_idx == self.bufs_tail {
            return underrun_error();
        }
        let byte = self.bufs_ring[next_r_buf_idx][0];
        // Move to the next buffer in the ring
        self.r_pos += 1;
        self.r_buf_idx = next_r_buf_idx;
        self.r_buf_pos = 1;

        Ok(byte)
    }

    /// Reads a 32-bit unsigned integer in little-endian format.
    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        const SIZE: usize = 4;
        if self.r_pos.saturating_add(SIZE) > self.size {
            return underrun_error();
        }

        // Fast path: all bytes are in the current buffer
        let current_buf = self.bufs_ring[self.r_buf_idx];
        if current_buf.len().saturating_sub(self.r_buf_pos) >= SIZE {
            let res = u32::from_le_bytes(self.get_bytes());
            self.r_pos += SIZE;
            self.r_buf_pos += SIZE;
            return Ok(res);
        }

        // Slow path: bytes span across buffers
        let mut buf = [0u8; SIZE];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Reads a 64-bit unsigned integer in little-endian format.
    pub fn read_u64_le(&mut self) -> io::Result<u64> {
        const SIZE: usize = 8;
        if self.r_pos.saturating_add(SIZE) > self.size {
            return underrun_error();
        }

        // Fast path: all bytes are in the current buffer
        let current_buf = self.bufs_ring[self.r_buf_idx];
        if current_buf.len().saturating_sub(self.r_buf_pos) >= SIZE {
            let res = u64::from_le_bytes(self.get_bytes());
            self.r_pos += SIZE;
            self.r_buf_pos += SIZE;
            return Ok(res);
        }

        // Slow path: bytes span across buffers
        let mut buf = [0u8; SIZE];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    // --- Private Helper Methods ---

    /// Fills the provided `buf` with data from the reader.
    ///
    /// This is a private helper used by the primitive read methods for their slow path.
    /// It is not part of the public-facing zero-copy API.
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let len = buf.len();
        let (slices, num_slices) = self.read_slices(len)?;

        let mut bytes_copied = 0;
        for i in 0..num_slices {
            let slice = slices[i];
            let copy_len = slice.len();
            buf[bytes_copied..bytes_copied + copy_len].copy_from_slice(slice);
            bytes_copied += copy_len;
        }

        if bytes_copied != len {
            // This should not be reached if read_slices is correct
            return underrun_error();
        }

        Ok(())
    }

    /// Advances the internal cursors (`r_buf_idx`, `r_buf_pos`) to the start
    /// of the next buffer that contains data.
    #[inline]
    fn advance_to_next_nonempty_buf(&mut self) -> io::Result<()> {
        loop {
            // Move to the next buffer in the ring
            self.r_buf_idx = (self.r_buf_idx + 1) % self.bufs_ring.len();
            self.r_buf_pos = 0;

            // Check if we have lapped the tail, meaning no more buffers are available
            if self.r_buf_idx == self.bufs_tail {
                return underrun_error();
            }

            // If the new buffer is not empty, we are done
            if !self.bufs_ring[self.r_buf_idx].is_empty() {
                return Ok(());
            }
            // Otherwise, loop to the next one
        }
    }

    #[inline]
    fn get_bytes<const N: usize>(&self) -> [u8; N] {
        self.bufs_ring[self.r_buf_idx][self.r_buf_pos..self.r_buf_pos + N]
            .try_into()
            .expect("slice with incorrect length")
    }
}
