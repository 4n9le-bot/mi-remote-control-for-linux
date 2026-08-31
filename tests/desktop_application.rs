use std::io;

use atvv_bridge::{DesktopApplication, DesktopShell, VoiceBridge};

#[derive(Default)]
struct FakeBridge {
    starts: usize,
}

impl VoiceBridge for FakeBridge {
    fn start(&mut self) -> io::Result<()> {
        self.starts += 1;
        Ok(())
    }
}

#[derive(Default)]
struct FakeDesktopShell {
    windows_created: usize,
    windows_presented: usize,
}

impl DesktopShell for FakeDesktopShell {
    fn create_status_window(&mut self) {
        self.windows_created += 1;
    }

    fn present_status_window(&mut self) {
        self.windows_presented += 1;
    }
}

#[test]
fn repeated_activation_reuses_the_bridge_and_status_window() {
    let mut application = DesktopApplication::new(FakeBridge::default());
    let mut shell = FakeDesktopShell::default();

    application
        .activate(&mut shell)
        .expect("the first desktop activation should start the bridge");
    application
        .activate(&mut shell)
        .expect("the second desktop activation should reuse the running application");

    assert_eq!(application.bridge().starts, 1);
    assert_eq!(shell.windows_created, 1);
    assert_eq!(shell.windows_presented, 2);
}
