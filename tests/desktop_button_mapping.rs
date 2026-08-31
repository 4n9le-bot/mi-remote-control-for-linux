use std::io;

use atvv_bridge::{
    ButtonMappingController, ButtonMappingEffect, ButtonMappingEvent, ButtonMappingState,
    DesktopApplication, DesktopShell, DesktopStatus, VoiceBridge,
    button_mapping::{ButtonId, Mapping, MappingTarget},
    button_mapping_backend::{
        BackendOperation, BackendResult, BackendStartError, ButtonMappingBackend,
    },
    helper_protocol::DecodedResponse,
};

#[derive(Default)]
struct Bridge;
impl VoiceBridge for Bridge {
    fn start(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn take_latest_status(&mut self) -> Option<DesktopStatus> {
        None
    }
}
struct FakeBackend {
    results: Vec<BackendResult>,
    started: Vec<BackendOperation>,
}
impl ButtonMappingBackend for FakeBackend {
    fn start(&mut self, op: BackendOperation) -> Result<(), BackendStartError> {
        self.started.push(op);
        Ok(())
    }
    fn try_take_result(&mut self) -> Option<BackendResult> {
        if self.results.is_empty() {
            None
        } else {
            Some(self.results.remove(0))
        }
    }
}
#[derive(Default)]
struct Shell {
    effects: Vec<ButtonMappingEffect>,
}
impl DesktopShell for Shell {
    fn create_status_window_with_close_action(&mut self) {}
    fn present_status_window(&mut self) {}
    fn display_status(&mut self, _: &DesktopStatus) {}
    fn tray_available(&self) -> bool {
        true
    }
    fn hide_status_window(&mut self) {}
    fn confirm_close_quits_bridge(&mut self) {}
    fn quit(&mut self) {}
    fn perform_button_mapping_effect(&mut self, effect: ButtonMappingEffect) {
        self.effects.push(effect);
    }
}

#[test]
fn mapping_is_lazy_and_edits_are_staged_until_apply() {
    let mapping = Mapping::defaults();
    let backend = FakeBackend {
        results: vec![Ok(DecodedResponse::Inspect {
            revision: "r1".into(),
            mapping: mapping.clone(),
        })],
        started: vec![],
    };
    let mut app = DesktopApplication::new_with_button_mapping(Bridge, backend);
    let mut shell = Shell::default();
    assert!(matches!(
        app.button_mapping().state(),
        ButtonMappingState::Unloaded
    ));
    app.button_mapping_event(ButtonMappingEvent::Open, &mut shell);
    assert!(matches!(
        app.button_mapping().state(),
        ButtonMappingState::Inspecting
    ));
    app.refresh_button_mapping(&mut shell);
    app.button_mapping_event(
        ButtonMappingEvent::Edit(ButtonId::Menu, MappingTarget::Disabled),
        &mut shell,
    );
    assert!(app.button_mapping().presentation().dirty);
    app.button_mapping_event(ButtonMappingEvent::Apply, &mut shell);
    assert!(matches!(
        app.button_mapping().state(),
        ButtonMappingState::Applying
    ));
}

#[test]
fn opening_mapping_navigates_the_existing_window_and_starts_inspection() {
    let backend = FakeBackend {
        results: vec![],
        started: vec![],
    };
    let mut app = DesktopApplication::new_with_button_mapping(Bridge, backend);
    let mut shell = Shell::default();

    app.button_mapping_event(ButtonMappingEvent::Open, &mut shell);

    assert_eq!(shell.effects, vec![ButtonMappingEffect::Open]);
    assert!(matches!(
        app.button_mapping().state(),
        ButtonMappingState::Inspecting
    ));
}

#[test]
fn recovery_reset_requires_confirmation_before_starting() {
    let backend = FakeBackend {
        results: vec![Ok(DecodedResponse::RecoveryRequired)],
        started: vec![],
    };
    let mut controller = ButtonMappingController::new(backend);
    controller.dispatch(ButtonMappingEvent::Open);
    controller.poll();

    assert_eq!(
        controller.dispatch(ButtonMappingEvent::Reset),
        Some(ButtonMappingEffect::ConfirmReset)
    );
    assert_eq!(controller.backend().started.len(), 1);
    assert_eq!(
        controller.dispatch(ButtonMappingEvent::ConfirmReset),
        Some(ButtonMappingEffect::AuthorizationRequired)
    );
    assert!(matches!(controller.state(), ButtonMappingState::Resetting));
}

#[test]
fn conflict_preserves_draft_and_requires_explicit_reload() {
    let mapping = Mapping::defaults();
    let backend = FakeBackend {
        results: vec![Ok(DecodedResponse::Inspect {
            revision: "r1".into(),
            mapping,
        })],
        started: vec![],
    };
    let mut controller = ButtonMappingController::new(backend);
    controller.dispatch(ButtonMappingEvent::Open);
    controller.poll();
    controller.dispatch(ButtonMappingEvent::Edit(
        ButtonId::Menu,
        MappingTarget::Disabled,
    ));
    controller.dispatch(ButtonMappingEvent::Apply);
    controller
        .backend_mut()
        .results
        .push(Ok(DecodedResponse::Error(
            atvv_bridge::helper_protocol::StableErrorCode::RevisionConflict,
        )));
    controller.poll();
    assert!(matches!(
        controller.state(),
        ButtonMappingState::Conflict { .. }
    ));
    assert!(!controller.presentation().can_apply);
}

#[test]
fn operation_presentation_keeps_the_staged_draft_visible() {
    let mapping = Mapping::defaults();
    let backend = FakeBackend {
        results: vec![Ok(DecodedResponse::Inspect {
            revision: "r1".into(),
            mapping,
        })],
        started: vec![],
    };
    let mut controller = ButtonMappingController::new(backend);
    controller.dispatch(ButtonMappingEvent::Open);
    controller.poll();
    controller.dispatch(ButtonMappingEvent::Edit(
        ButtonId::Menu,
        MappingTarget::Disabled,
    ));
    controller.dispatch(ButtonMappingEvent::Apply);

    assert_eq!(
        controller
            .presentation()
            .draft
            .expect("applying should present the staged draft")
            .get(ButtonId::Menu),
        MappingTarget::Disabled
    );
}

#[test]
fn invalid_mapping_has_a_distinct_editable_validation_presentation() {
    let mapping = Mapping::defaults();
    let backend = FakeBackend {
        results: vec![Ok(DecodedResponse::Inspect {
            revision: "r1".into(),
            mapping,
        })],
        started: vec![],
    };
    let mut controller = ButtonMappingController::new(backend);
    controller.dispatch(ButtonMappingEvent::Open);
    controller.poll();
    controller.dispatch(ButtonMappingEvent::Edit(
        ButtonId::Menu,
        MappingTarget::Disabled,
    ));
    controller.dispatch(ButtonMappingEvent::Apply);
    controller
        .backend_mut()
        .results
        .push(Ok(DecodedResponse::Error(
            atvv_bridge::helper_protocol::StableErrorCode::InvalidMapping,
        )));
    controller.poll();

    assert!(matches!(
        controller.state(),
        ButtonMappingState::Validation { .. }
    ));
    assert!(controller.presentation().can_reset);
    assert!(!controller.presentation().can_apply);

    controller.dispatch(ButtonMappingEvent::Edit(
        ButtonId::Menu,
        MappingTarget::Original,
    ));
    assert!(matches!(
        controller.state(),
        ButtonMappingState::Ready { .. }
    ));
}

#[test]
fn authorization_cancellation_preserves_the_staged_draft() {
    let mapping = Mapping::defaults();
    let backend = FakeBackend {
        results: vec![Ok(DecodedResponse::Inspect {
            revision: "r1".into(),
            mapping,
        })],
        started: vec![],
    };
    let mut controller = ButtonMappingController::new(backend);
    controller.dispatch(ButtonMappingEvent::Open);
    controller.poll();
    controller.dispatch(ButtonMappingEvent::Edit(
        ButtonId::Menu,
        MappingTarget::Disabled,
    ));
    controller.dispatch(ButtonMappingEvent::Apply);

    controller.backend_mut().results.push(Err(
        atvv_bridge::button_mapping_backend::BackendFailure::AuthorizationNotGranted,
    ));
    controller.poll();

    let presentation = controller.presentation();
    assert!(presentation.dirty);
    assert_eq!(
        presentation
            .draft
            .expect("authorization cancellation should preserve the draft")
            .get(ButtonId::Menu),
        MappingTarget::Disabled
    );
    assert!(
        presentation
            .notice
            .as_deref()
            .is_some_and(|notice| notice.contains("Authorization was cancelled"))
    );
}
