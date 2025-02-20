use std::ffi::c_void;
use std::io::{self, Seek};
use std::ptr::null_mut;
use std::sync::Arc;

use symphonia::core::io::MediaSource;
use windows::{core::Interface, Win32::Media::DirectShow::IAsyncReader};

pub(crate) struct WinIAsyncReader {
    reader: Arc<IAsyncReader>,
    pos: u64,
    total: u64,
}

impl WinIAsyncReader {
    pub fn new(reader: *mut c_void) -> Self {
        unsafe {
            let reader = IAsyncReader::from_raw(reader);
            let mut total: i64 = 0;
            reader.Length(&mut total, null_mut()).expect("Cannot call Length of IAsyncReader");

            WinIAsyncReader { reader: Arc::new(reader), pos: 0, total: total as u64 }
        }
    }
}

unsafe impl Send for WinIAsyncReader {}

unsafe impl Sync for WinIAsyncReader {}

impl MediaSource for WinIAsyncReader {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.total)
    }
}

impl Seek for WinIAsyncReader {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            io::SeekFrom::Start(offset) => offset,
            io::SeekFrom::Current(offset) => {
                let signed_pos = self.pos as i64 + offset;
                if signed_pos < 0 || signed_pos > self.total as i64 {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid seek offset"));
                }
                signed_pos as u64
            }
            io::SeekFrom::End(offset) => {
                let signed_pos = self.total as i64 + offset;
                if signed_pos < 0 {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid seek offset"));
                }
                signed_pos as u64
            }
        };
        if new_pos > self.total {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid seek offset"));
        }
        self.pos = new_pos;
        Ok(self.pos)
    }
}

impl io::Read for WinIAsyncReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let reader = &*self.reader;
            let l = if buf.len() as u64 > self.total - self.pos {
                (self.total - self.pos) as usize
            }
            else {
                buf.len()
            };

            if Ok(()) == reader.SyncRead(self.pos as i64, &mut buf[..l]) {
                self.pos += buf.len() as u64;
                Ok(l)
            }
            else {
                Ok(0)
            }
        }
    }
}
