use std::{fs, os::unix::fs::PermissionsExt};

use atvv_bridge::{Storage, system::SystemBoundaries};

#[test]
fn completed_wavs_are_private_and_use_distinct_paths() {
    let directory = tempfile::tempdir().expect("a temporary WAV directory should be available");
    let mut boundaries = SystemBoundaries::default();

    let first = boundaries
        .create_private_wav(directory.path(), b"first")
        .expect("the first WAV should be created");
    let second = boundaries
        .create_private_wav(directory.path(), b"second")
        .expect("the second WAV should be created");

    assert_ne!(first, second);
    assert_eq!(fs::read(&first).unwrap(), b"first");
    assert_eq!(fs::read(&second).unwrap(), b"second");
    assert_eq!(
        fs::metadata(first).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(second).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
