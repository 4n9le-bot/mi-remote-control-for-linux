use std::env;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

use adw::prelude::*;
use atvv_bridge::ConfigSelection;
use atvv_bridge::{
    AtvvProfileReadiness, BatteryStatus, ButtonMappingEffect, ButtonMappingEvent,
    ButtonMappingPresentation, ButtonMappingState, CaptureStatus, DesktopApplication, DesktopShell,
    DesktopStatus, InProcessVoiceBridge, RecentWavHandoff, RecoveryStatus, RemoteStatus,
    WavHandoffActivity, WavHandoffOutcome,
    button_mapping::{ButtonId, LOGICAL_KEYS, MappingTarget},
};

#[derive(Debug, Clone)]
enum DesktopEvent {
    ActivateRequested,
    ButtonMappingRequested,
    ButtonMapping(ButtonMappingEvent),
    CloseRequested,
    CloseConfirmed(bool),
    QuitRequested,
}

fn main() {
    let selection = ConfigSelection::resolve(
        None,
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    );
    let desktop = Rc::new(RefCell::new(DesktopApplication::new(
        InProcessVoiceBridge::new(selection),
    )));
    let (event_sender, event_receiver) = mpsc::channel();
    let shell = Rc::new(RefCell::new(GtkDesktopShell::new(event_sender)));
    let application = adw::Application::builder()
        .application_id("io.github.atvv_bridge")
        .build();
    let activated_desktop = Rc::clone(&desktop);
    let activated_shell = Rc::clone(&shell);
    application.connect_activate(move |application| {
        activated_shell.borrow_mut().set_application(application);
        if let Err(error) = activated_desktop
            .borrow_mut()
            .activate(&mut *activated_shell.borrow_mut())
        {
            eprintln!("atvv-bridge: could not start the desktop application: {error}");
        }
    });
    gtk::glib::timeout_add_local(Duration::from_millis(100), move || {
        let mut desktop = desktop.borrow_mut();
        let mut shell = shell.borrow_mut();
        while let Ok(event) = event_receiver.try_recv() {
            match event {
                DesktopEvent::ActivateRequested => {
                    if let Err(error) = desktop.activate(&mut *shell) {
                        eprintln!(
                            "atvv-bridge: could not activate the desktop application: {error}"
                        );
                    }
                }
                DesktopEvent::ButtonMappingRequested => {
                    if let Err(error) = desktop.activate(&mut *shell) {
                        eprintln!(
                            "atvv-bridge: could not activate the desktop application: {error}"
                        );
                    }
                    desktop.button_mapping_event(ButtonMappingEvent::Open, &mut *shell);
                }
                DesktopEvent::ButtonMapping(event) => {
                    desktop.button_mapping_event(event, &mut *shell);
                }
                DesktopEvent::CloseRequested => desktop.close_requested(&mut *shell),
                DesktopEvent::CloseConfirmed(confirmed) => {
                    desktop.close_confirmed(confirmed, &mut *shell);
                }
                DesktopEvent::QuitRequested => desktop.quit_requested(&mut *shell),
            }
        }
        desktop.refresh_status(&mut *shell);
        desktop.refresh_button_mapping(&mut *shell);
        gtk::glib::ControlFlow::Continue
    });
    application.run_with_args(&["atvv-bridge"]);
}

struct GtkDesktopShell {
    application: Option<adw::Application>,
    window: Option<adw::ApplicationWindow>,
    status_labels: Option<StatusLabels>,
    page_stack: Option<gtk::Stack>,
    mapping_widgets: Option<MappingWidgets>,
    event_sender: mpsc::Sender<DesktopEvent>,
    tray_available: Arc<AtomicBool>,
    tray: Option<ksni::blocking::Handle<StatusTray>>,
}

struct StatusLabels {
    actionable_failure: gtk::Label,
    remote: gtk::Label,
    profile: gtk::Label,
    capture: gtk::Label,
    wav_handoff: gtk::Label,
    handoff: gtk::Label,
    recovery: gtk::Label,
    battery: gtk::Label,
    diagnostics: gtk::Label,
}

struct MappingWidgets {
    rows: Vec<MappingRow>,
    notice: gtk::Label,
    progress: gtk::Spinner,
    apply: gtk::Button,
    reset: gtk::Button,
    reload: gtk::Button,
    retry: gtk::Button,
    updating: Rc<Cell<bool>>,
    targets: Rc<Vec<MappingTarget>>,
}

struct MappingRow {
    id: ButtonId,
    selector: gtk::DropDown,
    mapping_details: gtk::Label,
}

