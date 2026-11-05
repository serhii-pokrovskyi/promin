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

#[test]
fn snapshot_lookup() {
    let mut snapshot = RegisterSnapshot::new();
    snapshot.push_block(0x0003, vec![0x3231, 0x3036]);
    snapshot.push_block(0xFFFE, vec![7, 8]);
    assert_eq!(snapshot.get(0x0002), None);
    assert_eq!(snapshot.get(0x0003), Some(0x3231));
    assert_eq!(snapshot.get(0x0004), Some(0x3036));
    assert_eq!(snapshot.get(0x0005), None);
    assert_eq!(snapshot.get(0xFFFF), Some(8));
    assert_eq!(snapshot.blocks().len(), 2);
}

#[test]
fn snapshot_first_block_wins_on_overlap() {
    let mut snapshot = RegisterSnapshot::new();
    snapshot.push_block(10, vec![1, 2]);
    snapshot.push_block(11, vec![99, 3]);
    assert_eq!(snapshot.get(11), Some(2));
    assert_eq!(snapshot.get(12), Some(3));
}
