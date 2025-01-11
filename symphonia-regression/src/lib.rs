#[cfg(test)]
mod tests {
    use format::{assert_format, ExpFormatReader};
    use packet::{assert_packets, ExpPacket};
    use serde::Deserialize;
    use serde_yaml::{self};
    use std::fs;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    mod format;
    mod packet;

    #[derive(Debug, Deserialize)]
    #[allow(unused)]
    struct Expected {
        format: ExpFormatReader,
        packets: Option<Vec<ExpPacket>>,
    }

    const YML_PATH: &str = "test_files\\";
    const FILES_PATH: &str = "N:\\Media\\Torrent\\media-test\\regression\\";

    macro_rules! regression_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let file_path = $value;
                assert_with_file_path(|| verify_file(file_path), file_path);
            }
        )*
        }
    }

    regression_tests! {
        v_bunny_mp4:                                    "video\\bunny.mp4",
        v_avatar_mp4:                                   "video\\Avatar.2009.2160p.UHDRemux.Hybrid.HDR.DV-TheEqualizer.mp4",
        a_2_oga:                                        "audio\\good\\2.oga",
        a_2_wav:                                        "audio\\good\\2.wav",
        a_07_commence_mp3:                              "audio\\good\\07 commence.mp3",
        a_cameraclick_caf:                              "audio\\good\\cameraclick.caf",
        a_defeat_ogg:                                   "audio\\good\\defeat.ogg",
        a_flac_with_id3v2_flac:                         "audio\\good\\flac_with_id3v2.flac",
        a_full_test_aiff:                               "audio\\good\\full_test.aiff",
        a_nelly_ciobanu_hora_din_moldova_m4a:           "audio\\good\\Nelly Ciobanu - Hora din Moldova.m4a",
        a_untagged_aac:                                 "audio\\good\\untagged.aac",
    }

    fn assert_with_file_path<F: FnOnce()>(assertion: F, file_path: &str) {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(assertion)).is_err() {
            eprintln!("  Media File: {}:", FILES_PATH.to_owned() + file_path);
            eprintln!("   Yaml File: {}:", YML_PATH.to_owned() + file_path + ".yml");
            panic!("FAIL: Validation failed");
        }
    }

    fn verify_file(file: &str) {
        let yml_file_path = YML_PATH.to_owned() + file + ".yaml";
        let yaml_data = fs::read_to_string(&yml_file_path)
            .unwrap_or_else(|e| panic!("Failed to read YAML file '{}': {}", yml_file_path, e));

        // Deserialize into Rust structs
        let exp: Expected = serde_yaml::from_str(&yaml_data).expect("Failed to parse YAML");

        // Open the media source.
        let media_file_path = FILES_PATH.to_owned() + file;
        let src = std::fs::File::open(FILES_PATH.to_owned() + file)
            .unwrap_or_else(|e| panic!("Failed to read media file '{}': {}", media_file_path, e));
        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(src), Default::default());
        let mut act = symphonia::default::get_probe()
            .probe(&Hint::default(), mss, FormatOptions::default(), MetadataOptions::default())
            .expect("unsupported format");

        // assert format
        assert_format(&act, exp.format);

        // assert packets
        assert_packets(&mut act, exp.packets);
    }
}
