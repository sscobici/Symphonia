use std::{hint::black_box, sync::Arc};

use criterion::{criterion_group, criterion_main, Criterion};
use symphonia_core::{errors::Error, formats::{probe::Hint, FormatOptions, TrackType}, io::MediaSourceStream, meta::MetadataOptions};

fn demux_mkv_benchmark(c: &mut Criterion) {
    let mut g = c.benchmark_group("demux");
    g.bench_function("demux_mkv", |b| {
        b.iter_with_setup(|| {
            // Open the media source.
            let path = "D:\\Media\\Torrent\\Movies\\A.Million.Miles.Away.2023.DV.HDR.2160p.WEB-DL.H265.Master5.mkv";
            let src = std::fs::File::open(path).expect("failed to open media");

            // Create the media source stream.
            let mss = MediaSourceStream::new(Box::new(src), Default::default());

            // Create a probe hint
            let mut hint = Hint::new();

            // Use the default options for metadata and format readers.
            let meta_opts: MetadataOptions = Default::default();
            let fmt_opts: FormatOptions = Default::default();

            // Probe the media source.
            let mut format = symphonia::default::get_probe()
                .probe(&hint, mss, fmt_opts, meta_opts)
                .expect("unsupported format");

            (format)
        }, |(mut format)| {
            for _ in 0..5000 {
                    // Get the next packet from the media format.
                    let packet = match black_box(format.next_packet()) {
                        Ok(Some(packet)) => packet,
                        Ok(None) => {
                            // Reached the end of the stream.
                            break;
                        }
                        Err(Error::ResetRequired) => {
                            // The track list has been changed. Re-examine it and create a new set of decoders,
                            // then restart the decode loop. This is an advanced feature and it is not
                            // unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
                            // for chained OGG physical streams.
                            unimplemented!();
                        }
                        Err(err) => {
                            // A unrecoverable error occurred, halt decoding.
                            panic!("{}", err);
                        }
                    };
            }
        });
    });

    // g.bench_function("read_ioref_alloc", |b| {
    //     b.iter_with_setup(|| {
    //         let mut stream = MediaSourceStream::default();
    //         stream.add_iobuf(new_iobuf(b"abcdefg")).unwrap();
    //         stream.add_iobuf(new_iobuf(b"hijklmnop")).unwrap();
    //         let ioref = IoRef::default();
    //         (stream, ioref)
    //     }, |(mut stream, mut ioref)| {
    //         stream.read_ioref(black_box(&mut ioref), 8);
    //     });
    // });
}

criterion_group!(benches, demux_mkv_benchmark);
criterion_main!(benches);