impl GtkDesktopShell {
    fn new(event_sender: mpsc::Sender<DesktopEvent>) -> Self {
        Self {
            application: None,
            window: None,
            status_labels: None,
            page_stack: None,
            mapping_widgets: None,
            event_sender,
            tray_available: Arc::new(AtomicBool::new(false)),
            tray: None,
        }
    }

    fn set_application(&mut self, application: &adw::Application) {
        self.application = Some(application.clone());
    }

    fn start_tray(&mut self) {
        use ksni::blocking::TrayMethods;

        let available = Arc::new(AtomicBool::new(false));
        let tray = StatusTray {
            event_sender: self.event_sender.clone(),
            available: Arc::clone(&available),
        };
        match tray.spawn() {
            Ok(handle) => {
                self.tray_available = available;
                self.tray = Some(handle);
            }
            Err(error) => {
                self.tray_available.store(false, Ordering::Relaxed);
                eprintln!("atvv-bridge: system tray unavailable: {error}");
            }
        }
    }

    fn build_mapping_page(&self) -> (gtk::ScrolledWindow, MappingWidgets) {
        let targets = Rc::new(mapping_targets());
        let labels: Vec<String> = targets.iter().map(|target| target_label(*target)).collect();
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let model = gtk::StringList::new(&label_refs);
        let updating = Rc::new(Cell::new(true));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(18);
        content.set_margin_end(18);

        let notice = gtk::Label::builder().xalign(0.0).wrap(true).build();
        notice.add_css_class("caption");
        content.append(&notice);

        let mut rows = Vec::new();
        for (group, buttons) in mapping_groups() {
            let group_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            group_box.add_css_class("boxed-list");
            for id in buttons {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                row.set_margin_top(10);
                row.set_margin_bottom(10);
                row.set_margin_start(12);
                row.set_margin_end(12);
                let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
                text.set_hexpand(true);
                let title = gtk::Label::builder()
                    .label(button_label(id))
                    .xalign(0.0)
                    .build();
                title.add_css_class("heading");
                let mapping_details = gtk::Label::builder()
                    .label(format!("Native: {}", id.native_key()))
                    .xalign(0.0)
                    .wrap(true)
                    .build();
                mapping_details.add_css_class("caption");
                text.append(&title);
                text.append(&mapping_details);
                let selector = gtk::DropDown::builder()
                    .model(&model)
                    .enable_search(true)
                    .build();
                selector
                    .set_tooltip_text(Some(&format!("Map {} to a logical key", button_label(id))));
                selector.update_property(&[gtk::accessible::Property::Label(&format!(
                    "{} logical key",
                    button_label(id)
                ))]);
                let sender = self.event_sender.clone();
                let targets_for_change = Rc::clone(&targets);
                let updating_for_change = Rc::clone(&updating);
                selector.connect_selected_notify(move |selector| {
                    if updating_for_change.get() {
                        return;
                    }
                    let Some(target) = targets_for_change.get(selector.selected() as usize).copied() else {
                        return;
                    };
                    if id == ButtonId::Power && target == MappingTarget::Original {
                        let dialog = adw::AlertDialog::builder()
                            .heading("Enable the native Power key?")
                            .body("The remote's native KEY_POWER action may suspend or shut down this PC. Reset All returns Power to Disabled.")
                            .build();
                        dialog.add_responses(&[("cancel", "Cancel"), ("enable", "Keep Original")]);
                        dialog.set_default_response(Some("cancel"));
                        dialog.set_close_response("cancel");
                        dialog.set_response_appearance("enable", adw::ResponseAppearance::Destructive);
                        let sender = sender.clone();
                        let parent = selector.root().and_downcast::<gtk::Window>();
                        dialog.choose(parent.as_ref(), None::<&gtk::gio::Cancellable>, move |response| {
                            let event = if response == "enable" {
                                ButtonMappingEvent::Edit(id, target)
                            } else {
                                ButtonMappingEvent::Cancel
                            };
                            let _ = sender.send(DesktopEvent::ButtonMapping(event));
                        });
                    } else {
                        let _ = sender.send(DesktopEvent::ButtonMapping(
                            ButtonMappingEvent::Edit(id, target),
                        ));
                    }
                });
                row.append(&text);
                row.append(&selector);
                group_box.append(&row);
                rows.push(MappingRow {
                    id,
                    selector,
                    mapping_details,
                });
            }
            let frame = gtk::Frame::builder().label(group).child(&group_box).build();
            content.append(&frame);
        }

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let progress = gtk::Spinner::new();
        let retry = action_button("Retry", ButtonMappingEvent::Retry, &self.event_sender);
        let reload = action_button("Reload", ButtonMappingEvent::Reload, &self.event_sender);
        let reset = action_button("Reset All", ButtonMappingEvent::Reset, &self.event_sender);
        let apply = action_button("Apply", ButtonMappingEvent::Apply, &self.event_sender);
        apply.add_css_class("suggested-action");
        actions.append(&progress);
        actions.append(&retry);
        actions.append(&reload);
        actions.append(&reset);
        actions.append(&apply);
        content.append(&actions);
        updating.set(false);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&content)
            .build();
        (
            scrolled,
            MappingWidgets {
                rows,
                notice,
                progress,
                apply,
                reset,
                reload,
                retry,
                updating,
                targets,
            },
        )
    }
}

