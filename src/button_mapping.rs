use std::{collections::BTreeMap, fmt};

mod generated;
pub use generated::{
    LOGICAL_KEYS, REGISTRY_CATALOG_VERSION, REGISTRY_LICENSE, REGISTRY_LINUX_TAG,
    REGISTRY_SOURCE_SHA256, REGISTRY_SOURCE_URL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogicalKey {
    symbol: &'static str,
    code: u16,
    label: &'static str,
}

impl LogicalKey {
    pub const fn symbol(self) -> &'static str {
        self.symbol
    }
    pub const fn code(self) -> u16 {
        self.code
    }
    pub const fn label(self) -> &'static str {
        self.label
    }
}

pub const CATALOG_VERSION: u32 = 1;
pub const VENDOR_ID: &str = "2717";
pub const PRODUCT_ID: &str = "32B8";

pub fn logical_key_by_symbol(symbol: &str) -> Option<LogicalKey> {
    LOGICAL_KEYS
        .iter()
        .copied()
        .find(|key| key.symbol == symbol)
}
pub fn logical_keys_matching(label: &str) -> Vec<LogicalKey> {
    let needle = label.to_ascii_lowercase();
    LOGICAL_KEYS
        .iter()
        .copied()
        .filter(|key| {
            key.label.to_ascii_lowercase().contains(&needle)
                || key.symbol.to_ascii_lowercase().contains(&needle)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ButtonId {
    Power,
    Confirm,
    Up,
    Down,
    Left,
    Right,
    Back,
    VolumeUp,
    VolumeDown,
    Menu,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalButton {
    pub id: ButtonId,
    pub scan_code: &'static str,
    pub native_key: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonCatalog;

impl ButtonCatalog {
    pub const fn v1() -> &'static [PhysicalButton; 11] {
        &[
            PhysicalButton {
                id: ButtonId::Power,
                scan_code: "70066",
                native_key: "KEY_POWER",
            },
            PhysicalButton {
                id: ButtonId::Confirm,
                scan_code: "70028",
                native_key: "KEY_ENTER",
            },
            PhysicalButton {
                id: ButtonId::Up,
                scan_code: "70052",
                native_key: "KEY_UP",
            },
            PhysicalButton {
                id: ButtonId::Down,
                scan_code: "70051",
                native_key: "KEY_DOWN",
            },
            PhysicalButton {
                id: ButtonId::Left,
                scan_code: "70050",
                native_key: "KEY_LEFT",
            },
            PhysicalButton {
                id: ButtonId::Right,
                scan_code: "7004f",
                native_key: "KEY_RIGHT",
            },
            PhysicalButton {
                id: ButtonId::Back,
                scan_code: "700f1",
                native_key: "KEY_BACK",
            },
            PhysicalButton {
                id: ButtonId::VolumeUp,
                scan_code: "70080",
                native_key: "KEY_VOLUMEUP",
            },
            PhysicalButton {
                id: ButtonId::VolumeDown,
                scan_code: "70081",
                native_key: "KEY_VOLUMEDOWN",
            },
            PhysicalButton {
                id: ButtonId::Menu,
                scan_code: "70065",
                native_key: "KEY_COMPOSE",
            },
            PhysicalButton {
                id: ButtonId::Live,
                scan_code: "70035",
                native_key: "KEY_GRAVE",
            },
        ]
    }
}

fn catalog_button(id: ButtonId) -> &'static PhysicalButton {
    ButtonCatalog::v1()
        .iter()
        .find(|button| button.id == id)
        .expect("every catalog button has metadata")
}

impl ButtonId {
    pub const ALL: [Self; 11] = [
        Self::Power,
        Self::Confirm,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Back,
        Self::VolumeUp,
        Self::VolumeDown,
        Self::Menu,
        Self::Live,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Confirm => "confirm",
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Back => "back",
            Self::VolumeUp => "volume_up",
            Self::VolumeDown => "volume_down",
            Self::Menu => "menu",
            Self::Live => "live",
        }
    }
    pub fn scan_code(self) -> &'static str {
        catalog_button(self).scan_code
    }
    pub fn native_key(self) -> &'static str {
        catalog_button(self).native_key
    }
    pub const fn default_target(self) -> MappingTarget {
        if matches!(self, Self::Power) {
            MappingTarget::Disabled
        } else {
            MappingTarget::Original
        }
    }
}

impl TryFrom<&str> for ButtonId {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|id| id.as_str() == value)
            .ok_or(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingTarget {
    Original,
    Disabled,
    Key(LogicalKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping(BTreeMap<ButtonId, MappingTarget>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingError {
    MissingButton(ButtonId),
    DuplicateButton(ButtonId),
    UnknownButton,
    InvalidKey,
}

impl Mapping {
    pub fn defaults() -> Self {
        Self(
            ButtonId::ALL
                .into_iter()
                .map(|id| (id, id.default_target()))
                .collect(),
        )
    }
    pub fn from_entries(
        entries: impl IntoIterator<Item = (ButtonId, MappingTarget)>,
    ) -> Result<Self, MappingError> {
        let mut map = BTreeMap::new();
        for (button, target) in entries {
            if map.insert(button, target).is_some() {
                return Err(MappingError::DuplicateButton(button));
            }
        }
        for id in ButtonId::ALL {
            if !map.contains_key(&id) {
                return Err(MappingError::MissingButton(id));
            }
        }
        if map.len() != ButtonId::ALL.len() {
            return Err(MappingError::UnknownButton);
        }
        if map
            .values()
            .any(|target| matches!(target, MappingTarget::Key(key) if !LOGICAL_KEYS.contains(key)))
        {
            return Err(MappingError::InvalidKey);
        }
        Ok(Self(map))
    }
    pub fn get(&self, id: ButtonId) -> MappingTarget {
        self.0[&id]
    }
    pub fn iter(&self) -> impl Iterator<Item = (ButtonId, MappingTarget)> + '_ {
        self.0.iter().map(|(&id, &target)| (id, target))
    }
}

impl fmt::Display for ButtonId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
