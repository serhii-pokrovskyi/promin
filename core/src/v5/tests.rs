use super::*;
use crate::modbus::ReadKind;

/// "a5 17 00" -> [0xa5, 0x17, 0x00]
fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

// pysolarmanv5 issue #48: an input read of 6 registers from 0x80fe on slave 1,
// sent to logger 0xa1c37ce4 with sequence byte 0x0b, and the logger's answer.
const ISSUE_48_REQUEST: &str = "a5 17 00 10 45 0b 00 e4 7c c3 a1 02 00 00 00 00 00 00 00 00 \
                                00 00 00 00 00 00 01 04 80 fe 00 06 38 38 36 15";
const ISSUE_48_REPLY: &str = "a5 10 00 10 15 0b 1e e4 7c c3 a1 02 01 c2 0f 01 00 58 13 00 00 \
                              bb ed 84 65 05 00 f8 15";
// pysolarmanv5 issue #81: a heartbeat the logger sends on its own.
const HEARTBEAT: &str = "a5 01 00 10 47 02 d7 cb ea 91 e6 00 5d 15";

fn logger() -> LoggerSerial {
    LoggerSerial::new(0xA1C3_7CE4).unwrap()
}

#[test]
fn logger_serial_is_never_zero() {
    assert!(matches!(
        LoggerSerial::new(0),
        Err(Error::InvalidRequest(_))
    ));
    assert_eq!(LoggerSerial::new(1).unwrap().get(), 1);
}

#[test]
fn checksum_of_published_frames() {
    for frame in [ISSUE_48_REQUEST, ISSUE_48_REPLY, HEARTBEAT] {
        let bytes = hex(frame);
        let len = bytes.len();
        assert_eq!(checksum(&bytes[1..len - 2]), bytes[len - 2], "{frame}");
    }
}

#[test]
fn request_frame_matches_pysolarmanv5() {
    let req = ReadRequest::input(1, 0x80FE, 6).unwrap();
    let frame = request_frame(&req, logger(), 0x0B);
    assert_eq!(frame.to_vec(), hex(ISSUE_48_REQUEST));
}

#[test]
fn request_frame_fields() {
    // the serial as printed on a stick, from ha-solarman issue #1186.
    let logger = LoggerSerial::new(1_701_717_993).unwrap();
    let req = ReadRequest::holding(1, 0x0003, 5).unwrap();
    let frame = request_frame(&req, logger, 0xFF);
    assert_eq!(frame[5], 0xFF);
    assert_eq!(frame[7..11], [0xE9, 0x27, 0x6E, 0x65]);
    assert_eq!(frame[26..34], req.rtu_frame());
}

#[test]
fn request_frames_only_read() {
    for kind in [ReadKind::Holding, ReadKind::Input] {
        let req = ReadRequest::new(1, kind, 0x0000, 1).unwrap();
        let frame = request_frame(&req, logger(), 0x00);
        assert!(matches!(frame[27], 0x03 | 0x04), "{kind:?}");
    }
}

fn frame_of(found: Split) -> (Frame, usize) {
    match found {
        Split::Frame { frame, used } => (frame, used),
        other => panic!("expected a frame, got {other:?}"),
    }
}

/// splits the whole buffer the way the connection will: drop `used` bytes
/// after each answer, stop at `NeedMore`
fn split_all(mut buf: &[u8]) -> Vec<Split> {
    let mut found = Vec::new();
    loop {
        let next = split(buf);
        let used = match &next {
            Split::NeedMore => return found,
            Split::Frame { used, .. } | Split::Junk { used, .. } => *used,
        };
        // every answer but NeedMore drops at least one byte, so this loop ends
        assert!((1..=buf.len()).contains(&used), "{next:?}");
        buf = &buf[used..];
        found.push(next);
    }
}

#[test]
fn splits_heartbeat() {
    let (frame, used) = frame_of(split(&hex(HEARTBEAT)));
    assert_eq!(used, 14);
    assert_eq!(frame.control, 0x4710);
    assert_eq!(frame.seq, 0x02);
    assert_eq!(frame.logger, 0xE691_EACB);
    assert_eq!(frame.payload, [0x00]);
}

#[test]
fn splits_logger_reply() {
    let (frame, used) = frame_of(split(&hex(ISSUE_48_REPLY)));
    assert_eq!(used, 29);
    assert_eq!(frame.control, 0x1510);
    assert_eq!(frame.seq, 0x0B);
    assert_eq!(frame.logger, 0xA1C3_7CE4);
    assert_eq!(frame.payload.len(), 16);
    assert_eq!(frame.payload[14..], [0x05, 0x00]);
}

#[test]
fn splits_frames_read_together() {
    let mut buf = hex(ISSUE_48_REPLY);
    buf.extend(hex(HEARTBEAT));
    let found = split_all(&buf);
    assert_eq!(found.len(), 2);
    assert_eq!(frame_of(found[0].clone()).0.control, 0x1510);
    assert_eq!(frame_of(found[1].clone()).0.control, 0x4710);
}

#[test]
fn drops_bytes_before_start() {
    let junk = Split::Junk {
        used: 3,
        why: Skip::NoStart,
    };
    assert_eq!(split(&hex("00 15 ff")), junk);

    let mut buf = hex("00 15 ff");
    buf.extend(hex(HEARTBEAT));
    let found = split_all(&buf);
    assert_eq!(found[0], junk);
    assert_eq!(frame_of(found[1].clone()).0.control, 0x4710);
}

#[test]
fn waits_for_whole_frame() {
    let full = hex(ISSUE_48_REPLY);
    for len in 0..full.len() {
        assert_eq!(split(&full[..len]), Split::NeedMore, "first {len} bytes");
    }
}

#[test]
fn drops_start_byte_of_broken_frame() {
    let junk = |why| Split::Junk { used: 1, why };

    let mut bad_end = hex(HEARTBEAT);
    bad_end[13] = 0x00;
    assert_eq!(split(&bad_end), junk(Skip::BadEnd));

    let mut bad_checksum = hex(HEARTBEAT);
    bad_checksum[12] ^= 0xFF;
    assert_eq!(split(&bad_checksum), junk(Skip::BadChecksum));

    assert_eq!(split(&hex("a5 ff ff")), junk(Skip::TooLong));
}

#[test]
fn finds_frame_after_false_start() {
    // A stray a5 whose length (0) puts the end byte where there is none.
    let mut buf = hex("a5 00 00");
    buf.extend(hex(HEARTBEAT));
    let found = split_all(&buf);
    assert_eq!(
        found[..2],
        [
            Split::Junk {
                used: 1,
                why: Skip::BadEnd
            },
            Split::Junk {
                used: 2,
                why: Skip::NoStart
            },
        ]
    );
    assert_eq!(frame_of(found[2].clone()).0.control, 0x4710);
}

#[test]
fn garbage_never_panics() {
    // every byte of two real frames swapped for a few telling values.
    let mut real = hex(ISSUE_48_REPLY);
    real.extend(hex(HEARTBEAT));
    for i in 0..real.len() {
        for value in [0x00, 0x15, 0xA5, 0xFF] {
            let mut buf = real.clone();
            buf[i] = value;
            split_all(&buf);
        }
    }
    // made-up bytes, every fourth one a start byte. No rand crate: a simple
    // linear congruential generator is enough here.
    let mut state: u32 = 1;
    let mut next = || {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        state.to_be_bytes()[1]
    };
    for _ in 0..500 {
        let len = usize::from(next() % 64);
        let buf: Vec<u8> = (0..len)
            .map(|i| if i % 4 == 0 { START } else { next() })
            .collect();
        split_all(&buf);
    }
}