fn action_button(
    label: &str,
    event: ButtonMappingEvent,
    sender: &mpsc::Sender<DesktopEvent>,
) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    let sender = sender.clone();
    button.connect_clicked(move |_| {
        let _ = sender.send(DesktopEvent::ButtonMapping(event.clone()));
    });
    button
}

fn mapping_targets() -> Vec<MappingTarget> {
    const COMMON: [&str; 8] = [
        "KEY_ENTER",
        "KEY_ESC",
        "KEY_HOME",
        "KEY_MENU",
        "KEY_PLAYPAUSE",
        "KEY_TV",
        "KEY_VOLUMEUP",
        "KEY_VOLUMEDOWN",
    ];
    let mut targets = vec![MappingTarget::Original, MappingTarget::Disabled];
    for symbol in COMMON {
        if let Some(key) = LOGICAL_KEYS.iter().find(|key| key.symbol() == symbol) {
            targets.push(MappingTarget::Key(*key));
        }
    }
    for key in LOGICAL_KEYS {
        let target = MappingTarget::Key(*key);
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets
}

fn target_label(target: MappingTarget) -> String {
    match target {
        MappingTarget::Original => "Keep Original".into(),
        MappingTarget::Disabled => "Disabled".into(),
        MappingTarget::Key(key) => format!("{} — {}", key.label(), key.symbol()),
    }
}

fn button_label(id: ButtonId) -> &'static str {
    match id {
        ButtonId::Power => "Power",
        ButtonId::Confirm => "Confirm",
        ButtonId::Up => "Up",
        ButtonId::Down => "Down",
        ButtonId::Left => "Left",
        ButtonId::Right => "Right",
        ButtonId::Back => "Back",
        ButtonId::VolumeUp => "Volume Up",
        ButtonId::VolumeDown => "Volume Down",
        ButtonId::Menu => "Menu",
        ButtonId::Live => "Live",
    }
}

