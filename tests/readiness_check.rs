use std::io;

use atvv_bridge::{
    BluezClient, BluezSnapshot, Device, GattCharacteristic, GattService, NotReady, Readiness,
    check_readiness,
};

#[derive(Default)]
struct ControlledBluez {
    snapshot: BluezSnapshot,
}

impl BluezClient for ControlledBluez {
    fn managed_objects(&mut self) -> io::Result<BluezSnapshot> {
        Ok(self.snapshot.clone())
    }
}

#[test]
fn connected_atvv_remote_with_resolved_services_is_ready() {
    let mut bluez = ControlledBluez {
        snapshot: ready_atvv_remote("AA:BB:CC:DD:EE:FF", true, "one"),
    };

    let readiness = check_readiness(&mut bluez).expect("BlueZ query should succeed");

    assert_eq!(
        readiness,
        Readiness::Ready {
            address: "AA:BB:CC:DD:EE:FF".into(),
        }
    );
}

#[test]
fn disconnected_atvv_remote_is_not_ready() {
    let mut bluez = ControlledBluez {
        snapshot: ready_atvv_remote("AA:BB:CC:DD:EE:FF", false, "one"),
    };

    assert_eq!(
        check_readiness(&mut bluez).unwrap(),
        Readiness::NotReady {
            address: Some("AA:BB:CC:DD:EE:FF".into()),
            reason: NotReady::Disconnected,
        }
    );
}

#[test]
fn device_without_atvv_service_is_not_ready() {
    let mut bluez = ControlledBluez {
        snapshot: BluezSnapshot {
            devices: ready_atvv_remote("AA:BB:CC:DD:EE:FF", true, "one").devices,
            ..Default::default()
        },
    };

    assert_eq!(
        check_readiness(&mut bluez).unwrap(),
        Readiness::NotReady {
            address: None,
            reason: NotReady::NoAtvvRemote,
        }
    );
}

#[test]
fn atvv_service_without_every_characteristic_is_not_ready() {
    let mut snapshot = ready_atvv_remote("AA:BB:CC:DD:EE:FF", true, "one");
    snapshot.characteristics.pop();
    let mut bluez = ControlledBluez { snapshot };

    assert_eq!(
        check_readiness(&mut bluez).unwrap(),
        Readiness::NotReady {
            address: Some("AA:BB:CC:DD:EE:FF".into()),
            reason: NotReady::MissingCharacteristics,
        }
    );
}

#[test]
fn connected_atvv_remote_with_unresolved_services_is_not_ready() {
    let mut snapshot = ready_atvv_remote("AA:BB:CC:DD:EE:FF", true, "one");
    snapshot.devices[0].services_resolved = false;
    let mut bluez = ControlledBluez { snapshot };

    assert_eq!(
        check_readiness(&mut bluez).unwrap(),
        Readiness::NotReady {
            address: Some("AA:BB:CC:DD:EE:FF".into()),
            reason: NotReady::ServicesUnresolved,
        }
    );
}

#[test]
fn selection_prefers_connected_then_lowest_address() {
    let mut snapshot = ready_atvv_remote("00:00:00:00:00:01", false, "one");
    append(
        &mut snapshot,
        ready_atvv_remote("00:00:00:00:00:03", true, "three"),
    );
    append(
        &mut snapshot,
        ready_atvv_remote("00:00:00:00:00:02", true, "two"),
    );
    let mut bluez = ControlledBluez { snapshot };

    assert_eq!(
        check_readiness(&mut bluez).unwrap(),
        Readiness::Ready {
            address: "00:00:00:00:00:02".into(),
        }
    );
}

#[test]
fn any_complete_atvv_service_makes_the_selected_device_ready() {
    let mut snapshot = ready_atvv_remote("AA:BB:CC:DD:EE:FF", true, "one");
    snapshot.services.insert(
        0,
        GattService {
            path: "/org/bluez/hci0/dev_one/service0009".into(),
            device_path: "/org/bluez/hci0/dev_one".into(),
            uuid: "ab5e0001-5a21-4f05-bc7d-af01f617b664".into(),
        },
    );
    let mut bluez = ControlledBluez { snapshot };

    assert_eq!(
        check_readiness(&mut bluez).unwrap(),
        Readiness::Ready {
            address: "AA:BB:CC:DD:EE:FF".into(),
        }
    );
}

fn append(target: &mut BluezSnapshot, mut other: BluezSnapshot) {
    target.devices.append(&mut other.devices);
    target.services.append(&mut other.services);
    target.characteristics.append(&mut other.characteristics);
}

fn ready_atvv_remote(address: &str, connected: bool, path_suffix: &str) -> BluezSnapshot {
    let device_path = format!("/org/bluez/hci0/dev_{path_suffix}");
    let service_path = format!("{device_path}/service0010");
    BluezSnapshot {
        devices: vec![Device {
            path: device_path.clone(),
            address: address.into(),
            connected,
            services_resolved: true,
        }],
        services: vec![GattService {
            path: service_path.clone(),
            device_path,
            uuid: "ab5e0001-5a21-4f05-bc7d-af01f617b664".into(),
        }],
        characteristics: (2..=4)
            .map(|suffix| GattCharacteristic {
                path: format!("{service_path}/char000{suffix}"),
                service_path: service_path.clone(),
                uuid: format!("ab5e000{suffix}-5a21-4f05-bc7d-af01f617b664"),
            })
            .collect(),
    }
}
