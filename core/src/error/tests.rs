use super::*;

#[test]
fn error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<Error>();
}

#[test]
fn exception_code_names() {
    assert_eq!(
        ExceptionCode(0x02).to_string(),
        "illegal data address (0x02)"
    );
    assert_eq!(ExceptionCode(0x42).to_string(), "unknown (0x42)")
}

#[test]
fn messages_nest() {
    let fault = RtuFault::Crc {
        expected: 0x6e30,
        got: 0x6e31,
    };
    let err = Error::Rtu(RtuError(fault));
    assert_eq!(
        err.to_string(),
        "malformed modbus reply: crc 0x6e31, expected 0x6e30"
    )
}
