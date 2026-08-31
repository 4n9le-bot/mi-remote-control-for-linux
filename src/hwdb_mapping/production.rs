use std::{
    any::Any,
    collections::BTreeMap,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use super::{HwdbSystem, SystemError};

const PACKAGE_SOURCE: &str = "usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb";
const MANAGED_SOURCE: &str = "etc/udev/hwdb.d/99-atvv-bridge-button-mapping.hwdb";
const MANAGED_DIRECTORY: &str = "etc/udev/hwdb.d";
const LIVE_DATABASE: &str = "etc/udev/hwdb.bin";
const REMOTE_MODALIAS: &str = "evdev:input:b0005v2717p32B8*";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_COMMAND_OUTPUT_BYTES: u64 = 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 64 * 1024;
const ISOLATED_DIRECTORIES: [&str; 7] = [
    "usr",
    "usr/lib",
    "usr/lib/udev",
    "usr/lib/udev/hwdb.d",
    "etc",
    "etc/udev",
    MANAGED_DIRECTORY,
];
static STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct ProductionHwdbSystem {
    root: PathBuf,
    command: PathBuf,
    command_owner: u32,
    expected_owner: u32,
}

struct StagedSource {
    path: PathBuf,
}

struct SystemLock(File);

impl Drop for SystemLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

impl Drop for StagedSource {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl ProductionHwdbSystem {
    pub(super) fn host() -> Self {
        Self {
            root: PathBuf::from("/"),
            command: PathBuf::from("/usr/bin/systemd-hwdb"),
            command_owner: 0,
            expected_owner: 0,
        }
    }

    #[cfg(test)]
    fn isolated(root: PathBuf) -> Self {
        let command = PathBuf::from("/usr/bin/systemd-hwdb");
        let command_owner = fs::symlink_metadata(&command)
            .expect("systemd-hwdb is required for the isolated integration test")
            .uid();
        Self {
            root,
            command,
            command_owner,
            // Unit-test roots are deliberately owned by the invoking user.
            expected_owner: unsafe { libc::geteuid() },
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn validate_directory(&self, path: &Path) -> Result<(), SystemError> {
        let metadata = fs::symlink_metadata(path).map_err(|_| SystemError::OperationFailed)?;
        let mode = metadata.mode() & 0o7777;
        if !metadata.file_type().is_dir()
            || metadata.uid() != self.expected_owner
            || mode & 0o022 != 0
        {
            return Err(SystemError::OperationFailed);
        }
        Ok(())
    }

    fn validate_directory_tree(&self, relative: &str) -> Result<(), SystemError> {
        let mut current = self.root.clone();
        self.validate_directory(&current)?;
        for component in Path::new(relative).components() {
            current.push(component);
            self.validate_directory(&current)?;
        }
        Ok(())
    }

    fn validate_regular(&self, metadata: &fs::Metadata, writable: bool) -> Result<(), SystemError> {
        let mode = metadata.mode() & 0o7777;
        if !metadata.file_type().is_file()
            || metadata.uid() != self.expected_owner
            || metadata.nlink() != 1
            || mode & 0o022 != 0
            || (writable && mode != 0o644)
        {
            return Err(SystemError::OperationFailed);
        }
        Ok(())
    }

    fn open_secure(&self, path: &Path, max_bytes: u64) -> Result<File, SystemError> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| SystemError::OperationFailed)?;
        let metadata = file.metadata().map_err(|_| SystemError::OperationFailed)?;
        self.validate_regular(&metadata, true)?;
        if metadata.len() > max_bytes {
            return Err(SystemError::OperationFailed);
        }
        Ok(file)
    }

    fn read_package_source(&self) -> Result<Vec<u8>, SystemError> {
        self.validate_directory_tree("usr/lib/udev/hwdb.d")?;
        let path = self.path(PACKAGE_SOURCE);
        let mut file = self.open_secure(&path, MAX_SOURCE_BYTES)?;
        let mut source = Vec::new();
        file.read_to_end(&mut source)
            .map_err(|_| SystemError::OperationFailed)?;
        Ok(source)
    }

    fn write_stage(&self, source: &[u8]) -> Result<PathBuf, SystemError> {
        let directory = self.path(MANAGED_DIRECTORY);
        self.validate_directory_tree(MANAGED_DIRECTORY)?;
        for _ in 0..128 {
            let sequence = STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = directory.join(format!(".99-atvv-bridge-button-mapping.{sequence}.tmp"));
            let opened = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&path);
            let mut file = match opened {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(SystemError::OperationFailed),
            };
            let result = (|| {
                file.write_all(source)
                    .map_err(|_| SystemError::OperationFailed)?;
                file.set_permissions(fs::Permissions::from_mode(0o644))
                    .map_err(|_| SystemError::OperationFailed)?;
                self.validate_regular(
                    &file.metadata().map_err(|_| SystemError::OperationFailed)?,
                    true,
                )?;
                file.sync_all().map_err(|_| SystemError::OperationFailed)
            })();
            if result.is_err() {
                let _ = fs::remove_file(&path);
            }
            result?;
            return Ok(path);
        }
        Err(SystemError::OperationFailed)
    }

    fn read_stage(&self, path: &Path) -> Result<Vec<u8>, SystemError> {
        let mut file = self.open_secure(path, MAX_SOURCE_BYTES)?;
        let mut source = Vec::new();
        file.read_to_end(&mut source)
            .map_err(|_| SystemError::OperationFailed)?;
        Ok(source)
    }

    fn sync_managed_directory(&self) -> Result<(), SystemError> {
        File::open(self.path(MANAGED_DIRECTORY))
            .and_then(|directory| directory.sync_all())
            .map_err(|_| SystemError::OperationFailed)
    }

    fn run(&self, root: &Path, arguments: &[&OsStr]) -> Result<Vec<u8>, SystemError> {
        let command_metadata =
            fs::symlink_metadata(&self.command).map_err(|_| SystemError::Unsupported)?;
        if !command_metadata.file_type().is_file()
            || command_metadata.uid() != self.command_owner
            || command_metadata.mode() & 0o022 != 0
        {
            return Err(SystemError::Unsupported);
        }

        let mut output = tempfile::tempfile().map_err(|_| SystemError::OperationFailed)?;
        let stdout = output
            .try_clone()
            .map_err(|_| SystemError::OperationFailed)?;
        let mut command = Command::new(&self.command);
        command
            .arg(format!("--root={}", root.display()))
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|_| SystemError::Unsupported)?;
        let started = Instant::now();
        let status = loop {
            if output
                .metadata()
                .map_err(|_| SystemError::OperationFailed)?
                .len()
                > MAX_COMMAND_OUTPUT_BYTES
            {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SystemError::OperationFailed);
            }
            match child.try_wait().map_err(|_| SystemError::OperationFailed)? {
                Some(status) => break status,
                None if started.elapsed() < COMMAND_TIMEOUT => {
                    thread::sleep(Duration::from_millis(10));
                }
                None => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(SystemError::OperationFailed);
                }
            }
        };
        if !status.success() {
            return Err(SystemError::OperationFailed);
        }
        if output
            .metadata()
            .map_err(|_| SystemError::OperationFailed)?
            .len()
            > MAX_COMMAND_OUTPUT_BYTES
        {
            return Err(SystemError::OperationFailed);
        }
        output
            .seek(SeekFrom::Start(0))
            .map_err(|_| SystemError::OperationFailed)?;
        let mut bytes = Vec::new();
        output
            .read_to_end(&mut bytes)
            .map_err(|_| SystemError::OperationFailed)?;
        Ok(bytes)
    }

    fn update(&self, root: &Path) -> Result<(), SystemError> {
        let database = root.join(LIVE_DATABASE);
        let database_directory = database.parent().ok_or(SystemError::OperationFailed)?;
        self.validate_directory(database_directory)?;
        self.validate_existing_database(&database)?;
        self.run(root, &[OsStr::new("--strict"), OsStr::new("update")])?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&database)
            .map_err(|_| SystemError::OperationFailed)?;
        self.validate_regular(
            &file.metadata().map_err(|_| SystemError::OperationFailed)?,
            false,
        )?;
        file.sync_all().map_err(|_| SystemError::OperationFailed)?;
        self.validate_directory(database_directory)?;
        File::open(database_directory)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| SystemError::OperationFailed)
    }

    fn validate_existing_database(&self, database: &Path) -> Result<(), SystemError> {
        match fs::symlink_metadata(database) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(SystemError::OperationFailed),
            Ok(_) => {
                let file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(database)
                    .map_err(|_| SystemError::OperationFailed)?;
                self.validate_regular(
                    &file.metadata().map_err(|_| SystemError::OperationFailed)?,
                    false,
                )
            }
        }
    }

    fn query(&self, root: &Path) -> Result<BTreeMap<String, String>, SystemError> {
        let output = self.run(root, &[OsStr::new("query"), OsStr::new(REMOTE_MODALIAS)])?;
        let output = std::str::from_utf8(&output).map_err(|_| SystemError::OperationFailed)?;
        let mut properties = BTreeMap::new();
        for line in output.lines() {
            let Some((name, value)) = line.split_once('=') else {
                return Err(SystemError::OperationFailed);
            };
            properties.insert(name.to_owned(), value.to_owned());
        }
        Ok(properties)
    }

    fn create_validation_root(&self) -> Result<tempfile::TempDir, SystemError> {
        let root = tempfile::tempdir().map_err(|_| SystemError::OperationFailed)?;
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755))
            .map_err(|_| SystemError::OperationFailed)?;
        create_isolated_layout(root.path())?;
        let package_directory = root.path().join("usr/lib/udev/hwdb.d");
        write_validation_file(
            &package_directory.join("90-atvv-bridge.hwdb"),
            &self.read_package_source()?,
        )?;
        Ok(root)
    }

    fn compile_root_and_query(&self, root: &Path) -> Result<BTreeMap<String, String>, SystemError> {
        self.update(root)?;
        self.query(root)
    }
}

