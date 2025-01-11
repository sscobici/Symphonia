use serde::Deserialize;
use symphonia::core::formats::{FormatReader, Packet};

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct ExpPacket {
    id: String,
}

const MAX_PACKETS: usize = 100;

pub fn assert_packets(act: &mut Box<dyn FormatReader>, exp: Option<Vec<ExpPacket>>) {
    if let Some(exp) = exp {
        let mut i = 1;
        for exp_packet in exp {
            // process only MAX_PACKETS
            if i > MAX_PACKETS {
                panic!(
                    "Testing only first {} packets, remove the rest of expected packets",
                    MAX_PACKETS
                );
            }
            if let Ok(Some(packet)) = act.next_packet() {
                assert_packet(&packet, &exp_packet, i);
            }
            else {
                panic!("Expected packet {} but found None", i);
            }
            i += 1;
        }

        if i > MAX_PACKETS {
            return;
        }

        if let Ok(Some(p)) = act.next_packet() {
            let last_packet_id = i;
            // output the actual data to put in the yml file
            println!(
                "- id: \"{}, {}, {}, {}, {}, {}\"",
                i,
                p.track_id(),
                p.pts,
                p.dts,
                p.dur,
                p.data.len()
            );
            i += 1;
            while let Ok(Some(p)) = act.next_packet() {
                // output the actual data to put in the yml file
                println!(
                    "- id: \"{}, {}, {}, {}, {}, {}\"",
                    i,
                    p.track_id(),
                    p.pts,
                    p.dts,
                    p.dur,
                    p.data.len()
                );
                i += 1;

                // process only MAX_PACKETS
                if i - last_packet_id > 200 || i > MAX_PACKETS {
                    break;
                }
            }
            panic!(
                "There are more packets left in the media file from packet_id {}",
                last_packet_id
            );
        }
    }
}

fn assert_packet(act: &Packet, exp: &ExpPacket, packet_id: usize) {
    // Split the string by commas and trim whitespace
    let parts: Vec<&str> = exp.id.split(',').map(|s| s.trim()).collect();
    if parts.len() != 6 {
        panic!("packet_id {}, number of elements for packet.id should be 6", packet_id);
    }

    // Parse the parts into respective variables
    let id: usize =
        parts[0].parse().unwrap_or_else(|_| panic!("packet_id {}, Failed to parse id", packet_id));
    let track_id: u32 = parts[1]
        .parse()
        .unwrap_or_else(|_| panic!("packet_id {}, Failed to parse track_id", packet_id));
    let pts: u64 =
        parts[2].parse().unwrap_or_else(|_| panic!("packet_id {}, Failed to parse pts", packet_id));
    let dts: i64 =
        parts[3].parse().unwrap_or_else(|_| panic!("packet_id {}, Failed to parse dts", packet_id));
    let duration: u64 = parts[4]
        .parse()
        .unwrap_or_else(|_| panic!("packet_id {}, Failed to parse duration", packet_id));
    let data_len: usize = parts[5]
        .parse()
        .unwrap_or_else(|_| panic!("packet_id {}, Failed to parse data_len", packet_id));

    assert_eq!(packet_id, id, "packet_id: {}, packets[].track_id", packet_id);
    assert_eq!(act.track_id(), track_id, "packet_id: {}, packets[].track_id", packet_id);
    assert_eq!(act.pts, pts, "packet_id: {}, packets[].pts", packet_id);
    assert_eq!(act.dts, dts, "packet_id: {}, packets[].dts", packet_id);
    assert_eq!(act.dur, duration, "packet_id: {}, packets[].dur", packet_id);
    assert_eq!(act.data.len(), data_len, "packet_id: {}, packets[].data_len", packet_id);
}
