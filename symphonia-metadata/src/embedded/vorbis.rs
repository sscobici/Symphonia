// Symphonia
// Copyright (c) 2019-2024 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Vorbis Comment reading.

use std::collections::HashMap;

use lazy_static::lazy_static;
use log::warn;

use symphonia_core::errors::Result;
use symphonia_core::io::{BufReader, ReadBytes};
use symphonia_core::meta::{MetadataBuilder, RawTag, Visual};
use symphonia_core::util::text;

use crate::embedded::flac;
use crate::utils::images::try_get_image_info;
use crate::utils::std_tag::*;

lazy_static! {
    static ref VORBIS_COMMENT_MAP: RawTagParserMap = {
        let mut m: RawTagParserMap = HashMap::new();

        m.insert("accurateripcount"             , parse_accuraterip_count);
        m.insert("accurateripcountalloffsets"   , parse_accuraterip_count_all_offsets);
        m.insert("accurateripcountwithoffset"   , parse_accuraterip_count_with_offset);
        m.insert("accurateripcrc"               , parse_accuraterip_crc);
        m.insert("accurateripdiscid"            , parse_accuraterip_disc_id);
        m.insert("accurateripid"                , parse_accuraterip_id);
        m.insert("accurateripoffset"            , parse_accuraterip_offset);
        m.insert("accurateripresult"            , parse_accuraterip_result);
        m.insert("accurateriptotal"             , parse_accuraterip_total);
        m.insert("album artist"                 , parse_album_artist);
        m.insert("album"                        , parse_album);
        m.insert("albumartist"                  , parse_album_artist);
        m.insert("albumartistsort"              , parse_sort_album_artist);
        m.insert("albumsort"                    , parse_sort_album);
        m.insert("arranger"                     , parse_arranger);
        m.insert("artist"                       , parse_artist);
        m.insert("artistsort"                   , parse_sort_artist);
        // TODO: Is Author a synonym for Writer?
        m.insert("author"                       , parse_writer);
        m.insert("barcode"                      , parse_ident_barcode);
        m.insert("bpm"                          , parse_bpm);
        m.insert("catalog #"                    , parse_ident_catalog_number);
        m.insert("catalog"                      , parse_ident_catalog_number);
        m.insert("catalognumber"                , parse_ident_catalog_number);
        m.insert("catalogue #"                  , parse_ident_catalog_number);
        m.insert("cdtoc"                        , parse_cdtoc);
        m.insert("comment"                      , parse_comment);
        m.insert("compilation"                  , parse_compilation);
        m.insert("composer"                     , parse_composer);
        m.insert("conductor"                    , parse_conductor);
        m.insert("copyright"                    , parse_copyright);
        m.insert("ctdbdiscconfidence"           , parse_cuetoolsdb_disc_confidence);
        m.insert("ctdbtrackconfidence"          , parse_cuetoolsdb_track_confidence);
        m.insert("date"                         , parse_date);
        m.insert("description"                  , parse_description);
        m.insert("disc"                         , parse_disc_number_exclusive);
        m.insert("discnumber"                   , parse_disc_number);
        m.insert("discsubtitle"                 , parse_disc_subtitle);
        m.insert("disctotal"                    , parse_disc_total);
        m.insert("disk"                         , parse_disc_number_exclusive);
        m.insert("disknumber"                   , parse_disc_number);
        m.insert("disksubtitle"                 , parse_disc_subtitle);
        m.insert("disktotal"                    , parse_disc_total);
        m.insert("djmixer"                      , parse_mix_dj);
        m.insert("ean/upn"                      , parse_ident_ean_upn);
        m.insert("encoded-by"                   , parse_encoded_by);
        m.insert("encodedby"                    , parse_encoded_by);
        m.insert("encoder settings"             , parse_encoder_settings);
        m.insert("encoder"                      , parse_encoder);
        m.insert("encoding"                     , parse_encoder_settings);
        m.insert("engineer"                     , parse_engineer);
        m.insert("ensemble"                     , parse_ensemble);
        m.insert("genre"                        , parse_genre);
        m.insert("grouping"                     , parse_grouping);
        m.insert("isrc"                         , parse_ident_isrc);
        m.insert("language"                     , parse_language);
        m.insert("label"                        , parse_label);
        m.insert("labelno"                      , parse_ident_catalog_number);
        m.insert("license"                      , parse_license);
        m.insert("lyricist"                     , parse_lyricist);
        m.insert("lyrics"                       , parse_lyrics);
        m.insert("media"                        , parse_media_format);
        m.insert("mixer"                        , parse_mix_engineer);
        m.insert("mood"                         , parse_mood);
        m.insert("musicbrainz_albumartistid"    , parse_musicbrainz_album_artist_id);
        m.insert("musicbrainz_albumid"          , parse_musicbrainz_album_id);
        m.insert("musicbrainz_artistid"         , parse_musicbrainz_artist_id);
        m.insert("musicbrainz_discid"           , parse_musicbrainz_disc_id);
        m.insert("musicbrainz_originalalbumid"  , parse_musicbrainz_original_album_id);
        m.insert("musicbrainz_originalartistid" , parse_musicbrainz_original_artist_id);
        m.insert("musicbrainz_recordingid"      , parse_musicbrainz_recording_id);
        m.insert("musicbrainz_releasegroupid"   , parse_musicbrainz_release_group_id);
        m.insert("musicbrainz_releasetrackid"   , parse_musicbrainz_release_track_id);
        m.insert("musicbrainz_trackid"          , parse_musicbrainz_track_id);
        m.insert("musicbrainz_workid"           , parse_musicbrainz_work_id);
        m.insert("opus"                         , parse_opus);
        m.insert("organization"                 , parse_label);
        m.insert("originaldate"                 , parse_original_date);
        m.insert("originalyear"                 , parse_original_year);
        m.insert("part"                         , parse_part);
        m.insert("partnumber"                   , parse_part_number_exclusive);
        m.insert("performer"                    , parse_performer);
        m.insert("producer"                     , parse_producer);
        m.insert("productnumber"                , parse_ident_pn);
        // TODO: Is Publisher a synonym for Label?
        m.insert("publisher"                    , parse_label);
        m.insert("rating"                       , parse_rating);
        m.insert("releasecountry"               , parse_release_country);
        m.insert("releasestatus"                , parse_musicbrainz_release_status);
        m.insert("releasetype"                  , parse_musicbrainz_release_type);
        m.insert("remixer"                      , parse_remixer);
        m.insert("replaygain_album_gain"        , parse_replaygain_album_gain);
        m.insert("replaygain_album_peak"        , parse_replaygain_album_peak);
        m.insert("replaygain_reference_loudness", parse_replaygain_reference_loudness);
        m.insert("replaygain_track_gain"        , parse_replaygain_track_gain);
        m.insert("replaygain_track_peak"        , parse_replaygain_track_peak);
        m.insert("script"                       , parse_script);
        m.insert("subtitle"                     , parse_track_subtitle);
        m.insert("title"                        , parse_track_title);
        m.insert("titlesort"                    , parse_sort_track_title);
        m.insert("totaldiscs"                   , parse_disc_total);
        m.insert("totaltracks"                  , parse_track_total);
        m.insert("track"                        , parse_track_number_exclusive);
        m.insert("tracknumber"                  , parse_track_number);
        m.insert("tracktotal"                   , parse_track_total);
        m.insert("unsyncedlyrics"               , parse_lyrics);
        m.insert("upc"                          , parse_ident_upc);
        m.insert("version"                      , parse_remixer);
        m.insert("version"                      , parse_version);
        m.insert("work"                         , parse_work);
        m.insert("writer"                       , parse_writer);
        m.insert("year"                         , parse_date);
        m
    };
}

