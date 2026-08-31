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
struct Shell;
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
    fn perform_button_mapping_effect(&mut self, _: ButtonMappingEffect) {}
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
    let mut shell = Shell;
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
