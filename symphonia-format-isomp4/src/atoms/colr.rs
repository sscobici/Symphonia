// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::codecs::video::ColorSpace;
use symphonia_core::errors::Result;
use symphonia_core::io::ReadBytes;

use crate::atoms::stsd::VisualSampleEntry;
use crate::atoms::{Atom, AtomHeader};

#[allow(dead_code)]
#[derive(Debug)]
pub struct ColrAtom {
    color_space: Option<ColorSpace>,
}

impl Atom for ColrAtom {
    fn read<B: ReadBytes>(reader: &mut B, _header: AtomHeader) -> Result<Self> {
        // ISO/IEC 14496-12:2012 - 8.5.2.2 ColourInformationBox
        let color_type = reader.read_quad_bytes()?;
        let color_space = match &color_type {
            b"nclx" => {
                // on-screen colours
                Some(ColorSpace {
                    colour_primaries: reader.read_be_u16()? as u8,
                    transfer_characteristics: reader.read_be_u16()? as u8,
                    matrix_coefficients: reader.read_be_u16()? as u8,
                    range: reader.read_byte()? >> 7,
                    ..Default::default()
                })
            }
            _ => None,
        };

        Ok(Self { color_space })
    }
}

impl ColrAtom {
    pub fn fill_video_sample_entry(self, entry: &mut VisualSampleEntry) {
        entry.color_space = self.color_space.clone();
    }
}
