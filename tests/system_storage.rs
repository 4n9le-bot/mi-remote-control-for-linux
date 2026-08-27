use std::{fs, os::unix::fs::PermissionsExt};

use atvv_bridge::{Storage, system::SystemBoundaries};

#[test]
fn completed_wavs_are_private_and_use_distinct_paths() {
    let directory = tempfile::tempdir().expect("a temporary WAV directory should be available");
    let paths = (0..32)
        .map(|index| {
            SystemBoundaries::default()
                .create_private_wav(directory.path(), index.to_string().as_bytes())
                .expect("a WAV should survive fresh boundary instances")
        })
        .collect::<Vec<_>>();

    let mut distinct = paths.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), paths.len());
    for (index, path) in paths.into_iter().enumerate() {
        assert_eq!(fs::read(&path).unwrap(), index.to_string().as_bytes());
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
