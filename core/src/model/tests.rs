use super::*;

#[test]
fn device_id_takes_inverter_serials() {
    for id in ["2106234258", "D262129198A0", "sn-1_b"] {
        assert_eq!(DeviceId::new(id).unwrap().as_str(), id)
    }
}

#[test]
fn device_id_refuses_unsafe_text() {
    let too_long = "1".repeat(33);
    for id in ["", "21062 34258", "a/b", "\0\0\0", "серійник", &too_long] {
        let result = DeviceId::new(id);
        assert!(matches!(result, Err(Error::InvalidDeviceId(_))), "{id:?}");
    }
}