fn mapping_groups() -> [(&'static str, Vec<ButtonId>); 4] {
    [
        ("Device", vec![ButtonId::Power]),
        (
            "Navigation",
            vec![
                ButtonId::Confirm,
                ButtonId::Up,
                ButtonId::Down,
                ButtonId::Left,
                ButtonId::Right,
                ButtonId::Back,
            ],
        ),
        ("Volume", vec![ButtonId::VolumeUp, ButtonId::VolumeDown]),
        ("Special", vec![ButtonId::Menu, ButtonId::Live]),
    ]
}

struct StatusTray {
    event_sender: mpsc::Sender<DesktopEvent>,
    available: Arc<AtomicBool>,
}

impl StatusTray {
    fn request_activation(&self) {
        let _ = self.event_sender.send(DesktopEvent::ActivateRequested);
    }
}

impl ksni::Tray for StatusTray {
    fn id(&self) -> String {
        "atvv-bridge".into()
    }

    fn title(&self) -> String {
        "ATVV Voice Bridge".into()
    }

    fn icon_name(&self) -> String {
        "preferences-desktop-peripherals-symbolic".into()
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.request_activation();
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;

        vec![
            StandardItem {
                label: "Show Status".into(),
                activate: Box::new(|tray: &mut Self| {
                    tray.request_activation();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Button Mapping".into(),
                icon_name: "input-keyboard-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.event_sender.send(DesktopEvent::ButtonMappingRequested);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.event_sender.send(DesktopEvent::QuitRequested);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_online(&self) {
        self.available.store(true, Ordering::Relaxed);
    }

    fn watcher_offline(&self, _reason: ksni::OfflineReason) -> bool {
        self.available.store(false, Ordering::Relaxed);
        self.request_activation();
        true
    }
}

impl DesktopShell for GtkDesktopShell {
    fn create_status_window_with_close_action(&mut self) {
        let application = self
            .application
            .as_ref()
            .expect("GTK application is set before activation");
        let statuses = gtk::Box::new(gtk::Orientation::Vertical, 8);
        statuses.set_margin_top(24);
        statuses.set_margin_bottom(24);
        statuses.set_margin_start(24);
        statuses.set_margin_end(24);
        let status_label = |text| {
            gtk::Label::builder()
                .label(text)
                .xalign(0.0)
                .selectable(true)
                .wrap(true)
                .build()
        };
        let status_labels = StatusLabels {
            actionable_failure: status_label(""),
            remote: status_label(""),
            profile: status_label(""),
            capture: status_label(""),
            wav_handoff: status_label(""),
            handoff: status_label(""),
            recovery: status_label(""),
            battery: status_label(""),
            diagnostics: status_label(""),
        };
        statuses.append(&status_labels.actionable_failure);
        statuses.append(&status_labels.remote);
        statuses.append(&status_labels.profile);
        statuses.append(&status_labels.capture);
        statuses.append(&status_labels.wav_handoff);
        statuses.append(&status_labels.handoff);
        statuses.append(&status_labels.recovery);
        statuses.append(&status_labels.battery);
        let diagnostics = gtk::Expander::builder()
            .label("Diagnostics")
            .child(&status_labels.diagnostics)
            .build();
        statuses.append(&diagnostics);
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        stack.add_titled(&statuses, Some("status"), "Status");
        let (mapping_page, mapping_widgets) = self.build_mapping_page();
        stack.add_titled(&mapping_page, Some("button-mapping"), "Button Mapping");
        let page_sender = self.event_sender.clone();
        stack.connect_visible_child_name_notify(move |stack| {
            if stack.visible_child_name().as_deref() == Some("button-mapping") {
                let _ = page_sender.send(DesktopEvent::ButtonMapping(ButtonMappingEvent::Open));
            }
        });
        let header_bar = adw::HeaderBar::builder()
            .show_start_title_buttons(true)
            .show_end_title_buttons(true)
            .build();
        let switcher = gtk::StackSwitcher::builder().stack(&stack).build();
        switcher.update_property(&[gtk::accessible::Property::Label("Application page")]);
        header_bar.set_title_widget(Some(&switcher));
        let window_content = adw::ToolbarView::new();
        window_content.add_top_bar(&header_bar);
        window_content.set_content(Some(&stack));
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("ATVV Voice Bridge")
            .default_width(680)
            .default_height(720)
            .content(&window_content)
            .build();
        let event_sender = self.event_sender.clone();
        window.connect_close_request(move |_| {
            let _ = event_sender.send(DesktopEvent::CloseRequested);
            gtk::glib::Propagation::Stop
        });
        self.window = Some(window);
        self.status_labels = Some(status_labels);
        self.page_stack = Some(stack);
        self.mapping_widgets = Some(mapping_widgets);
        self.start_tray();
        self.display_status(&DesktopStatus::default());
    }

    fn present_status_window(&mut self) {
        self.page_stack
            .as_ref()
            .expect("page stack is created before presentation")
            .set_visible_child_name("status");
        self.window
            .as_ref()
            .expect("status window is created before presentation")
            .present();
    }

    fn display_status(&mut self, status: &DesktopStatus) {
        let remote = match &status.remote {
            RemoteStatus::Waiting => "ATVV Remote: Waiting",
            RemoteStatus::Connected { .. } => "ATVV Remote: Connected",
        };
        let profile = match status.profile {
            AtvvProfileReadiness::Waiting => "ATVV Profile: Waiting",
            AtvvProfileReadiness::Ready { .. } => "ATVV Profile: Ready",
            AtvvProfileReadiness::Unsupported { .. } => "ATVV Profile: Unsupported",
        };
        let capture = match status.capture {
            CaptureStatus::Idle => "Capture: Idle",
            CaptureStatus::Active => "Capture: Active",
        };
        let wav_handoff = match status.wav_handoff {
            WavHandoffActivity::Idle => "WAV Handoff: Idle",
            WavHandoffActivity::Active => "WAV Handoff: Active",
        };
        let handoff = match &status.recent_wav_handoff {
            RecentWavHandoff::NoOutcome => "Recent WAV Handoff: No outcome".into(),
            RecentWavHandoff::Succeeded {
                outcome: WavHandoffOutcome::TextCommitted,
            } => "Recent WAV Handoff: Succeeded (text committed)".into(),
            RecentWavHandoff::Succeeded {
                outcome: WavHandoffOutcome::NoSpeech,
            } => "Recent WAV Handoff: Succeeded (no speech)".into(),
            RecentWavHandoff::Failed { stage, .. } => {
                format!("Recent WAV Handoff: Failed ({stage:?})")
            }
        };
        let recovery = match &status.recovery {
            RecoveryStatus::Idle => "Recovery: Idle".into(),
            RecoveryStatus::Retrying {
                next_attempt_at, ..
            } => format!(
                "Recovery: Next attempt at Unix ms {}",
                next_attempt_at
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
        };
        let battery = match status.battery {
            BatteryStatus::Unknown => "Battery: Unknown".into(),
            BatteryStatus::Percentage(percentage) => format!("Battery: {}%", percentage.get()),
        };
        let actionable_failure = status
            .actionable_failure
            .as_ref()
            .map(|failure| format!("Bridge Status: {}. {}", failure.summary, failure.action))
            .unwrap_or_else(|| "Bridge Status: Operational".into());
        let mut diagnostics = Vec::new();
        if let Some(failure) = &status.actionable_failure {
            diagnostics.push(failure.diagnostics.clone());
        }
        if let RecoveryStatus::Retrying { failure, .. } = &status.recovery {
            diagnostics.push(format!("ATVV Remote recovery: {failure}"));
        }
        if let RecentWavHandoff::Failed { stage, error } = &status.recent_wav_handoff {
            diagnostics.push(format!("WAV Handoff {stage:?}: {error}"));
        }
        let diagnostics = if diagnostics.is_empty() {
            "No diagnostics available.".into()
        } else {
            diagnostics.join("\n")
        };
        let labels = self
            .status_labels
            .as_ref()
            .expect("status window is created before status display");
        labels.actionable_failure.set_label(&actionable_failure);
        labels.remote.set_label(remote);
        labels.profile.set_label(profile);
        labels.capture.set_label(capture);
        labels.wav_handoff.set_label(wav_handoff);
        labels.handoff.set_label(&handoff);
        labels.recovery.set_label(&recovery);
        labels.battery.set_label(&battery);
        labels.diagnostics.set_label(&diagnostics);
    }

    fn tray_available(&self) -> bool {
        self.tray_available.load(Ordering::Relaxed)
    }

    fn hide_status_window(&mut self) {
        self.window
            .as_ref()
            .expect("status window is created before it is hidden")
            .set_visible(false);
    }

    fn confirm_close_quits_bridge(&mut self) {
        let dialog = adw::AlertDialog::builder()
            .heading("Quit ATVV Voice Bridge?")
            .body("No system tray is available. Closing the window will stop voice input.")
            .build();
        dialog.add_responses(&[("cancel", "Cancel"), ("quit", "Quit")]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
        let event_sender = self.event_sender.clone();
        dialog.choose(
            self.window.as_ref(),
            None::<&gtk::gio::Cancellable>,
            move |response| {
                let _ = event_sender.send(DesktopEvent::CloseConfirmed(response == "quit"));
            },
        );
    }

    fn quit(&mut self) {
        self.application
            .as_ref()
            .expect("GTK application is set before quit")
            .quit();
    }

    fn render_button_mapping(&mut self, presentation: &ButtonMappingPresentation) {
        let widgets = self
            .mapping_widgets
            .as_ref()
            .expect("button mapping page is created before rendering");
        let draft = presentation.draft.as_ref();
        let editable = matches!(
            presentation.state,
            ButtonMappingState::Ready { .. } | ButtonMappingState::Validation { .. }
        );
        widgets.updating.set(true);
        for row in &widgets.rows {
            row.selector.set_sensitive(editable);
            if let Some(draft) = draft {
                let target = draft.get(row.id);
                if let Some(index) = widgets
                    .targets
                    .iter()
                    .position(|candidate| *candidate == target)
                {
                    row.selector.set_selected(index as u32);
                }
                row.mapping_details.set_label(&format!(
                    "Native: {}  •  Draft: {}",
                    row.id.native_key(),
                    target_label(target)
                ));
            } else {
                row.mapping_details
                    .set_label(&format!("Native: {}", row.id.native_key()));
            }
        }
        widgets.updating.set(false);

        let busy = matches!(
            presentation.state,
            ButtonMappingState::Inspecting
                | ButtonMappingState::Applying
                | ButtonMappingState::Resetting
        );
        if busy {
            widgets.progress.start();
        } else {
            widgets.progress.stop();
        }
        widgets.apply.set_sensitive(presentation.can_apply);
        widgets.reset.set_sensitive(presentation.can_reset);
        widgets.reset.set_label(
            if matches!(presentation.state, ButtonMappingState::RecoveryRequired) {
                "Restore Defaults"
            } else {
                "Reset All"
            },
        );
        widgets.reload.set_visible(matches!(
            presentation.state,
            ButtonMappingState::Conflict { .. }
        ));
        widgets.retry.set_visible(matches!(
            presentation.state,
            ButtonMappingState::Unavailable
        ));
        let state_notice = match &presentation.state {
            ButtonMappingState::Unloaded => "Open Button Mapping to inspect the installed mapping.",
            ButtonMappingState::Inspecting => "Loading the installed button mapping…",
            ButtonMappingState::Ready { draft, .. }
                if draft.get(ButtonId::Power) == MappingTarget::Original =>
            {
                "Warning: native KEY_POWER may suspend or shut down this PC. Reset All returns Power to Disabled."
            }
            ButtonMappingState::Ready { .. } if presentation.dirty => {
                "Changes are staged. Apply writes them to the system."
            }
            ButtonMappingState::Ready { .. } => "The installed mapping is up to date.",
            ButtonMappingState::Applying => {
                "Authorization is required in the system prompt. Canceling it preserves staged edits; after approval, the mapping will be applied…"
            }
            ButtonMappingState::Resetting => {
                "Authorization is required in the system prompt. Canceling it preserves staged edits; after approval, defaults will be restored…"
            }
            ButtonMappingState::Conflict { .. } => {
                "The installed mapping changed elsewhere. Reload before editing or applying; reload discards this draft."
            }
            ButtonMappingState::Validation { .. } => {
                "The staged mapping is invalid. Change a selector or restore defaults."
            }
            ButtonMappingState::RecoveryRequired => {
                "Mapping storage needs recovery. Restore defaults to repair it."
            }
            ButtonMappingState::Unavailable => {
                "Button Mapping is unavailable. Voice input remains available."
            }
        };
        widgets
            .notice
            .set_label(presentation.notice.as_deref().unwrap_or(state_notice));
    }

    fn perform_button_mapping_effect(&mut self, effect: ButtonMappingEffect) {
        match effect {
            ButtonMappingEffect::Open => {
                self.page_stack
                    .as_ref()
                    .expect("page stack exists before navigation")
                    .set_visible_child_name("button-mapping");
                self.window
                    .as_ref()
                    .expect("window exists before navigation")
                    .present();
            }
            ButtonMappingEffect::ConfirmReset => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Restore all button defaults?")
                    .body("This removes every custom mapping and returns Power to Disabled.")
                    .build();
                dialog.add_responses(&[("cancel", "Cancel"), ("reset", "Reset All")]);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
                let sender = self.event_sender.clone();
                dialog.choose(
                    self.window.as_ref(),
                    None::<&gtk::gio::Cancellable>,
                    move |response| {
                        let event = if response == "reset" {
                            ButtonMappingEvent::ConfirmReset
                        } else {
                            ButtonMappingEvent::Cancel
                        };
                        let _ = sender.send(DesktopEvent::ButtonMapping(event));
                    },
                );
            }
            ButtonMappingEffect::ConfirmReload => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Reload the installed mapping?")
                    .body("Reloading discards all staged edits in this draft.")
                    .build();
                dialog.add_responses(&[("cancel", "Cancel"), ("reload", "Reload")]);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                let sender = self.event_sender.clone();
                dialog.choose(
                    self.window.as_ref(),
                    None::<&gtk::gio::Cancellable>,
                    move |response| {
                        let event = if response == "reload" {
                            ButtonMappingEvent::ConfirmReload
                        } else {
                            ButtonMappingEvent::Cancel
                        };
                        let _ = sender.send(DesktopEvent::ButtonMapping(event));
                    },
                );
            }
            ButtonMappingEffect::AuthorizationRequired => {
                if let Some(widgets) = &self.mapping_widgets {
                    widgets.notice.set_label(
                        "Complete or cancel authorization in the system prompt. Cancellation preserves staged edits.",
                    );
                }
            }
            ButtonMappingEffect::Render
            | ButtonMappingEffect::Hide
            | ButtonMappingEffect::ConfirmQuit
            | ButtonMappingEffect::Quit => {}
        }
    }
}