impl HwdbSystem for ProductionHwdbSystem {
    fn try_lock(&mut self) -> Result<Box<dyn Any>, SystemError> {
        self.validate_directory_tree(MANAGED_DIRECTORY)?;
        let file =
            File::open(self.path(MANAGED_DIRECTORY)).map_err(|_| SystemError::OperationFailed)?;
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            if matches!(error.raw_os_error(), Some(libc::EAGAIN)) {
                return Err(SystemError::Busy);
            }
            return Err(SystemError::OperationFailed);
        }
        Ok(Box::new(SystemLock(file)))
    }

    fn is_supported(&mut self) -> Result<bool, SystemError> {
        self.read_package_source()?;
        let help = match self.run(&self.root, &[OsStr::new("--help")]) {
            Ok(help) => help,
            Err(SystemError::Unsupported) => return Ok(false),
            Err(error) => return Err(error),
        };
        let help = String::from_utf8_lossy(&help);
        Ok(["--root", "--strict", "update", "query"]
            .into_iter()
            .all(|capability| help.contains(capability)))
    }

    fn read_managed(&mut self) -> Result<Option<Vec<u8>>, SystemError> {
        self.validate_directory_tree(MANAGED_DIRECTORY)?;
        let path = self.path(MANAGED_SOURCE);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(SystemError::OperationFailed),
            Ok(_) => {
                let mut file = self.open_secure(&path, MAX_SOURCE_BYTES)?;
                let mut source = Vec::new();
                file.read_to_end(&mut source)
                    .map_err(|_| SystemError::OperationFailed)?;
                Ok(Some(source))
            }
        }
    }

    fn stage_candidate(&mut self, source: &[u8]) -> Result<Box<dyn Any>, SystemError> {
        if source.len() as u64 > MAX_SOURCE_BYTES {
            return Err(SystemError::OperationFailed);
        }
        self.write_stage(source)
            .map(|path| Box::new(StagedSource { path }) as Box<dyn Any>)
    }

    fn compile_staged_and_query(
        &mut self,
        staged: &dyn Any,
    ) -> Result<BTreeMap<String, String>, SystemError> {
        let staged = staged
            .downcast_ref::<StagedSource>()
            .ok_or(SystemError::OperationFailed)?;
        let source = self.read_stage(&staged.path)?;
        let root = self.create_validation_root()?;
        fs::copy(&staged.path, root.path().join(MANAGED_SOURCE))
            .map_err(|_| SystemError::OperationFailed)?;
        let copied = self.read_stage(&root.path().join(MANAGED_SOURCE))?;
        if copied != source {
            return Err(SystemError::OperationFailed);
        }
        self.compile_root_and_query(root.path())
    }

    fn compile_defaults_and_query(&mut self) -> Result<BTreeMap<String, String>, SystemError> {
        let root = self.create_validation_root()?;
        self.compile_root_and_query(root.path())
    }

    fn commit_staged(&mut self, staged: Box<dyn Any>) -> Result<(), SystemError> {
        let mut staged = staged
            .downcast::<StagedSource>()
            .map_err(|_| SystemError::OperationFailed)?;
        let target = self.path(MANAGED_SOURCE);
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                let _ = self.open_secure(&target, MAX_SOURCE_BYTES)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(SystemError::OperationFailed),
        }
        fs::rename(&staged.path, &target).map_err(|_| SystemError::OperationFailed)?;
        staged.path = PathBuf::new();
        self.sync_managed_directory()
            .map_err(|_| SystemError::CommittedButNotDurable)
    }

    fn replace_managed(&mut self, source: &[u8]) -> Result<(), SystemError> {
        if source.len() as u64 > MAX_SOURCE_BYTES {
            return Err(SystemError::OperationFailed);
        }
        let target = self.path(MANAGED_SOURCE);
        match fs::symlink_metadata(&target) {
            Ok(_) => {
                let _ = self.open_secure(&target, MAX_SOURCE_BYTES)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(SystemError::OperationFailed),
        }
        let stage = self.write_stage(source)?;
        if fs::rename(&stage, &target).is_err() {
            let _ = fs::remove_file(stage);
            return Err(SystemError::OperationFailed);
        }
        self.sync_managed_directory()
            .map_err(|_| SystemError::CommittedButNotDurable)
    }

    fn remove_managed(&mut self) -> Result<(), SystemError> {
        self.validate_directory_tree(MANAGED_DIRECTORY)?;
        let target = self.path(MANAGED_SOURCE);
        let removed = match fs::symlink_metadata(&target) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(SystemError::OperationFailed),
            Ok(_) => {
                let _ = self.open_secure(&target, MAX_SOURCE_BYTES)?;
                fs::remove_file(target).map_err(|_| SystemError::OperationFailed)?;
                true
            }
        };
        self.sync_managed_directory().map_err(|_| {
            if removed {
                SystemError::CommittedButNotDurable
            } else {
                SystemError::OperationFailed
            }
        })
    }

    fn update_live(&mut self) -> Result<(), SystemError> {
        self.update(&self.root)
    }

    fn query_live(&mut self) -> Result<BTreeMap<String, String>, SystemError> {
        self.query(&self.root)
    }
}

