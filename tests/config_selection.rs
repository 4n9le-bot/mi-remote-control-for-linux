use std::{ffi::OsStr, path::PathBuf};

use atvv_bridge::ConfigSelection;

#[test]
fn explicit_configuration_overrides_standard_locations() {
    assert_eq!(
        ConfigSelection::resolve(
            Some(PathBuf::from("/run/user/1000/bridge.toml")),
            Some(OsStr::new("/xdg")),
            Some(OsStr::new("/home/test")),
        ),
        ConfigSelection::Explicit("/run/user/1000/bridge.toml".into())
    );
}

#[test]
fn xdg_configuration_location_precedes_home() {
    assert_eq!(
        ConfigSelection::resolve(
            None,
            Some(OsStr::new("/xdg")),
            Some(OsStr::new("/home/test")),
        ),
        ConfigSelection::DefaultPath("/xdg/atvv-bridge/config.toml".into())
    );
}

#[test]
fn home_supplies_the_configuration_location_without_valid_xdg() {
    assert_eq!(
        ConfigSelection::resolve(
            None,
            Some(OsStr::new("relative-xdg")),
            Some(OsStr::new("/home/test")),
        ),
        ConfigSelection::DefaultPath("/home/test/.config/atvv-bridge/config.toml".into())
    );
}

#[test]
fn unavailable_standard_locations_use_defaults_only() {
    assert_eq!(
        ConfigSelection::resolve(None, None, None),
        ConfigSelection::DefaultsOnly
    );
}
