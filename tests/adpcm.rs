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
