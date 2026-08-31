use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use atvv_bridge::{
    button_mapping::{ButtonId, Mapping, MappingTarget, logical_key_by_symbol},
    helper_protocol::{DecodedResponse, Request, decode_response, encode_request},
};

const REMOTE_MODALIAS: &str = "evdev:input:b0005v2717p32B8e00A4-e0,1,4,14,k71,72,73,74,75";

struct IsolatedHelper {
    _directory: tempfile::TempDir,
    root: PathBuf,
    helper: PathBuf,
    systemd_hwdb: PathBuf,
    multiarch_lib: PathBuf,
}

impl IsolatedHelper {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("an isolated helper root should be created");
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = directory.path().join("root");
        for relative in [
            "",
            "usr",
            "usr/lib",
            "usr/lib/udev",
            "usr/lib/udev/hwdb.d",
            "etc",
            "etc/udev",
            "etc/udev/hwdb.d",
        ] {
            let path = root.join(relative);
            fs::create_dir_all(&path).expect("the isolated hwdb layout should be created");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("isolated hwdb directories should have trusted modes");
        }
        fs::copy(
            repository.join("packaging/90-atvv-bridge.hwdb"),
            root.join("usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb"),
        )
        .expect("the package hwdb source should be installed in the isolated root");
        fs::set_permissions(
            root.join("usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();

        let helper = directory.path().join("atvv-button-mapping-helper");
        fs::copy(env!("CARGO_BIN_EXE_atvv-button-mapping-helper"), &helper)
            .expect("the helper executable should be copied into the namespace");
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
        let systemd_hwdb = directory.path().join("systemd-hwdb");
        fs::copy("/usr/bin/systemd-hwdb", &systemd_hwdb)
            .expect("systemd-hwdb should be copied into the namespace");
        fs::set_permissions(&systemd_hwdb, fs::Permissions::from_mode(0o755)).unwrap();

        let multiarch = Command::new("dpkg-architecture")
            .arg("-qDEB_HOST_MULTIARCH")
            .output()
            .expect("dpkg-architecture should report the host library directory");
        assert!(multiarch.status.success());
        let multiarch = String::from_utf8(multiarch.stdout).unwrap();
        let multiarch_lib = PathBuf::from("/usr/lib").join(multiarch.trim());

        let status = Command::new("systemd-hwdb")
            .arg(format!("--root={}", root.display()))
            .args(["--strict", "update"])
            .status()
            .expect("the package defaults should compile in the isolated root");
        assert!(status.success());

        Self {
            _directory: directory,
            root,
            helper,
            systemd_hwdb,
            multiarch_lib,
        }
    }

    fn invoke(&self, argument: &str, request: &Request) -> DecodedResponse {
        let mut command = Command::new("bwrap");
        command
            .args(["--unshare-user", "--uid", "0", "--gid", "0", "--tmpfs", "/"])
            .args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"])
            .args(["--dir", "/bin", "--dir", "/lib", "--dir", "/lib64"])
            .args([
                "--dir",
                "/usr",
                "--dir",
                "/usr/bin",
                "--dir",
                "/usr/lib",
                "--dir",
                "/usr/lib/udev",
            ])
            .args(["--dir", "/etc", "--dir", "/etc/udev"])
            .args(["--ro-bind", "/lib", "/lib"])
            .arg("--ro-bind")
            .arg(&self.multiarch_lib)
            .arg(&self.multiarch_lib)
            .args(["--ro-bind"])
            .arg(&self.systemd_hwdb)
            .arg("/usr/bin/systemd-hwdb")
            .args(["--bind"])
            .arg(self.root.join("usr/lib/udev/hwdb.d"))
            .arg("/usr/lib/udev/hwdb.d")
            .args(["--bind"])
            .arg(self.root.join("etc/udev"))
            .arg("/etc/udev")
            .args(["--ro-bind"])
            .arg(&self.helper)
            .arg("/bin/atvv-button-mapping-helper");
        if Path::new("/lib64").exists() {
            command.args(["--ro-bind", "/lib64", "/lib64"]);
        }
        command.args(["/bin/atvv-button-mapping-helper", argument]);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the helper should start inside the isolated namespace");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&encode_request(request).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "isolated helper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        decode_response(&output.stdout).expect("the helper should return one protocol response")
    }

    fn query_remote(&self) -> String {
        let output = Command::new("systemd-hwdb")
            .arg(format!("--root={}", self.root.display()))
            .args(["query", REMOTE_MODALIAS])
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }
}

fn mapping_with_menu_home() -> Mapping {
    let home = logical_key_by_symbol("KEY_HOME").expect("KEY_HOME should be in the catalog");
    Mapping::from_entries(Mapping::defaults().iter().map(|(button, target)| {
        (
            button,
            if button == ButtonId::Menu {
                MappingTarget::Key(home)
            } else {
                target
            },
        )
    }))
    .unwrap()
}

#[test]
fn desktop_requests_cross_the_helper_and_mutate_only_an_isolated_hwdb_root() {
    let isolated = IsolatedHelper::new();
    let DecodedResponse::Inspect { revision, mapping } =
        isolated.invoke("inspect", &Request::Inspect)
    else {
        panic!("defaults should inspect successfully");
    };
    assert_eq!(mapping, Mapping::defaults());

    let custom = mapping_with_menu_home();
    assert!(matches!(
        isolated.invoke(
            "apply",
            &Request::Apply {
                expected_revision: revision,
                mapping: custom.clone(),
            },
        ),
        DecodedResponse::Apply { .. }
    ));
    let DecodedResponse::Inspect { mapping, .. } = isolated.invoke("inspect", &Request::Inspect)
    else {
        panic!("installed mapping should inspect successfully");
    };
    assert_eq!(mapping, custom);
    let properties = isolated.query_remote();
    assert!(
        properties
            .lines()
            .any(|line| line == "KEYBOARD_KEY_70065=home")
    );

    assert!(matches!(
        isolated.invoke("reset", &Request::Reset),
        DecodedResponse::Reset { .. }
    ));
    let properties = isolated.query_remote();
    assert!(
        !properties
            .lines()
            .any(|line| line.starts_with("KEYBOARD_KEY_70065="))
    );
    assert!(
        properties
            .lines()
            .any(|line| line == "KEYBOARD_KEY_7003e=reserved")
    );
    assert!(
        properties
            .lines()
            .any(|line| line == "KEYBOARD_KEY_70066=reserved")
    );
}
