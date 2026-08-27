use std::{collections::VecDeque, io};

use atvv_bridge::{
    AttachmentError, AttachmentMonitor, AtvvChange, AtvvCodec, AtvvEvent, AtvvGatt,
    AtvvInteractionModel, AtvvProfile, AtvvVersion, BluezSnapshot, Device, GattCharacteristic,
    GattService, ProfileError, attach_online_remote, select_profile,
};

#[derive(Debug, PartialEq, Eq)]
enum GattOperation {
    Snapshot,
    WatchConnection(String),
    Subscribe(String),
    GetCapabilities { tx: String, control: String },
    WaitForChange,
}

struct ControlledGatt {
    snapshots: VecDeque<BluezSnapshot>,
    changes: VecDeque<AtvvChange>,
    capabilities: Vec<u8>,
    operations: Vec<GattOperation>,
}

impl AtvvGatt for ControlledGatt {
    fn snapshot(&mut self) -> io::Result<BluezSnapshot> {
        self.operations.push(GattOperation::Snapshot);
        self.snapshots.pop_front().ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "snapshot script exhausted")
        })
    }

    fn subscribe(&mut self, characteristic_path: &str) -> io::Result<()> {
        self.operations
            .push(GattOperation::Subscribe(characteristic_path.into()));
        Ok(())
    }

    fn watch_connection(&mut self, device_path: &str) -> io::Result<()> {
        self.operations
            .push(GattOperation::WatchConnection(device_path.into()));
        Ok(())
    }

    fn get_capabilities(&mut self, tx_path: &str, control_path: &str) -> io::Result<Vec<u8>> {
        self.operations.push(GattOperation::GetCapabilities {
            tx: tx_path.into(),
            control: control_path.into(),
        });
        Ok(self.capabilities.clone())
    }

    fn wait_for_change(&mut self) -> io::Result<AtvvChange> {
        self.operations.push(GattOperation::WaitForChange);
        Ok(self
            .changes
            .pop_front()
            .unwrap_or(AtvvChange::TopologyChanged))
    }
}

#[test]
fn observed_capabilities_select_the_certified_atvv_profile() {
    let profile = select_profile(&[0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00])
        .expect("the observed Xiaomi capabilities should be certified");

    assert_eq!(profile, AtvvProfile::XIAOMI_V1_HTT_16KHZ_120);
    assert_eq!(profile.version(), AtvvVersion::V1_0);
    assert_eq!(
        profile.interaction_model(),
        AtvvInteractionModel::HoldToTalk
    );
    assert_eq!(profile.codec(), AtvvCodec::ImaDviAdpcm);
    assert_eq!(profile.sample_rate_hz(), 16_000);
    assert_eq!(profile.frame_bytes(), 120);
    assert!(profile.frames_are_headerless());
}

#[test]
fn malformed_capabilities_fail_closed_without_echoing_the_payload() {
    let error = select_profile(&[0x0B, 0x01, 0x00, 0x02])
        .expect_err("a truncated capability response must be rejected");

    assert_eq!(error, ProfileError::MalformedCapabilities);
    assert_eq!(error.to_string(), "malformed ATVV capability response");
}

