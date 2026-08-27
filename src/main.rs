use std::{env, path::PathBuf, process::ExitCode};

use atvv_bridge::{Application, ConfigSelection, system::SystemBoundaries};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let selection = ConfigSelection::resolve(
        cli.config,
        env::var_os("XDG_CONFIG_HOME").as_deref(),
        env::var_os("HOME").as_deref(),
    );
    let mut boundaries = SystemBoundaries;

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
