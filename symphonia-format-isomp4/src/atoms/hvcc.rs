// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::codecs::video::well_known::CODEC_ID_HEVC;
use symphonia_core::codecs::video::VideoCodecParameters;
use symphonia_core::codecs::CodecProfile;
use symphonia_core::errors::{decode_error, Error, Result};
use symphonia_core::io::{BitReaderLtr, ReadBitsLtr, ReadBytes};

use crate::atoms::{Atom, AtomHeader};

#[allow(dead_code)]
#[derive(Debug)]
pub struct HvcCAtom {
    /// HEVC extra data (HEVCDecoderConfigurationRecord).
    extra_data: Box<[u8]>,
    profile: CodecProfile,
    level: u32,
}

impl Atom for HvcCAtom {
    fn read<B: ReadBytes>(reader: &mut B, header: AtomHeader) -> Result<Self> {
        // The HEVCConfiguration atom payload is a single HEVCDecoderConfigurationRecord. This record
        // forms the defacto codec extra data.
        let len = header
            .data_len()
            .ok_or_else(|| Error::DecodeError("isomp4 (hvcC): expected atom size to be known"))?;

        let extra_data = reader.read_boxed_slice_exact(len as usize)?;

        // Parse the HEVCDecoderConfigurationRecord to get the profile and level. Defined in
        // ISO/IEC 14496-15 section 8.3.3.1.2
        let mut br = BitReaderLtr::new(&extra_data);

        // Configuration version is always 1.
        let configuration_version = br.read_bits_leq32(8)?;

        if configuration_version != 1 {
            return decode_error(
                "isomp4 (hvcC): unexpected hevc decoder configuration record version",
            );
        }

        // HEVC profile as defined in ISO/IEC 23008-2.
        let _general_profile_space = br.read_bits_leq32(2)?;
        let _general_tier_flag = br.read_bits_leq32(1)?;
        let general_profile_idc  = br.read_bits_leq32(5)?;
        let _general_profile_compatibility_flags = br.read_bits_leq32(32)?;
        let _general_constraint_indicator_flags = br.read_bits_leq64(48)?;
        let general_level_idc = br.read_bits_leq32(8)?;

        Ok(Self {
            extra_data,
            profile: CodecProfile::new(general_profile_idc),
            level: general_level_idc,
        })
    }
}

impl HvcCAtom {
    pub fn fill_codec_params(&self, codec_params: &mut VideoCodecParameters) {
        codec_params
            .for_codec(CODEC_ID_HEVC)
            .with_profile(self.profile)
            .with_level(self.level)
            .with_extra_data(self.extra_data.clone());
    }
}