#[test]
fn unknown_profile_fields_fail_closed_with_specific_operational_errors() {
    let cases = [
        (
            [0x0B, 0x02, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
            ProfileError::UnsupportedVersion,
        ),
        (
            [0x0B, 0x01, 0x00, 0x01, 0x03, 0x00, 0x78, 0x00, 0x00],
            ProfileError::UnsupportedCodec,
        ),
        (
            [0x0B, 0x01, 0x00, 0x02, 0x01, 0x00, 0x78, 0x00, 0x00],
            ProfileError::UnsupportedInteractionModel,
        ),
        (
            [0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x86, 0x00, 0x00],
            ProfileError::UnsupportedFrameShape,
        ),
    ];

    for (payload, expected) in cases {
        assert_eq!(select_profile(&payload), Err(expected));
        assert!(!expected.to_string().contains("0x"));
    }
}

#[test]
fn attachment_subscribes_before_get_caps_and_becomes_ready_after_negotiation() {
    let device_path = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF";
    let service_path = format!("{device_path}/service0010");
    let mut gatt = ControlledGatt {
        snapshots: VecDeque::from([ready_snapshot()]),
        changes: VecDeque::new(),
        capabilities: vec![0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
        operations: Vec::new(),
    };

    let attached = attach_online_remote(&mut gatt)
        .expect("attachment should succeed")
        .expect("an online ATVV Remote should be selected");

    assert_eq!(attached.address, "AA:BB:CC:DD:EE:FF");
    assert_eq!(attached.profile, AtvvProfile::XIAOMI_V1_HTT_16KHZ_120);
    assert_eq!(
        gatt.operations,
        [
            GattOperation::Snapshot,
            GattOperation::WatchConnection(device_path.into()),
            GattOperation::Subscribe(format!("{service_path}/char0004")),
            GattOperation::Subscribe(format!("{service_path}/char0003")),
            GattOperation::GetCapabilities {
                tx: format!("{service_path}/char0002"),
                control: format!("{service_path}/char0004"),
            },
        ]
    );
}

#[test]
fn monitor_waits_without_failing_and_attaches_when_a_remote_appears() {
    let ready = ready_snapshot();
    let mut gatt = ControlledGatt {
        snapshots: VecDeque::from([BluezSnapshot::default(), BluezSnapshot::default(), ready]),
        changes: VecDeque::new(),
        capabilities: vec![0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
        operations: Vec::new(),
    };
    let mut monitor = AttachmentMonitor::default();

    assert_eq!(
        monitor
            .next_event(&mut gatt)
            .expect("waiting should be healthy"),
        AtvvEvent::WaitingForRemote
    );
    assert_eq!(
        monitor
            .next_event(&mut gatt)
            .expect("attachment should succeed"),
        AtvvEvent::RemoteReady {
            address: "AA:BB:CC:DD:EE:FF".into(),
            profile: AtvvProfile::XIAOMI_V1_HTT_16KHZ_120,
        }
    );
    assert!(gatt.operations.contains(&GattOperation::WaitForChange));
}

#[test]
fn monitor_does_not_report_ready_when_capability_negotiation_fails() {
    let mut gatt = ControlledGatt {
        snapshots: VecDeque::from([ready_snapshot()]),
        changes: VecDeque::new(),
        capabilities: vec![0x0B, 0x02, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
        operations: Vec::new(),
    };
    let mut monitor = AttachmentMonitor::default();

    let error = monitor
        .next_event(&mut gatt)
        .expect_err("an uncertified profile must fail closed");

    assert!(matches!(
        error,
        AttachmentError::Profile(ProfileError::UnsupportedVersion)
    ));
    assert_eq!(
        error.to_string(),
        "ATVV capability negotiation failed: unsupported ATVV protocol version"
    );
}

#[test]
fn rebuilt_gatt_endpoints_force_resubscription_and_renegotiation() {
    let first = ready_snapshot();
    let mut rebuilt = ready_snapshot();
    for characteristic in &mut rebuilt.characteristics {
        characteristic.path = characteristic.path.replace("char000", "char001");
    }
    let mut gatt = ControlledGatt {
        snapshots: VecDeque::from([first, rebuilt.clone(), rebuilt]),
        changes: VecDeque::new(),
        capabilities: vec![0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
        operations: Vec::new(),
    };
    let mut monitor = AttachmentMonitor::default();

    assert!(matches!(
        monitor.next_event(&mut gatt),
        Ok(AtvvEvent::RemoteReady { .. })
    ));
    assert_eq!(
        monitor.next_event(&mut gatt).unwrap(),
        AtvvEvent::WaitingForRemote
    );
    assert!(matches!(
        monitor.next_event(&mut gatt),
        Ok(AtvvEvent::RemoteReady { .. })
    ));
    assert_eq!(
        gatt.operations
            .iter()
            .filter(|operation| matches!(operation, GattOperation::GetCapabilities { .. }))
            .count(),
        2
    );
    assert!(gatt.operations.contains(&GattOperation::Subscribe(
        "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF/service0010/char0014".into()
    )));
}

#[test]
fn reconnect_with_stable_gatt_paths_still_forces_renegotiation() {
    let ready = ready_snapshot();
    let mut gatt = ControlledGatt {
        snapshots: VecDeque::from([ready.clone(), ready.clone(), ready]),
        changes: VecDeque::from([AtvvChange::ConnectionChanged]),
        capabilities: vec![0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
        operations: Vec::new(),
    };
    let mut monitor = AttachmentMonitor::default();

    assert!(matches!(
        monitor.next_event(&mut gatt),
        Ok(AtvvEvent::RemoteReady { .. })
    ));
    assert_eq!(
        monitor.next_event(&mut gatt).unwrap(),
        AtvvEvent::WaitingForRemote
    );
    assert!(matches!(
        monitor.next_event(&mut gatt),
        Ok(AtvvEvent::RemoteReady { .. })
    ));
    assert_eq!(
        gatt.operations
            .iter()
            .filter(|operation| matches!(operation, GattOperation::GetCapabilities { .. }))
            .count(),
        2
    );
}

#[test]
fn attached_monitor_forwards_control_and_audio_notifications() {
    let ready = ready_snapshot();
    let mut gatt = ControlledGatt {
        snapshots: VecDeque::from([ready.clone(), ready.clone(), ready]),
        changes: VecDeque::from([
            AtvvChange::ControlNotification(vec![0x04, 0x03, 0x02, 0x91]),
            AtvvChange::AudioNotification(vec![0x11; 120]),
        ]),
        capabilities: vec![0x0B, 0x01, 0x00, 0x02, 0x03, 0x00, 0x78, 0x00, 0x00],
        operations: Vec::new(),
    };
    let mut monitor = AttachmentMonitor::default();

    assert!(matches!(
        monitor.next_event(&mut gatt),
        Ok(AtvvEvent::RemoteReady { .. })
    ));
    assert_eq!(
        monitor.next_event(&mut gatt).unwrap(),
        AtvvEvent::ControlNotification(vec![0x04, 0x03, 0x02, 0x91])
    );
    assert_eq!(
        monitor.next_event(&mut gatt).unwrap(),
        AtvvEvent::AudioNotification(vec![0x11; 120])
    );
}

fn ready_snapshot() -> BluezSnapshot {
    let device_path = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF";
    let service_path = format!("{device_path}/service0010");
    BluezSnapshot {
        devices: vec![Device {
            path: device_path.into(),
            address: "AA:BB:CC:DD:EE:FF".into(),
            connected: true,
            services_resolved: true,
        }],
        services: vec![GattService {
            path: service_path.clone(),
            device_path: device_path.into(),
            uuid: "AB5E0001-5A21-4F05-BC7D-AF01F617B664".into(),
        }],
        characteristics: (2..=4)
            .map(|suffix| GattCharacteristic {
                path: format!("{service_path}/char000{suffix}"),
                service_path: service_path.clone(),
                uuid: format!("AB5E000{suffix}-5A21-4F05-BC7D-AF01F617B664"),
            })
            .collect(),
    }
}
