use super::*;

/// "01 03 0a" -> [0x01, 0x03, 0x0a]
fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect()
}

// inverter serial "2106234258" from registers 0x0003..=0x0007, sent with the
// deye double CRC. Taken from pysolarmanv5 issue #62.
const SERIAL_REPLY_DOUBLE_CRC: &str = "01 03 0a 32 31 30 36 32 33 34 32 35 38 30 6e 00 00";
const SERIAL_WORDS: [u16; 5] = [0x3231, 0x3036, 0x3233, 0x3432, 0x3538];

fn serial_request() -> ReadRequest {
    ReadRequest::holding(1, 0x0003, 5).unwrap()
}

fn serial_reply() -> Vec<u8> {
    let mut frame = hex(SERIAL_REPLY_DOUBLE_CRC);
    frame.truncate(15);
    frame
}

fn fault(result: Result<Vec<u16>, Error>) -> RtuFault {
    match result {
        Err(Error::Rtu(RtuError(fault))) => fault,
        other => panic!("expected an RTU error, got {other:?}"),
    }
}

#[test]
fn crc16_check_value() {
    // The standard check input for CRC catalogues.
    assert_eq!(crc16(b"123456789"), 0x4B37);
}

#[test]
fn request_frames() {
    let holding = ReadRequest::holding(1, 0x0000, 10).unwrap();
    assert_eq!(holding.rtu_frame().to_vec(), hex("01 03 00 00 00 0a c5 cd"));
    let input = ReadRequest::input(1, 0x0000, 10).unwrap();
    assert_eq!(input.rtu_frame().to_vec(), hex("01 04 00 00 00 0a 70 0d"));
}

#[test]
fn requests_only_read() {
    for kind in [ReadKind::Holding, ReadKind::Input] {
        for (start, count) in [(0x0000, 1), (0x0003, 5), (0xFF83, 125)] {
            let frame = ReadRequest::new(1, kind, start, count).unwrap().rtu_frame();
            assert!(matches!(frame[1], 0x03 | 0x04), "{kind:?} {start} {count}");
        }
    }
}

#[test]
fn request_limits() {
    assert!(ReadRequest::holding(1, 0x0000, 125).is_ok());
    assert!(ReadRequest::holding(247, 0xFFFF, 1).is_ok());
    for (slave, start, count) in [
        (0, 0x0000, 1),
        (248, 0x0000, 1),
        (1, 0x0000, 0),
        (1, 0x0000, 126),
        (1, 0xFFFF, 2),
    ] {
        let result = ReadRequest::holding(slave, start, count);
        assert!(
            matches!(result, Err(Error::InvalidRequest(_))),
            "{slave} {start} {count}"
        );
    }
}

#[test]
fn parses_reply() {
    let words = serial_request().parse_reply(&serial_reply()).unwrap();
    assert_eq!(words, SERIAL_WORDS);
}

#[test]
fn accepts_double_crc() {
    let words = serial_request()
        .parse_reply(&hex(SERIAL_REPLY_DOUBLE_CRC))
        .unwrap();
    assert_eq!(words, SERIAL_WORDS);
}

#[test]
fn rejects_other_trailing_bytes() {
    for tail in ["00", "00 01", "00 00 00 00"] {
        let mut frame = serial_reply();
        frame.extend(hex(tail));
        let fault = fault(serial_request().parse_reply(&frame));
        assert!(
            matches!(fault, RtuFault::Length { .. }),
            "{tail}: {fault:?}"
        );
    }
}

#[test]
fn exception_reply() {
    // slave 1 answers a 0x03 read with "illegal data address".
    let err = serial_request()
        .parse_reply(&hex("01 83 02 c0 f1"))
        .unwrap_err();
    assert!(matches!(err, Error::Exception(code) if code.code() == 0x02));
    assert_eq!(
        err.to_string(),
        "modbus exception: illegal data address (0x02)"
    );
}

#[test]
fn bad_crc() {
    let mut frame = serial_reply();
    frame[14] ^= 0xFF;
    let fault = fault(serial_request().parse_reply(&frame));
    assert!(matches!(fault, RtuFault::Crc { .. }), "{fault:?}");
}

#[test]
fn reply_to_another_request() {
    let reply = serial_reply();
    let error = |req: ReadRequest| req.parse_reply(&reply).unwrap_err().to_string();

    assert_eq!(
        error(ReadRequest::holding(2, 0x0003, 5).unwrap()),
        "malformed modbus reply: from slave 1, expected 2"
    );
    assert_eq!(
        error(ReadRequest::input(1, 0x0003, 5).unwrap()),
        "malformed modbus reply: function 0x03, expected 0x04"
    );
    assert_eq!(
        error(ReadRequest::holding(1, 0x0003, 6).unwrap()),
        "malformed modbus reply: 10 data bytes, expected 12"
    );
}

#[test]
fn short_or_damaged_replies_are_errors() {
    let req = serial_request();
    let full = hex(SERIAL_REPLY_DOUBLE_CRC);
    for len in 0..full.len() {
        let ok = req.parse_reply(&full[..len]).is_ok();
        assert_eq!(ok, len == 15, "first {len} bytes");
    }
    // crc-16 catches every single-bit error, so each flip must be refused.
    for i in 0..full.len() {
        for bit in 0..8 {
            let mut frame = full.clone();
            frame[i] ^= 1 << bit;
            assert!(req.parse_reply(&frame).is_err(), "byte {i} bit {bit}");
        }
    }
}
