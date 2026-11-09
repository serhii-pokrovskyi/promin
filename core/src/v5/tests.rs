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
