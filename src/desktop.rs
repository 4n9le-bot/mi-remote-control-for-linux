use std::{io, sync::mpsc, thread};

use crate::{Application, ConfigSelection, StartupError, system::SystemBoundaries};

/// The desktop-facing lifecycle of the ATVV Voice Bridge.
pub trait VoiceBridge {
    fn start(&mut self) -> io::Result<()>;
}

/// The desktop operations the application needs, independent of GTK widgets.
pub trait DesktopShell {
    fn create_status_window(&mut self);
    fn present_status_window(&mut self);
}

/// A single desktop application that owns one ATVV Voice Bridge and status window.
pub struct DesktopApplication<B> {
    bridge: B,
    started: bool,
}

impl<B> DesktopApplication<B>
where
    B: VoiceBridge,
{
    pub fn new(bridge: B) -> Self {
        Self {
            bridge,
            started: false,
        }
    }

    pub fn activate(&mut self, shell: &mut impl DesktopShell) -> io::Result<()> {
        if !self.started {
            self.bridge.start()?;
            self.started = true;
            shell.create_status_window();
        }
        shell.present_status_window();
        Ok(())
    }

    pub fn bridge(&self) -> &B {
        &self.bridge
    }
}

/// Starts the production bridge on its own in-process thread.
pub struct InProcessVoiceBridge {
    selection: Option<ConfigSelection>,
}

impl InProcessVoiceBridge {
    pub fn new(selection: ConfigSelection) -> Self {
        Self {
            selection: Some(selection),
        }
    }
}

impl VoiceBridge for InProcessVoiceBridge {
    fn start(&mut self) -> io::Result<()> {
        let selection = self.selection.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::AlreadyExists, "ATVV Voice Bridge started")
        })?;
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("atvv-voice-bridge".into())
            .spawn(move || {
                let mut boundaries = SystemBoundaries::default();
                let result = Application::start(selection, &mut boundaries);
                match result {
                    Ok(application) => {
                        let _ = started_tx.send(Ok(()));
                        let _ = application.run();
                    }
                    Err(error) => {
                        let _ = started_tx.send(Err(startup_error_to_io(error)));
                    }
                }
            })?;
        started_rx
            .recv()
            .map_err(|_| io::Error::other("ATVV Voice Bridge stopped during startup"))?
    }
}

fn startup_error_to_io(error: StartupError) -> io::Error {
    io::Error::other(error)
}
