#[cfg(feature = "desktop")]
use std::{cell::RefCell, rc::Rc};
use std::{env, path::PathBuf, process::ExitCode};

#[cfg(feature = "desktop")]
use adw::prelude::*;
#[cfg(not(feature = "desktop"))]
use atvv_bridge::Application;
use atvv_bridge::{ConfigSelection, Readiness, check_readiness, system::SystemBoundaries};
#[cfg(feature = "desktop")]
use atvv_bridge::{DesktopApplication, DesktopShell, InProcessVoiceBridge};
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
    application.connect_activate(move |application| {
        shell.borrow_mut().set_application(application);
        if let Err(error) = desktop.borrow_mut().activate(&mut *shell.borrow_mut()) {
            eprintln!("atvv-bridge: could not start the desktop application: {error}");
        }
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
        self.window = Some(
            adw::ApplicationWindow::builder()
                .application(application)
                .title("ATVV Voice Bridge")
                .default_width(360)
                .default_height(180)
                .content(
                    &gtk::Label::builder()
                        .label("ATVV Voice Bridge is starting.")
                        .margin_top(24)
                        .margin_bottom(24)
                        .margin_start(24)
                        .margin_end(24)
                        .build(),
                )
                .build(),
        );
    }

    fn present_status_window(&mut self) {
        self.window
            .as_ref()
            .expect("status window is created before presentation")
            .present();
    }
}
