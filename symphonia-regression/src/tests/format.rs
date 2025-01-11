
    use serde::Deserialize;
    use symphonia::core::codecs::audio::well_known::profiles::*;
    use symphonia::core::codecs::audio::well_known::*;
    use symphonia::core::codecs::audio::CODEC_ID_NULL_AUDIO;
    use symphonia::core::codecs::audio::{AudioCodecId, AudioCodecParameters};
    use symphonia::core::codecs::subtitle::well_known::*;
    use symphonia::core::codecs::subtitle::CODEC_ID_NULL_SUBTITLE;
    use symphonia::core::codecs::subtitle::{SubtitleCodecId, SubtitleCodecParameters};
    use symphonia::core::codecs::video::well_known::extra_data::*;
    use symphonia::core::codecs::video::well_known::profiles::*;
    use symphonia::core::codecs::video::well_known::*;
    use symphonia::core::codecs::video::VIDEO_EXTRA_DATA_ID_NULL;
    use symphonia::core::codecs::video::{
        VideoCodecId, VideoCodecParameters, VideoExtraDataId, CODEC_ID_NULL_VIDEO,
    };
    use symphonia::core::codecs::{CodecParameters, CodecProfile};
    use symphonia::core::formats::well_known::*;
    use symphonia::core::formats::{
        FormatId, FormatInfo, FormatReader, Track, TrackFlags, FORMAT_ID_NULL,
    };

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    pub struct ExpFormatReader {
        format_info: ExpFormatInfo,
        tracks: Vec<ExpTrack>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct ExpFormatInfo {
        format: String,
        short_name: String,
        long_name: String,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct ExpTrack {
        id: u32,
        codec_params: Option<ExpCodecParams>,
        language: Option<String>,
        time_base: Option<String>,
        num_frames: Option<u64>,
        start_ts: u64,
        delay: Option<u32>,
        padding: Option<u32>,
        flags: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct ExpCodecParams {
        codec_type: String,
        codec: String,
        profile: Option<String>,

        // video properties
        level: Option<u32>,
        width: Option<u16>,
        height: Option<u16>,
        v_extra_data: Option<Vec<ExpIdExtraData>>,

        // audio properties
        sample_rate: Option<u32>,
        sample_format: Option<String>,
        bits_per_sample: Option<u32>,
        bits_per_coded_sample: Option<u32>,
        channels: Option<String>,
        max_frames_per_packet: Option<u64>,
        verification_check: Option<String>,
        frames_per_block: Option<u64>,
        extra_data: Option<ExpExtraData>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct ExpIdExtraData {
        id: String,
        data_len: usize,
    }

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct ExpExtraData {
        data_len: usize,
    }

    pub fn assert_format(act: &Box<dyn FormatReader>, exp: ExpFormatReader) {
        assert_format_info(act.format_info(), exp.format_info);
        assert_eq!(act.tracks().len(), exp.tracks.len(), "format.tracks length is different");
        for (exp_track, act_track) in exp.tracks.iter().zip(act.tracks()) {
            assert_track(act_track, exp_track);
        }
    }

    fn assert_format_info(act: &FormatInfo, exp: ExpFormatInfo) {
        assert_format_id(act.format, &exp.format);
        assert_eq!(act.short_name, exp.short_name, "format.format_info.short_name");
        assert_eq!(act.long_name, exp.long_name, "format.format_info.long_name");
    }

    fn assert_track(act: &Track, exp: &ExpTrack) {
        assert_eq!(act.id, exp.id, "format.track.id");

        assert_track_codec_params(&act.codec_params, &exp.codec_params, exp.id);

        assert_eq!(act.language, exp.language, "track: {}, format.track.language", exp.id);

        assert_track_time_base(act, exp);

        assert_eq!(act.num_frames, exp.num_frames, "track: {}, format.track.num_frames", exp.id);
        assert_eq!(act.start_ts, exp.start_ts, "track: {}, format.track.start_ts", exp.id);
        assert_eq!(act.delay, exp.delay, "track: {}, format.track.delay", exp.id);
        assert_eq!(act.padding, exp.padding, "track: {}, format.track.padding", exp.id);

        assert_track_flags(act, exp);
    }

    fn assert_track_codec_params(
        act: &Option<CodecParameters>,
        exp: &Option<ExpCodecParams>,
        track_id: u32,
    ) {
        assert_eq!(act.is_some(), exp.is_some(), "track: {}, format.track.codec_params", track_id);
        if exp.is_none() {
            return;
        }

        match (act, exp) {
            (Some(CodecParameters::Video(act_param)), Some(exp_param)) => {
                assert_v_coder_params(act_param, exp_param, track_id)
            }
            (Some(CodecParameters::Audio(act_param)), Some(exp_param)) => {
                assert_a_codec_params(act_param, exp_param, track_id)
            }
            (Some(CodecParameters::Subtitle(act_param)), Some(exp_param)) => {
                assert_s_codec_params(act_param, exp_param, track_id)
            }
            _ => {}
        }
    }

    fn assert_v_coder_params(act: &VideoCodecParameters, exp: &ExpCodecParams, track_id: u32) {
        assert_eq!(
            "Video",
            exp.codec_type.as_str(),
            "track: {}, format.track.codec_params.codec_type",
            track_id
        );
        assert_v_codec_id(act.codec, &exp.codec, track_id);
        assert_v_profile(&exp.codec, &act.profile, &exp.profile, track_id);

        assert_eq!(act.level, exp.level, "track: {}, format.track.codec_param.level", track_id);
        assert_eq!(act.width, exp.width, "track: {}, format.track.codec_param.width", track_id);
        assert_eq!(act.height, exp.height, "track: {}, format.track.codec_param.height", track_id);

        assert_v_extra_data(act, exp, track_id);

        if exp.sample_rate.is_some()                // _
       || exp.sample_format.is_some()           // _
       || exp.bits_per_sample.is_some()         // _
       || exp.bits_per_coded_sample.is_some()   // _
       || exp.channels.is_some()                // _
       || exp.max_frames_per_packet.is_some()   // _
       || exp.verification_check.is_some()      // _
       || exp.frames_per_block.is_some()        // _
       || exp.extra_data.is_some()
        {
            panic!(
                "track: {}, Expected format.track.codec_param video has unrecognized fields",
                track_id
            );
        }
    }

    fn assert_a_codec_params(act: &AudioCodecParameters, exp: &ExpCodecParams, track_id: u32) {
        assert_eq!(
            "Audio",
            exp.codec_type.as_str(),
            "track: {}, format.track.codec_params.codec_type",
            track_id
        );
        assert_a_codec_id(act.codec, &exp.codec, track_id);
        assert_a_profile(&exp.codec, &act.profile, &exp.profile, track_id);
        assert_eq!(
            act.sample_rate, exp.sample_rate,
            "track: {}, format.track.codec_params.sample_rate",
            track_id
        );
        assert_eq!(
            act.sample_format.map(|x| format!("{:?}", x)),
            exp.sample_format,
            "track: {}, format.track.codec_params.sample_format",
            track_id
        );
        assert_eq!(
            act.bits_per_sample, exp.bits_per_sample,
            "track: {}, format.track.codec_params.bits_per_sample",
            track_id
        );
        assert_eq!(
            act.bits_per_coded_sample, exp.bits_per_coded_sample,
            "track: {}, format.track.codec_params.bits_per_coded_sample",
            track_id
        );
        assert_eq!(
            act.channels.as_ref().map(|x| format!("{}", x)),
            exp.channels,
            "track: {}, format.track.codec_params.channels",
            track_id
        );
        assert_eq!(
            act.sample_format.map(|x| format!("{:?}", x)),
            exp.verification_check,
            "track: {}, format.track.codec_params.verification_check",
            track_id
        );
        assert_eq!(
            act.frames_per_block, exp.frames_per_block,
            "track: {}, format.track.codec_params.frames_per_block",
            track_id
        );

        assert_extra_data(&act.extra_data, &exp.extra_data, track_id);

        if exp.level.is_some()                  // _
    || exp.width.is_some()                  // _
    || exp.height.is_some()                 // _
    || exp.v_extra_data.is_some()
        {
            panic!(
                "track: {}, Expected format.track.codec_param audio has unrecognized fields",
                track_id
            );
        }
    }

    fn assert_s_codec_params(act: &SubtitleCodecParameters, exp: &ExpCodecParams, track_id: u32) {
        assert_eq!(
            "Subtitle",
            exp.codec_type.as_str(),
            "track: {}, format.track.codec_params.codec_type",
            track_id
        );
        assert_s_codec_id(act.codec, &exp.codec, track_id);

        assert_extra_data(&act.extra_data, &exp.extra_data, track_id);

        if exp.level.is_some()                   // _
    || exp.width.is_some()                   // _
    || exp.height.is_some()                  // _
    || exp.v_extra_data.is_some()            // _
    || exp.sample_rate.is_some()             // _
    || exp.sample_format.is_some()           // _
    || exp.bits_per_sample.is_some()         // _
    || exp.bits_per_coded_sample.is_some()   // _
    || exp.channels.is_some()                // _
    || exp.max_frames_per_packet.is_some()   // _
    || exp.verification_check.is_some()      // _
    || exp.frames_per_block.is_some()
        {
            panic!(
                "track: {}, Expected format.track.codec_param subtitle has unrecognized fields",
                track_id
            );
        }
    }

    fn assert_v_extra_data(act: &VideoCodecParameters, exp: &ExpCodecParams, track_id: u32) {
        assert_eq!(
            !act.extra_data.is_empty(),
            exp.v_extra_data.is_some(),
            "track: {}, format.track.codec_param.v_extra_data",
            track_id
        );
        if exp.v_extra_data.is_none() {
            return;
        }
        assert_eq!(
            act.extra_data.len(),
            exp.v_extra_data.as_ref().unwrap().len(),
            "track: {}, format.track.codec_param.v_extra_data length is different",
            track_id
        );
        for (extra_data_id, (exp_extra_data, act_extra_data)) in
            exp.v_extra_data.as_ref().unwrap().iter().zip(&act.extra_data).enumerate()
        {
            assert_v_extra_data_id(act_extra_data.id, &exp_extra_data.id, track_id, extra_data_id);
            assert_eq!(
                act_extra_data.data.len(),
                exp_extra_data.data_len,
                "track: {}, format.track.codec_param.v_extra_data[{}].data length is different",
                track_id,
                extra_data_id
            );
        }
    }

    fn assert_extra_data(act: &Option<Box<[u8]>>, exp: &Option<ExpExtraData>, track_id: u32) {
        assert_eq!(
            act.is_some(),
            exp.is_some(),
            "track: {}, format.track.codec_param.extra_data",
            track_id
        );
        if exp.is_some() {
            assert_eq!(
                act.as_ref().unwrap().len(),
                exp.as_ref().unwrap().data_len,
                "track: {}, format.track.codec_param.extra_data length is different",
                track_id
            );
        }
    }

    fn assert_track_time_base(act: &Track, exp: &ExpTrack) {
        let act_time_base = act.time_base.as_ref().map(|t| format!("{}/{}", t.numer, t.denom));
        assert_eq!(act_time_base, exp.time_base, "track: {}, format.track.time_base", exp.id);
    }

    fn assert_track_flags(act: &Track, exp: &ExpTrack) {
        let mut act_flags = String::new();
        for flag in act.flags {
            let name = match flag {
                TrackFlags::DEFAULT => "DEFAULT",
                TrackFlags::FORCED => "FORCED",
                TrackFlags::ORIGINAL_LANGUAGE => "ORIGINAL_LANGUAGE",
                TrackFlags::COMMENTARY => "COMMENTARY",
                TrackFlags::HEARING_IMPAIRED => "HEARING_IMPAIRED",
                TrackFlags::VISUALLY_IMPAIRED => "VISUALLY_IMPAIRED",
                TrackFlags::TEXT_DESCRIPTIONS => "TEXT_DESCRIPTIONS",
                _ => "*UNKNOWN*",
            };
            if act_flags.is_empty() {
                act_flags += name;
            }
            else {
                act_flags = act_flags + " | " + name;
            }
        }
        assert_eq!(
            if act_flags.is_empty() { None } else { Some(act_flags) },
            exp.flags,
            "track: {}, format.track.flags",
            exp.id
        );
    }

    fn assert_format_id(act: FormatId, exp: &String) {
        let act_format = match act {
            FORMAT_ID_WAVE => "WAVE",
            FORMAT_ID_AIFF => "AIFF",
            FORMAT_ID_AVI => "AVI",
            FORMAT_ID_CAF => "CAF",
            FORMAT_ID_MP1 => "MP1",
            FORMAT_ID_MP2 => "MP2",
            FORMAT_ID_MP3 => "MP3",
            FORMAT_ID_ADTS => "ADTS",
            FORMAT_ID_OGG => "OGG",
            FORMAT_ID_FLAC => "FLAC",
            FORMAT_ID_WAVPACK => "WAVPACK",
            FORMAT_ID_ISOMP4 => "ISOMP4",
            FORMAT_ID_MKV => "MKV",
            FORMAT_ID_FLV => "FLV",
            FORMAT_ID_NULL => "NULL",
            _ => {
                eprintln!("cannot detect format.format_info.format FormatId");
                "Unknown"
            }
        };
        assert_eq!(act_format, exp, "format.format_info.format");
    }

    fn assert_v_codec_id(act: VideoCodecId, exp: &String, track_id: u32) {
        let act_codec = match act {
            CODEC_ID_MJPEG => "MJPEG",
            CODEC_ID_BINK_VIDEO => "BINK_VIDEO",
            CODEC_ID_SMACKER_VIDEO => "SMACKER_VIDEO",
            CODEC_ID_CINEPAK => "CINEPAK",
            CODEC_ID_INDEO2 => "INDEO2",
            CODEC_ID_INDEO3 => "INDEO3",
            CODEC_ID_INDEO4 => "INDEO4",
            CODEC_ID_INDEO5 => "INDEO5",
            CODEC_ID_SVQ1 => "SVQ1",
            CODEC_ID_SVQ3 => "SVQ3",
            CODEC_ID_FLV => "FLV",
            CODEC_ID_RV10 => "RV10",
            CODEC_ID_RV20 => "RV20",
            CODEC_ID_RV30 => "RV30",
            CODEC_ID_RV40 => "RV40",
            CODEC_ID_MSMPEG4V1 => "MSMPEG4V1",
            CODEC_ID_MSMPEG4V2 => "MSMPEG4V2",
            CODEC_ID_MSMPEG4V3 => "MSMPEG4V3",
            CODEC_ID_WMV1 => "WMV1",
            CODEC_ID_WMV2 => "WMV2",
            CODEC_ID_WMV3 => "WMV3",
            CODEC_ID_VP3 => "VP3",
            CODEC_ID_VP4 => "VP4",
            CODEC_ID_VP5 => "VP5",
            CODEC_ID_VP6 => "VP6",
            CODEC_ID_VP7 => "VP7",
            CODEC_ID_VP8 => "VP8",
            CODEC_ID_VP9 => "VP9",
            CODEC_ID_THEORA => "THEORA",
            CODEC_ID_AV1 => "AV1",
            CODEC_ID_MPEG1 => "MPEG1",
            CODEC_ID_MPEG2 => "MPEG2",
            CODEC_ID_MPEG4 => "MPEG4",
            CODEC_ID_H261 => "H261",
            CODEC_ID_H263 => "H263",
            CODEC_ID_H264 => "H264",
            CODEC_ID_HEVC => "HEVC",
            CODEC_ID_VVC => "VVC",
            CODEC_ID_VC1 => "VC1",
            CODEC_ID_AVS1 => "AVS1",
            CODEC_ID_AVS2 => "AVS2",
            CODEC_ID_AVS3 => "AVS3",
            CODEC_ID_NULL_VIDEO => "NULL_VIDEO",
            _ => {
                eprintln!(
                    "track: {}, cannot detect format.track.codec_params.codec VideoCodecId",
                    track_id
                );
                "Unknown"
            }
        };
        assert_eq!(act_codec, exp, "track: {}, format.track.codec_params.codec", track_id);
    }

    fn assert_a_codec_id(act: AudioCodecId, exp: &String, track_id: u32) {
        let act_codec = match act {
            CODEC_ID_PCM_S32LE => "PCM_S32LE",
            CODEC_ID_PCM_S32LE_PLANAR => "PCM_S32LE_PLANAR",
            CODEC_ID_PCM_S32BE => "PCM_S32BE",
            CODEC_ID_PCM_S32BE_PLANAR => "PCM_S32BE_PLANAR",
            CODEC_ID_PCM_S24LE => "PCM_S24LE",
            CODEC_ID_PCM_S24LE_PLANAR => "PCM_S24LE_PLANAR",
            CODEC_ID_PCM_S24BE => "PCM_S24BE",
            CODEC_ID_PCM_S24BE_PLANAR => "PCM_S24BE_PLANAR",
            CODEC_ID_PCM_S16LE => "PCM_S16LE",
            CODEC_ID_PCM_S16LE_PLANAR => "PCM_S16LE_PLANAR",
            CODEC_ID_PCM_S16BE => "PCM_S16BE",
            CODEC_ID_PCM_S16BE_PLANAR => "PCM_S16BE_PLANAR",
            CODEC_ID_PCM_S8 => "PCM_S8",
            CODEC_ID_PCM_S8_PLANAR => "PCM_S8_PLANAR",
            CODEC_ID_PCM_U32LE => "PCM_U32LE",
            CODEC_ID_PCM_U32LE_PLANAR => "PCM_U32LE_PLANAR",
            CODEC_ID_PCM_U32BE => "PCM_U32BE",
            CODEC_ID_PCM_U32BE_PLANAR => "PCM_U32BE_PLANAR",
            CODEC_ID_PCM_U24LE => "PCM_U24LE",
            CODEC_ID_PCM_U24LE_PLANAR => "PCM_U24LE_PLANAR",
            CODEC_ID_PCM_U24BE => "PCM_U24BE",
            CODEC_ID_PCM_U24BE_PLANAR => "PCM_U24BE_PLANAR",
            CODEC_ID_PCM_U16LE => "PCM_U16LE",
            CODEC_ID_PCM_U16LE_PLANAR => "PCM_U16LE_PLANAR",
            CODEC_ID_PCM_U16BE => "PCM_U16BE",
            CODEC_ID_PCM_U16BE_PLANAR => "PCM_U16BE_PLANAR",
            CODEC_ID_PCM_U8 => "PCM_U8",
            CODEC_ID_PCM_U8_PLANAR => "PCM_U8_PLANAR",
            CODEC_ID_PCM_F32LE => "PCM_F32LE",
            CODEC_ID_PCM_F32LE_PLANAR => "PCM_F32LE_PLANAR",
            CODEC_ID_PCM_F32BE => "PCM_F32BE",
            CODEC_ID_PCM_F32BE_PLANAR => "PCM_F32BE_PLANAR",
            CODEC_ID_PCM_F64LE => "PCM_F64LE",
            CODEC_ID_PCM_F64LE_PLANAR => "PCM_F64LE_PLANAR",
            CODEC_ID_PCM_F64BE => "PCM_F64BE",
            CODEC_ID_PCM_F64BE_PLANAR => "PCM_F64BE_PLANAR",
            CODEC_ID_PCM_ALAW => "PCM_ALAW",
            CODEC_ID_PCM_MULAW => "PCM_MULAW",
            CODEC_ID_ADPCM_G722 => "ADPCM_G722",
            CODEC_ID_ADPCM_G726 => "ADPCM_G726",
            CODEC_ID_ADPCM_G726LE => "ADPCM_G726LE",
            CODEC_ID_ADPCM_MS => "ADPCM_MS",
            CODEC_ID_ADPCM_IMA_WAV => "ADPCM_IMA_WAV",
            CODEC_ID_ADPCM_IMA_QT => "ADPCM_IMA_QT",
            CODEC_ID_VORBIS => "VORBIS",
            CODEC_ID_OPUS => "OPUS",
            CODEC_ID_SPEEX => "SPEEX",
            CODEC_ID_MUSEPACK => "MUSEPACK",
            CODEC_ID_MP1 => "MP1",
            CODEC_ID_MP2 => "MP2",
            CODEC_ID_MP3 => "MP3",
            CODEC_ID_AAC => "AAC",
            CODEC_ID_AC3 => "AC3",
            CODEC_ID_EAC3 => "EAC3",
            CODEC_ID_AC4 => "AC4",
            CODEC_ID_DCA => "DCA",
            CODEC_ID_ATRAC1 => "ATRAC1",
            CODEC_ID_ATRAC3 => "ATRAC3",
            CODEC_ID_ATRAC3PLUS => "ATRAC3PLUS",
            CODEC_ID_ATRAC9 => "ATRAC9",
            CODEC_ID_WMA => "WMA",
            CODEC_ID_RA10 => "RA10",
            CODEC_ID_RA20 => "RA20",
            CODEC_ID_SIPR => "SIPR",
            CODEC_ID_COOK => "COOK",
            CODEC_ID_SBC => "SBC",
            CODEC_ID_APTX => "APTX",
            CODEC_ID_APTX_HD => "APTX_HD",
            CODEC_ID_LDAC => "LDAC",
            CODEC_ID_BINK_AUDIO => "BINK_AUDIO",
            CODEC_ID_SMACKER_AUDIO => "SMACKER_AUDIO",
            CODEC_ID_FLAC => "FLAC",
            CODEC_ID_WAVPACK => "WAVPACK",
            CODEC_ID_MONKEYS_AUDIO => "MONKEYS_AUDIO",
            CODEC_ID_ALAC => "ALAC",
            CODEC_ID_TTA => "TTA",
            CODEC_ID_RALF => "RALF",
            CODEC_ID_TRUEHD => "TRUEHD",
            CODEC_ID_NULL_AUDIO => "NULL_AUDIO",
            _ => {
                eprintln!(
                    "track: {}, cannot detect format.track.codec_params.codec AudioCodecId",
                    track_id
                );
                "Unknown"
            }
        };
        assert_eq!(act_codec, exp, "track: {}, format.track.codec_params.codec", track_id);
    }

    fn assert_s_codec_id(act: SubtitleCodecId, exp: &String, track_id: u32) {
        let act_codec = match act {
            CODEC_ID_TEXT_UTF8 => "TEXT_UTF8",
            CODEC_ID_SSA => "SSA",
            CODEC_ID_ASS => "ASS",
            CODEC_ID_SAMI => "SAMI",
            CODEC_ID_SRT => "SRT",
            CODEC_ID_WEBVTT => "WEBVTT",
            CODEC_ID_DVBSUB => "DVBSUB",
            CODEC_ID_HDMV_TEXTST => "HDMV_TEXTST",
            CODEC_ID_MOV_TEXT => "MOV_TEXT",
            CODEC_ID_BMP => "BMP",
            CODEC_ID_VOBSUB => "VOBSUB",
            CODEC_ID_HDMV_PGS => "HDMV_PGS",
            CODEC_ID_KATE => "KATE",
            CODEC_ID_NULL_SUBTITLE => "NULL_SUBTITLE",
            _ => {
                eprintln!(
                    "track: {}, cannot detect format.track.codec_params.codec SubtitleCodecId",
                    track_id
                );
                "Unknown"
            }
        };
        assert_eq!(act_codec, exp, "track: {}, format.track.codec_params.codec", track_id);
    }

    fn assert_v_profile(
        codec: &str,
        act: &Option<CodecProfile>,
        exp: &Option<String>,
        track_id: u32,
    ) {
        let act_profile = match codec {
            "AVI" => act.map(|x| match x {
                CODEC_PROFILE_AV1_MAIN => "MAIN",
                CODEC_PROFILE_AV1_HIGH => "HIGH",
                CODEC_PROFILE_AV1_PROFESSIONAL => "PROFESSIONAL",
                _ => "Unknown",
            }),
            "MPEG2" => act.map(|x| match x {
                CODEC_PROFILE_MPEG2_SIMPLE => "SIMPLE",
                CODEC_PROFILE_MPEG2_MAIN => "MAIN",
                CODEC_PROFILE_MPEG2_SNR_SCALABLE => "SNR_SCALABLE",
                CODEC_PROFILE_MPEG2_SPATIAL_SCALABLE => "SPATIAL_SCALABLE",
                CODEC_PROFILE_MPEG2_HIGH => "HIGH",
                CODEC_PROFILE_MPEG2_422 => "422",
                _ => "Unknown",
            }),
            "MPEG4" => act.map(|x| match x {
                CODEC_PROFILE_MPEG4_SIMPLE => "SIMPLE",
                CODEC_PROFILE_MPEG4_ADVANCED_SIMPLE => "ADVANCED_SIMPLE",
                _ => "Unknown",
            }),
            "H264" => act.map(|x| match x {
                CODEC_PROFILE_H264_BASELINE => "BASELINE",
                CODEC_PROFILE_H264_CONSTRAINED_BASELINE => "CONSTRAINED_BASELINE",
                CODEC_PROFILE_H264_MAIN => "MAIN",
                CODEC_PROFILE_H264_EXTENDED => "EXTENDED",
                CODEC_PROFILE_H264_HIGH => "HIGH",
                CODEC_PROFILE_H264_PROGRESSIVE_HIGH => "PROGRESSIVE_HIGH",
                CODEC_PROFILE_H264_CONSTRAINED_HIGH => "CONSTRAINED_HIGH",
                CODEC_PROFILE_H264_HIGH_10 => "HIGH_10",
                CODEC_PROFILE_H264_HIGH_10_INTRA => "HIGH_10_INTRA",
                CODEC_PROFILE_H264_HIGH_422 => "HIGH_422",
                CODEC_PROFILE_H264_HIGH_422_INTRA => "HIGH_422_INTRA",
                CODEC_PROFILE_H264_HIGH_444 => "HIGH_444",
                CODEC_PROFILE_H264_HIGH_444_PREDICTIVE => "HIGH_444_PREDICTIVE",
                CODEC_PROFILE_H264_HIGH_444_INTRA => "HIGH_444_INTRA",
                CODEC_PROFILE_H264_CAVLC_444 => "CAVLC_444",
                _ => "Unknown",
            }),
            "HEVC" => act.map(|x| match x {
                CODEC_PROFILE_HEVC_MAIN => "MAIN",
                CODEC_PROFILE_HEVC_MAIN_10 => "MAIN_10",
                CODEC_PROFILE_HEVC_MAIN_STILL_PICTURE => "MAIN_STILL_PICTURE",
                _ => "Unknown",
            }),
            "VP9" => act.map(|x| match x {
                CODEC_PROFILE_VP9_0 => "0",
                CODEC_PROFILE_VP9_1 => "1",
                CODEC_PROFILE_VP9_2 => "2",
                CODEC_PROFILE_VP9_3 => "3",
                _ => "Unknown",
            }),
            "VC1" => act.map(|x| match x {
                CODEC_PROFILE_VC1_SIMPLE => "SIMPLE",
                CODEC_PROFILE_VC1_MAIN => "MAIN",
                CODEC_PROFILE_VC1_ADVANCED => "ADVANCED",
                _ => "Unknown",
            }),
            _ => act.map(|_| "Unknown"),
        };
        assert_eq!(
            act_profile,
            exp.as_deref(),
            "track: {}, format.track.codec_params.profile",
            track_id
        );
    }

    fn assert_a_profile(
        codec: &str,
        act: &Option<CodecProfile>,
        exp: &Option<String>,
        track_id: u32,
    ) {
        let act_profile = match codec {
            "AAC" => act.map(|x| match x {
                CODEC_PROFILE_AAC_MAIN => "MAIN",
                CODEC_PROFILE_AAC_LC => "LC",
                CODEC_PROFILE_AAC_SSR => "SSR",
                CODEC_PROFILE_AAC_LTP => "LTP",
                CODEC_PROFILE_AAC_HE => "HE",
                CODEC_PROFILE_AAC_HE_V2 => "HE_V2",
                CODEC_PROFILE_AAC_USAC => "USAC",
                _ => "Unknown",
            }),
            _ => act.map(|_| "Unknown"),
        };
        assert_eq!(
            act_profile,
            exp.as_deref(),
            "track: {}, format.track.codec_params.profile",
            track_id
        );
    }

    fn assert_v_extra_data_id(
        act: VideoExtraDataId,
        exp: &String,
        track_id: u32,
        extra_data_id: usize,
    ) {
        let act_codec = match act {
            VIDEO_EXTRA_DATA_ID_AVC_DECODER_CONFIG => "AVC_DECODER_CONFIG",
            VIDEO_EXTRA_DATA_ID_HEVC_DECODER_CONFIG => "HEVC_DECODER_CONFIG",
            VIDEO_EXTRA_DATA_ID_VP9_DECODER_CONFIG => "VP9_DECODER_CONFIG",
            VIDEO_EXTRA_DATA_ID_AV1_DECODER_CONFIG => "AV1_DECODER_CONFIG",
            VIDEO_EXTRA_DATA_ID_DOLBY_VISION_CONFIG => "DOLBY_VISION_CONFIG",
            VIDEO_EXTRA_DATA_ID_DOLBY_VISION_EL_HEVC => "DOLBY_VISION_EL_HEVC",
            VIDEO_EXTRA_DATA_ID_NULL => "VIDEO_EXTRA_DATA_ID_NULL",
            _ => {
                eprintln!(
                "track: {}, cannot detect format.track.codec_param.v_extra_data[{}].data.id VideoExtraDataId",
                track_id,
                extra_data_id
            );
                "Unknown"
            }
        };
        assert_eq!(
            act_codec, exp,
            "track: {}, format.track.codec_param.v_extra_data[{}].data.id",
            track_id, extra_data_id
        );
    }
