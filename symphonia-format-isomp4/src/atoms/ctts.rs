// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::errors::{decode_error, Result};
use symphonia_core::io::ReadBytes;

use crate::atoms::{Atom, AtomHeader};

#[derive(Debug)]
pub struct SampleOffsetEntry {
    pub sample_count: u32,
    pub sample_delta: i32,
}

/// Composition time to sample atom. Used for video to calculate PTS
#[allow(dead_code)]
#[derive(Debug)]
pub struct CttsAtom {
    pub entries: Vec<SampleOffsetEntry>,
}

impl Atom for CttsAtom {
    fn read<B: ReadBytes>(reader: &mut B, mut header: AtomHeader) -> Result<Self> {
        let (_, _) = header.read_extended_header(reader)?;

        // minimum data size is 4 bytes
        let len = match header.data_len() {
            Some(len) if len >= 4 => len as u32,
            Some(_) => return decode_error("isomp4 (ctts): atom size is less than 16 bytes"),
            None => return decode_error("isomp4 (ctts): expected atom size to be known"),
        };

        let entry_count = reader.read_be_u32()?;
        if entry_count != (len - 4) / 8 {
            return decode_error("isomp4 (ctts): invalid entry count");
        }

        // TODO: Limit table length.
        let mut entries = Vec::with_capacity(entry_count as usize);

        for _ in 0..entry_count {
            let sample_count = reader.read_be_u32()?;
            // version 0: Offsets are unsigned 32-bit integers.
            // version 1: Offsets are signed 32-bit integers.
            // read always as signed, because PTS offsets should not be too far away from DTS
            let sample_delta = reader.read_be_i32()?;

            if sample_count == 0 {
                return decode_error("isomp4 (ctts): sample count cannot be 0");
            }

            entries.push(SampleOffsetEntry { sample_count, sample_delta });
        }

        Ok(CttsAtom { entries })
    }
}
