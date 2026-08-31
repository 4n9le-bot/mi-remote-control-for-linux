use atvv_bridge::button_mapping::{
    ButtonCatalog, ButtonId, LOGICAL_KEYS, Mapping, MappingError, MappingTarget,
};

#[test]
fn catalog_v1_has_certified_buttons_and_safe_defaults() {
    assert_eq!(ButtonCatalog::v1().len(), 11);
    assert_eq!(ButtonCatalog::v1()[0].id, ButtonId::Power);
    assert_eq!(
        Mapping::defaults().get(ButtonId::Power),
        MappingTarget::Disabled
    );
    assert_eq!(
        Mapping::defaults().get(ButtonId::Confirm),
        MappingTarget::Original
    );
}

#[test]
fn complete_mapping_rejects_duplicate_physical_buttons() {
    let mut entries: Vec<_> = Mapping::defaults().iter().collect();
    entries.push((ButtonId::Power, MappingTarget::Original));
    assert_eq!(
        Mapping::from_entries(entries),
        Err(MappingError::DuplicateButton(ButtonId::Power))
    );
}

#[test]
fn registry_excludes_reserved_unknown_and_max_and_has_native_keys() {
    assert!(
        LOGICAL_KEYS
            .iter()
            .all(|key| !matches!(key.symbol(), "KEY_RESERVED" | "KEY_UNKNOWN" | "KEY_MAX"))
    );
    for symbol in ["KEY_POWER", "KEY_ENTER", "KEY_GRAVE"] {
        assert!(LOGICAL_KEYS.iter().any(|key| key.symbol() == symbol));
    }
}
