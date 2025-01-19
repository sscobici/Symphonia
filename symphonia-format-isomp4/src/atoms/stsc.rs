// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::atoms::limits::*;
use crate::atoms::{
    Atom, AtomHeader, AtomIterator, Co64Atom, ReadAtom, Result, StcoAtom, decode_error,
    unsupported_error,
};

#[derive(Debug)]
pub struct StscEntry {
    pub first_chunk: u32,
    pub first_sample: u32,
    pub samples_per_chunk: u32,
    #[allow(dead_code)]
    pub sample_desc_index: u32,
}

/// Sample to Chunk Atom
#[allow(dead_code)]
#[derive(Debug)]
pub struct StscAtom {
    /// Entries.
    pub entries: Vec<StscEntry>,
}

impl StscAtom {
    /// Finds the `StscEntry` for the sample indicated by `sample_num`. Note, `sample_num` is indexed
    /// relative to the `StscAtom`. Complexity is O(log2 N).
    pub fn find_entry_for_sample(&self, sample_num: u32) -> Option<&StscEntry> {
        let mut left = 1;
        let mut right = self.entries.len();

        while left < right {
            let mid = left + (right - left) / 2;

            let entry = self.entries.get(mid).unwrap();

            if entry.first_sample < sample_num {
                left = mid + 1;
            }
            else {
                right = mid;
            }
        }

        // The index found above (left) is the exclusive upper bound of all entries where
        // first_sample < sample_num. Therefore, the entry to return has an index of left-1. The
        // index will never equal 0 so this is safe. If the table were empty, left == 1, thus calling
        // get with an index of 0, and safely returning None.
        self.entries.get(left - 1)
    }

    pub fn post_processing(
        &mut self,
        stco: &Option<StcoAtom>,
        co64: &Option<Co64Atom>,
    ) -> Result<()> {
        // Cross check entries.first_chunk agains total_chunks.
        if !self.entries.is_empty() {
            // stco and co64 can both be absent, this is a spec. violation, but some m4a files appear to lack these atoms.
            // set maximum possible total_chunks in this case
            let mut total_chunks = u32::MAX;

            if let Some(stco) = stco {
                total_chunks = stco.chunk_offsets.len() as u32;
            }
            else if let Some(co64) = co64 {
                total_chunks = co64.chunk_offsets.len() as u32;
            }

            // Validate the last first_chunk against the total number of chunks, note that first_chunk is 0 indexed here
            if self.entries.last().unwrap().first_chunk >= total_chunks {
                return decode_error(
                    "isomp4 (stsc): last entry's first chunk exceeds total chunks",
                );
            }
        }

        Ok(())
    }
}

impl Atom for StscAtom {
    fn read<R: ReadAtom>(it: &mut AtomIterator<R>, _header: &AtomHeader) -> Result<Self> {
        let (_, _) = it.read_extended_header()?;

        let entry_count = it.read_u32()?;

        // Limit the maximum initial capacity to prevent malicious files from using all the
        // available memory.
        let mut entries = Vec::with_capacity(MAX_TABLE_INITIAL_CAPACITY.min(entry_count as usize));
        let mut prev_first_chunk = 0;
        let mut prev_first_sample: u32 = 0;

        for i in 0..entry_count {
            let first_chunk = it.read_u32()?;

            // Validate that the first_chunk in the first entry is 1
            if i == 0 && first_chunk != 1 {
                return decode_error(
                    "isomp4 (stsc): first_chunk index in the first entry must be 1",
                );
            }

            // Validate that first_chunk is monotonic across all entries.
            if prev_first_chunk > first_chunk {
                return decode_error("isomp4 (stsc): entry's first_chunk index must be monotonic");
            }

            let samples_per_chunk = it.read_u32()?;

            // Validate that samples per chunk is > 0. Could the entry be ignored?
            if samples_per_chunk == 0 {
                return decode_error("isomp4 (stsc): entry has 0 samples per chunk");
            }

            // Validate that the first_sample calculation does not overflow.
            let n = if i == 0 { 0 } else { first_chunk - prev_first_chunk };
            let first_sample = n
                .checked_mul(samples_per_chunk)
                .and_then(|product| prev_first_sample.checked_add(product))
                .ok_or_else(|| {
                    decode_error::<Self>("isomp4 (stsc): first_sample calculation overflowed")
                        .unwrap_err()
                })?;

            let sample_desc_index = it.read_u32()?;

            // Validate that sample_desc_index is 1, since stsd parsing only supports a single sample entry
            if sample_desc_index != 1 {
                return unsupported_error(
                    "isomp4 (stsc): more than 1 sample entry in stsd is not supported",
                );
            }

            prev_first_chunk = first_chunk;
            prev_first_sample = first_sample;

            entries.push(StscEntry {
                first_chunk: first_chunk - 1,
                first_sample,
                samples_per_chunk,
                sample_desc_index,
            });
        }

        Ok(StscAtom { entries })
    }
}
