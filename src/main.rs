#[cfg(feature = "desktop")]
use std::{cell::RefCell, rc::Rc, time::Duration};
use std::{env, path::PathBuf, process::ExitCode};

#[cfg(feature = "desktop")]
use adw::prelude::*;
#[cfg(not(feature = "desktop"))]
use atvv_bridge::Application;
#[cfg(feature = "desktop")]
use atvv_bridge::{
    AtvvProfileReadiness, BatteryStatus, CaptureStatus, DesktopApplication, DesktopShell,
    DesktopStatus, InProcessVoiceBridge, RecentWavHandoff, RecoveryStatus, RemoteStatus,
    WavHandoffActivity, WavHandoffOutcome,
};
use atvv_bridge::{ConfigSelection, Readiness, check_readiness, system::SystemBoundaries};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long)]
    check: bool,
}

#[cfg(feature = "desktop")]
fn main() {
    let cli = Cli::parse();
    if cli.check {
        let exit_code = run_check();
        if exit_code == ExitCode::SUCCESS {
            return;
        }
        std::process::exit(1);
    }
    let selection = ConfigSelection::resolve(
        cli.config,
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    );
    let desktop = Rc::new(RefCell::new(DesktopApplication::new(
        InProcessVoiceBridge::new(selection),
    )));
    let shell = Rc::new(RefCell::new(GtkDesktopShell::default()));
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
        desktop
            .borrow_mut()
            .refresh_status(&mut *shell.borrow_mut());
        gtk::glib::ControlFlow::Continue
    });
    application.run_with_args(&["atvv-bridge"]);
}

#[cfg(not(feature = "desktop"))]
fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.check {
        return run_check();
    }
    let mut boundaries = SystemBoundaries::default();
    let selection = ConfigSelection::resolve(
        cli.config,
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    );
    match Application::start(selection, &mut boundaries) {
        Ok(application) => match application.run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("atvv-bridge: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("atvv-bridge: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_check() -> ExitCode {
    let mut boundaries = SystemBoundaries::default();
    match check_readiness(&mut boundaries) {
        Ok(Readiness::Ready { address }) => {
            eprintln!("event=check_selected_atvv_remote address={address:?} ready=true");
            ExitCode::SUCCESS
        }
        Ok(Readiness::NotReady { address, reason }) => {
            if let Some(address) = address {
                eprintln!(
                    "event=check_selected_atvv_remote address={address:?} ready=false reason={reason:?}"
                );
            } else {
                eprintln!("event=check_failed ready=false reason={reason:?}");
            }
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("atvv-bridge: BlueZ readiness check failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "desktop")]
#[derive(Default)]
struct GtkDesktopShell {
    application: Option<adw::Application>,
    window: Option<adw::ApplicationWindow>,
    status_labels: Option<StatusLabels>,
}

#[cfg(feature = "desktop")]
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

#[cfg(feature = "desktop")]
impl GtkDesktopShell {
    fn set_application(&mut self, application: &adw::Application) {
        self.application = Some(application.clone());
    }
}

#[cfg(feature = "desktop")]
impl DesktopShell for GtkDesktopShell {
    fn create_status_window(&mut self) {
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
        self.window = Some(
            adw::ApplicationWindow::builder()
                .application(application)
                .title("ATVV Voice Bridge")
                .default_width(360)
                .default_height(220)
                .content(&statuses)
                .build(),
        );
        self.status_labels = Some(status_labels);
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
}
