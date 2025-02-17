use std::{io, sync::Arc};

use windows::Win32::Media::DirectShow::IAsyncReader;

pub(crate) struct WinIAsyncReader {
    reader: Arc<*mut IAsyncReader>,
}

impl WinIAsyncReader {
    pub fn new(reader: *mut IAsyncReader) -> Self {
        Self { reader: Arc::from(reader) }
    }
}

unsafe impl Send for WinIAsyncReader {}

unsafe impl Sync for WinIAsyncReader {}

impl io::Read for WinIAsyncReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let reader = (*self.reader).as_ref().unwrap();
            let mut total: i64 = 0;
            let mut available: i64 = 0;
            reader.Length(&mut total, &mut available)?;

            if total == 0 {
                println!("IAsyncReader.Length() = 0");
            }
            let position = total - available;
            reader.SyncRead(position, buf)?;

            Ok(buf.len())
        }
    }
}
