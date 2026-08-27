use std::{env, path::PathBuf, process::ExitCode};

use atvv_bridge::{
    Application, ConfigSelection, Readiness, check_readiness, system::SystemBoundaries,
};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    #[arg(long)]
    check: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut boundaries = SystemBoundaries::default();
    if cli.check {
        return match check_readiness(&mut boundaries) {
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
        };
    }
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
