use atvv_bridge::ImaAdpcmDecoder;

#[test]
fn standard_ima_nibbles_decode_high_first_from_zero_state() {
    let mut decoder = ImaAdpcmDecoder::default();

    assert_eq!(decoder.decode(&[0x10, 0x70]), [1, 1, 12, 14]);
}

#[test]
fn certified_profile_keeps_decoder_state_between_notifications() {
    let mut decoder = ImaAdpcmDecoder::default();

    assert_eq!(decoder.decode(&[0x70]), [11, 13]);
    assert_eq!(decoder.decode(&[0x70]), [38, 41]);
}

#[test]
fn sanitized_notification_yields_two_samples_per_byte_and_saturates() {
    let mut decoder = ImaAdpcmDecoder::default();
    let samples = decoder.decode(&[0x77; 120]);

    assert_eq!(samples.len(), 240);
    assert_eq!(&samples[..6], [11, 41, 104, 240, 533, 1164]);
    assert_eq!(samples.last(), Some(&32_767));
}

#[test]
fn standard_ima_state_is_clamped_at_both_bounds() {
    let mut upper = ImaAdpcmDecoder::default();
    upper.reset(i16::MAX, u8::MAX);
    assert_eq!(upper.decode(&[0x77]), [i16::MAX, i16::MAX]);

    let mut lower = ImaAdpcmDecoder::default();
    lower.reset(i16::MIN, 0);
    assert_eq!(lower.decode(&[0xff]), [i16::MIN, i16::MIN]);
}

#[test]
fn synchronized_state_uses_high_nibble_first_and_persists() {
    let mut decoder = ImaAdpcmDecoder::default();
    decoder.reset(1_000, 20);

    assert_eq!(decoder.decode(&[0x35]), [1_043, 1_104]);
    assert_eq!(decoder.decode(&[0x90]), [1_080, 1_087]);
}
