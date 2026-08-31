use std::env;
use std::{
    cell::RefCell,
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
    AtvvProfileReadiness, BatteryStatus, CaptureStatus, DesktopApplication, DesktopShell,
    DesktopStatus, InProcessVoiceBridge, RecentWavHandoff, RecoveryStatus, RemoteStatus,
    WavHandoffActivity, WavHandoffOutcome,
};

#[derive(Debug, Clone, Copy)]
enum DesktopEvent {
    ActivateRequested,
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

impl GtkDesktopShell {
    fn new(event_sender: mpsc::Sender<DesktopEvent>) -> Self {
        Self {
            application: None,
            window: None,
            status_labels: None,
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
        let header_bar = adw::HeaderBar::builder()
            .show_start_title_buttons(true)
            .show_end_title_buttons(true)
            .build();
        let window_content = adw::ToolbarView::new();
        window_content.add_top_bar(&header_bar);
        window_content.set_content(Some(&statuses));
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("ATVV Voice Bridge")
            .default_width(360)
            .default_height(220)
            .content(&window_content)
            .build();
        let event_sender = self.event_sender.clone();
        window.connect_close_request(move |_| {
            let _ = event_sender.send(DesktopEvent::CloseRequested);
            gtk::glib::Propagation::Stop
        });
        self.window = Some(window);
        self.status_labels = Some(status_labels);
        self.start_tray();
        self.display_status(&DesktopStatus::default());
    }

    fn present_status_window(&mut self) {
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
}
