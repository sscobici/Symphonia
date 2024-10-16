// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::codecs::video::well_known::extra_data::VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG;
use symphonia_core::codecs::video::VideoExtraData;
use symphonia_core::errors::{Error, Result};
use symphonia_core::io::ReadBytes;

use crate::atoms::{Atom, AtomHeader};

#[allow(dead_code)]
#[derive(Debug)]
pub struct DvvCAtom {
    pub extra_data: VideoExtraData,
}

impl Atom for DvvCAtom {
    fn read<B: ReadBytes>(reader: &mut B, header: AtomHeader) -> Result<Self> {
        // The Dolby Vision Configuration atom payload
        let len = header
            .data_len()
            .ok_or_else(|| Error::DecodeError("isomp4 (dvvC): expected atom size to be known"))?;

        let dv_data = VideoExtraData {
            id: VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG,
            data: reader.read_boxed_slice_exact(len as usize)?,
        };

        Ok(Self { extra_data: dv_data})
    }
}