/// Parse a string containing a base64 encoded FLAC picture block into a visual.
fn parse_base64_picture_block(b64: &str, builder: &mut MetadataBuilder) {
    if let Some(data) = base64_decode(b64) {
        if flac::read_flac_picture_block(&mut BufReader::new(&data), builder).is_err() {
            warn!("invalid picture block data");
        }
    }
    else {
        warn!("the base64 encoding of a picture block is invalid");
    }
}

// Parse a string containing a base64 encoding image file into a visual.
fn parse_base64_cover_art(b64: &str, builder: &mut MetadataBuilder) {
    if let Some(data) = base64_decode(b64) {
        if let Some(image_info) = try_get_image_info(&data) {
            builder.add_visual(Visual {
                media_type: Some(image_info.media_type),
                dimensions: Some(image_info.dimensions),
                color_mode: Some(image_info.color_mode),
                usage: None,
                tags: vec![],
                data,
            });
        }
        else {
            warn!("could not detect cover art image format")
        }
    }
    else {
        warn!("the base64 encoding of cover art is invalid");
    }
}

/// Parse the given Vorbis Comment string into a `Tag`.
fn parse_vorbis_comment(buf: &[u8], builder: &mut MetadataBuilder) {
    // Vorbis Comments are stored as <Key>=<Value> pairs where <Key> is a reduced ASCII-only
    // identifier and <Value> is a UTF-8 string value.
    //
    // Convert the entire comment into a UTF-8 string.
    let comment = String::from_utf8_lossy(buf);

    // Split the comment into key and value at the first '=' character.
    if let Some((key, value)) = comment.split_once('=') {
        // The key should only contain ASCII 0x20 through 0x7e (officially 0x7d, but this probably a
        // typo), with 0x3d ('=') excluded.
        let key = key.chars().filter(text::filter::ascii_text).collect::<String>();

        if key.eq_ignore_ascii_case("metadata_block_picture") {
            // A comment with a key "METADATA_BLOCK_PICTURE" is a FLAC picture block encoded in
            // base64. Attempt to decode it as such.
            parse_base64_picture_block(value, builder);
        }
        else if key.eq_ignore_ascii_case("coverart") {
            // A comment with a key "COVERART" is a base64 encoded image. Attempt to decode it as
            // such.
            parse_base64_cover_art(value, builder);
        }
        else {
            // Add a tag created from the key-value pair, while also attempting to map it to a
            // standard tag.
            builder.add_mapped_tags(RawTag::new(key, value), &VORBIS_COMMENT_MAP);
        }
    }
}