fn write_validation_file(path: &Path, contents: &[u8]) -> Result<(), SystemError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(path)
        .map_err(|_| SystemError::OperationFailed)?;
    file.write_all(contents)
        .map_err(|_| SystemError::OperationFailed)?;
    file.set_permissions(fs::Permissions::from_mode(0o644))
        .map_err(|_| SystemError::OperationFailed)
}

fn create_isolated_layout(root: &Path) -> Result<(), SystemError> {
    fs::create_dir_all(root.join("usr/lib/udev/hwdb.d"))
        .map_err(|_| SystemError::OperationFailed)?;
    fs::create_dir_all(root.join(MANAGED_DIRECTORY)).map_err(|_| SystemError::OperationFailed)?;
    for relative in ISOLATED_DIRECTORIES {
        fs::set_permissions(root.join(relative), fs::Permissions::from_mode(0o755))
            .map_err(|_| SystemError::OperationFailed)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        button_mapping::{ButtonId, Mapping, MappingTarget, logical_key_by_symbol},
        hwdb_mapping::{properties_match, render},
    };
    use std::os::unix::fs::symlink;

    fn system_root() -> (tempfile::TempDir, ProductionHwdbSystem) {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
        create_isolated_layout(root.path()).unwrap();
        write_validation_file(
            &root.path().join(PACKAGE_SOURCE),
            include_bytes!("../../packaging/90-atvv-bridge.hwdb"),
        )
        .unwrap();
        let system = ProductionHwdbSystem::isolated(root.path().to_owned());
        (root, system)
    }

    #[test]
    fn real_systemd_hwdb_validates_generated_properties_in_an_isolated_root() {
        let (_root, mut system) = system_root();
        let mut entries: Vec<_> = Mapping::defaults().iter().collect();
        entries[1].1 = MappingTarget::Key(logical_key_by_symbol("KEY_SPACE").unwrap());
        entries[2].1 = MappingTarget::Disabled;
        let mapping = Mapping::from_entries(entries).unwrap();
        let source = render(&mapping);

        let staged = system.stage_candidate(&source).unwrap();
        let properties = system
            .compile_staged_and_query(staged.as_ref())
            .expect("the host systemd-hwdb should compile and query the canonical source");
        assert!(properties_match(&mapping, &properties));
        assert_eq!(
            properties.get(&format!("KEYBOARD_KEY_{}", ButtonId::Confirm.scan_code())),
            Some(&"space".to_owned())
        );
    }

    #[test]
    fn privileged_source_operations_reject_symlinks_and_unsafe_modes() {
        let (root, mut system) = system_root();
        let managed = root.path().join(MANAGED_SOURCE);
        symlink("/dev/null", &managed).unwrap();
        assert!(system.read_managed().is_err());
        fs::remove_file(&managed).unwrap();

        write_validation_file(&managed, &render(&Mapping::defaults())).unwrap();
        fs::set_permissions(&managed, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(system.read_managed().is_err());
        fs::remove_file(&managed).unwrap();

        fs::create_dir(&managed).unwrap();
        assert!(system.read_managed().is_err());
    }

    #[test]
    fn every_privileged_path_category_rejects_unsafe_metadata() {
        let (root, mut system) = system_root();

        let package = root.path().join(PACKAGE_SOURCE);
        fs::remove_file(&package).unwrap();
        symlink("/dev/null", &package).unwrap();
        assert!(system.read_package_source().is_err());

        fs::set_permissions(
            root.path().join("etc/udev"),
            fs::Permissions::from_mode(0o777),
        )
        .unwrap();
        assert!(system.read_managed().is_err());

        let command_metadata = fs::symlink_metadata(&system.command).unwrap();
        system.command_owner = command_metadata.uid().wrapping_add(1);
        assert!(matches!(
            system.run(&system.root, &[OsStr::new("--help")]),
            Err(SystemError::Unsupported)
        ));

        system.expected_owner = unsafe { libc::geteuid() }.wrapping_add(1);
        assert!(system.validate_regular(&command_metadata, false).is_err());
    }

    #[test]
    fn live_database_is_rejected_before_update_when_metadata_is_unsafe() {
        let (root, mut system) = system_root();
        let database = root.path().join(LIVE_DATABASE);

        symlink("/dev/null", &database).unwrap();
        assert!(system.validate_existing_database(&database).is_err());
        fs::remove_file(&database).unwrap();

        fs::create_dir(&database).unwrap();
        assert!(system.validate_existing_database(&database).is_err());
        fs::remove_dir(&database).unwrap();

        write_validation_file(&database, b"unsafe").unwrap();
        fs::set_permissions(&database, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(system.validate_existing_database(&database).is_err());
        fs::set_permissions(&database, fs::Permissions::from_mode(0o444)).unwrap();

        system.expected_owner = unsafe { libc::geteuid() }.wrapping_add(1);
        assert!(system.validate_existing_database(&database).is_err());
    }

    #[test]
    fn command_output_is_bounded() {
        let (root, mut system) = system_root();
        let command = root.path().join("large-output-command");
        write_validation_file(&command, b"#!/bin/sh\nhead -c 2097152 /dev/zero\n").unwrap();
        fs::set_permissions(&command, fs::Permissions::from_mode(0o755)).unwrap();
        system.command = command;
        system.command_owner = unsafe { libc::geteuid() };

        assert!(matches!(
            system.run(root.path(), &[OsStr::new("query")]),
            Err(SystemError::OperationFailed)
        ));
    }

    #[test]
    fn every_operation_uses_the_same_nonblocking_system_lock() {
        let (root, mut first) = system_root();
        let mut second = ProductionHwdbSystem::isolated(root.path().to_owned());

        let held = first.try_lock().unwrap();
        assert!(matches!(second.try_lock(), Err(SystemError::Busy)));
        drop(held);
        assert!(
            second.try_lock().is_ok(),
            "the lock must be released when its guard is dropped"
        );
    }
}
