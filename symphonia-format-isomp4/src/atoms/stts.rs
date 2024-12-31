// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::errors::{decode_error, Result};
use symphonia_core::io::ReadBytes;

use crate::atoms::{Atom, AtomHeader, CttsAtom};
use crate::stream::SampleTiming;

#[derive(Debug)]
pub struct SampleDurationEntry {
    pub sample_count: u32,
    pub sample_delta: u32,
}

/// Time-to-sample atom.
#[allow(dead_code)]
#[derive(Debug)]
pub struct SttsAtom {
    pub entries: Vec<SampleDurationEntry>,
    pub total_duration: u64,
    pub ctts: Option<CttsAtom>,
    /// by how much DTS should be shifted to make min PTS of the samples to be 0
    pub dts_offset: i32,
}

pub const MAX_SAMPLES_FOR_DTS_OFFSET: usize = 10;

impl SttsAtom {
    /// Get the timestamp and duration for the sample indicated by `sample_num`. Note, `sample_num`
    /// is indexed relative to the `SttsAtom`. Complexity of this function in O(N).
    pub fn find_timing_for_sample(&self, sample_num: u32) -> Option<SampleTiming> {
        // Consider timings from stts as PTS for video, ensuring PTS starts from 0.
        // Use i64 to avoid overflow during calculations.
        let mut pts = 0;
        let mut next_entry_first_sample = 0;

        // The Stts atom compactly encodes a mapping between number of samples and sample duration.
        // Iterate through each entry until the entry containing the next sample is found. The next
        // packet timestamp is then the sum of the product of sample count and sample duration for
        // the n-1 iterated entries, plus the product of the number of consumed samples in the n-th
        // iterated entry and sample duration.
        for entry in &self.entries {
            next_entry_first_sample += entry.sample_count;

            if sample_num < next_entry_first_sample {
                let entry_sample_offset = sample_num + entry.sample_count - next_entry_first_sample;
                pts += i64::from(entry.sample_delta) * i64::from(entry_sample_offset);

                // for audio DTS is equal to PTS
                let mut dts = pts;

                // adjust PTS / DTS if ctts atom is present and have items
                if let Some(ctts) = &self.ctts {
                    if !ctts.entries.is_empty() {
                        // as stts timings is considered PTS there is a need to deduct DTS offset to get DTS
                        dts -= self.dts_offset as i64;
                        // adjust PTS to be equal to DTS, in case there are not enough entries
                        pts -= self.dts_offset as i64;

                        next_entry_first_sample = 0;
                        for entry in &ctts.entries {
                            next_entry_first_sample += entry.sample_count;

                            if sample_num < next_entry_first_sample {
                                // when the entry is found adjust the PTS
                                pts += entry.sample_delta as i64;
                                break;
                            }
                        }
                    }
                }
                assert!(pts >= 0);
                return Some(SampleTiming { pts: pts as u64, dts, dur: entry.sample_delta });
            }

            pts += i64::from(entry.sample_count) * i64::from(entry.sample_delta);
        }

        None
    }

    /// Get the sample that contains the timestamp indicated by `ts`. Note, the returned `sample_num`
    /// is indexed relative to the `SttsAtom`. Complexity of this function in O(N).
    pub fn find_sample_for_timestamp(&self, ts: u64) -> Option<u32> {
        let mut ts_accum = 0;
        let mut sample_num = 0;

        for entry in &self.entries {
            let delta = u64::from(entry.sample_delta) * u64::from(entry.sample_count);

            if ts_accum + delta > ts {
                sample_num += ((ts - ts_accum) / u64::from(entry.sample_delta)) as u32;
                return Some(sample_num);
            }

            ts_accum += delta;
            sample_num += entry.sample_count;
        }

        None
    }

    /// Assign CttsAtom and calculate DTS offset that will be applied to calculate PTS for Video
    pub fn post_processing(&mut self, ctts: Option<CttsAtom>) {
        self.ctts = ctts;

        if self.ctts.is_none() {
            // DTS offset remains 0
            return;
        }

        let ctts = self.ctts.as_ref().unwrap();

        // calculate DTS offset using the first 10 samples
        // find the minimum negative timing that can occur when ctts data is applied
        let mut ts: i32 = 0;
        let mut ctts_sample_index = 0;
        let mut sample_num = 0;

        let mut ctts_iter = ctts.entries.iter();
        if let Some(ctts_entry) = ctts_iter.next() {
            self.dts_offset = ctts_entry.sample_delta;

            for stts_entry in &self.entries {
                for _ in 0..stts_entry.sample_count {
                    sample_num += 1;
                    ts += stts_entry.sample_delta as i32;

                    let offset = if ctts_sample_index < ctts_entry.sample_count {
                        // get offset value from the current entry
                        ctts_sample_index += 1;
                        ctts_entry.sample_delta
                    }
                    else {
                        // get offset value from the next entry
                        if let Some(ctts_entry) = ctts_iter.next() {
                            ctts_sample_index = 1;
                            ctts_entry.sample_delta
                        }
                        else {
                            // there are no more ctts entries
                            return;
                        }
                    };

                    // change DTS offset if it's lower than the current value
                    if ts + offset < self.dts_offset {
                        self.dts_offset = ts + offset;
                    }

                    if sample_num > MAX_SAMPLES_FOR_DTS_OFFSET {
                        // analyse only MAX_SAMPLES samples from stts
                        return;
                    }
                }
            }
        }
    }
}

impl Atom for SttsAtom {
    fn read<B: ReadBytes>(reader: &mut B, mut header: AtomHeader) -> Result<Self> {
        let (_, _) = header.read_extended_header(reader)?;

        // minimum data size is 4 bytes
        let len = match header.data_len() {
            Some(len) if len >= 4 => len as u32,
            Some(_) => return decode_error("isomp4 (stts): atom size is less than 16 bytes"),
            None => return decode_error("isomp4 (stts): expected atom size to be known"),
        };

        let entry_count = reader.read_be_u32()?;
        if entry_count != (len - 4) / 8 {
            return decode_error("isomp4 (stts): invalid entry count");
        }

        let mut total_duration = 0;

        // TODO: Limit table length.
        let mut entries = Vec::with_capacity(entry_count as usize);

        for _ in 0..entry_count {
            let sample_count = reader.read_be_u32()?;
            let sample_delta = reader.read_be_u32()?;

            if sample_count == 0 {
                return decode_error("isomp4 (stts): sample count cannot be 0");
            }

            total_duration += u64::from(sample_count) * u64::from(sample_delta);

            entries.push(SampleDurationEntry { sample_count, sample_delta });
        }

        Ok(SttsAtom { entries, total_duration, ctts: None, dts_offset: 0 })
    }
}