pub fn read_vorbis_comment<B: ReadBytes>(
    reader: &mut B,
    builder: &mut MetadataBuilder,
) -> Result<()> {
    // Read the vendor string length in bytes.
    let vendor_length = reader.read_u32()?;

    // Ignore the vendor string.
    reader.ignore_bytes(u64::from(vendor_length))?;

    // Read the number of comments.
    let n_comments = reader.read_u32()? as usize;

    for _ in 0..n_comments {
        // Read the comment string length in bytes.
        let comment_length = reader.read_u32()?;

        // TODO: Apply a limit.

        // Read the comment string.
        let mut comment_data = vec![0; comment_length as usize];
        reader.read_buf_exact(&mut comment_data)?;

        // Parse the Vorbis comment into a Tag and add it to the builder.
        parse_vorbis_comment(&comment_data, builder);
    }

    Ok(())
}

/// Decode a RFC4648 Base64 encoded string.
fn base64_decode(encoded: &str) -> Option<Box<[u8]>> {
    // A sentinel value indicating that an invalid symbol was encountered.
    const BAD_SYM: u8 = 0xff;

    /// Generates a lookup table mapping RFC4648 base64 symbols to their 6-bit decoded values at
    /// compile time.
    const fn rfc4648_base64_symbols() -> [u8; 256] {
        const SYMBOLS: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut table = [BAD_SYM; 256];
        let mut i = 0;

        while i < SYMBOLS.len() {
            table[SYMBOLS[i] as usize] = i as u8;
            i += 1
        }

        table
    }

    const SYM_VALUE: [u8; 256] = rfc4648_base64_symbols();

    // Trim padding, since it's not required for decoding.
    let encoded = encoded.trim_end_matches('=');

    // Each valid base64 symbol decodes to 6 bits. Therefore, the decoded byte length is 3 / 4 the
    // number of symbols in the base64 encoded string.
    let mut decoded = Vec::with_capacity((encoded.len() * 3) / 4);

    // Decode in chunks of 4 symbols, yielding 3 bytes per chunk. Since base64 symbols are ASCII
    // characters (1 byte per character), iterate over the bytes of the base64 string instead of
    // chars (4 bytes per character). This allows the use of a lookup table to determine the symbol
    // value.
    let mut iter = encoded.as_bytes().chunks_exact(4);

    for enc in &mut iter {
        let v0 = SYM_VALUE[usize::from(enc[0])];
        let v1 = SYM_VALUE[usize::from(enc[1])];
        let v2 = SYM_VALUE[usize::from(enc[2])];
        let v3 = SYM_VALUE[usize::from(enc[3])];

        // Check for invalid symbols.
        if v0 == BAD_SYM || v1 == BAD_SYM || v2 == BAD_SYM || v3 == BAD_SYM {
            return None;
        }

        // 6 bits from v0, 2 bits from v1 (4 remaining).
        decoded.push(((v0 & 0x3f) << 2) | (v1 >> 4));
        // 4 bits from v1, 4 bits from v2 (2 remaining).
        decoded.push(((v1 & 0x0f) << 4) | (v2 >> 2));
        // 2 bits from v2, 6 bits from v3 (0 remaining).
        decoded.push(((v2 & 0x03) << 6) | (v3 >> 0));
    }

    // Decode the remaining 2 to 3 symbols.
    let rem = iter.remainder();

    // If there are atleast 2 symbols remaining, then a minimum of one extra byte may be decoded.
    if rem.len() >= 2 {
        let v0 = SYM_VALUE[usize::from(rem[0])];
        let v1 = SYM_VALUE[usize::from(rem[1])];

        if v0 == BAD_SYM || v1 == BAD_SYM {
            return None;
        }

        decoded.push(((v0 & 0x3f) << 2) | (v1 >> 4));

        // If there were 3 symbols remaining, then one additional byte may be decoded.
        if rem.len() >= 3 {
            let v2 = SYM_VALUE[usize::from(rem[2])];

            if v2 == BAD_SYM {
                return None;
            }

            decoded.push(((v1 & 0x0f) << 4) | (v2 >> 2));
        }
    }
    else if rem.len() == 1 {
        // Atleast 2 symbols are required to decode a single byte. Therefore, this is an error.
        return None;
    }

    Some(decoded.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::base64_decode;

    #[test]
    fn verify_base64_decode() {
        // Valid, with padding.
        assert_eq!(Some(b"".as_slice()), base64_decode("").as_deref());
        assert_eq!(Some(b"f".as_slice()), base64_decode("Zg==").as_deref());
        assert_eq!(Some(b"fo".as_slice()), base64_decode("Zm8=").as_deref());
        assert_eq!(Some(b"foo".as_slice()), base64_decode("Zm9v").as_deref());
        assert_eq!(Some(b"foob".as_slice()), base64_decode("Zm9vYg==").as_deref());
        assert_eq!(Some(b"fooba".as_slice()), base64_decode("Zm9vYmE=").as_deref());
        assert_eq!(Some(b"foobar".as_slice()), base64_decode("Zm9vYmFy").as_deref());
        // Valid, without padding.
        assert_eq!(Some(b"".as_slice()), base64_decode("").as_deref());
        assert_eq!(Some(b"f".as_slice()), base64_decode("Zg").as_deref());
        assert_eq!(Some(b"fo".as_slice()), base64_decode("Zm8").as_deref());
        assert_eq!(Some(b"foo".as_slice()), base64_decode("Zm9v").as_deref());
        assert_eq!(Some(b"foob".as_slice()), base64_decode("Zm9vYg").as_deref());
        assert_eq!(Some(b"fooba".as_slice()), base64_decode("Zm9vYmE").as_deref());
        assert_eq!(Some(b"foobar".as_slice()), base64_decode("Zm9vYmFy").as_deref());
        // Invalid.
        assert_eq!(None, base64_decode("a").as_deref());
        assert_eq!(None, base64_decode("ab!c").as_deref());
        assert_eq!(None, base64_decode("ab=c").as_deref());
    }
}
