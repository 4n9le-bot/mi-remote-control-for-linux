use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io,
    path::Path,
    process,
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_lite::{StreamExt, future};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use crate::{
    AttachmentMonitor, AtvvChange, AtvvEvent, AtvvGatt, AtvvTransport, BluezClient, BluezSnapshot,
    Clock, Command, CommandOutput, Device, GattCharacteristic, GattService, OperationalEvent,
    OperationalEvents, ProcessExecutor, Storage,
};

static PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Default)]
pub struct SystemBoundaries {
    attachment: AttachmentMonitor,
    bluez: Option<zbus::blocking::Connection>,
    connection_watch: Option<ConnectionWatch>,
}

#[derive(Debug)]
struct ConnectionWatch {
    changes: mpsc::Receiver<()>,
    cancel: async_channel::Sender<()>,
}

impl Drop for ConnectionWatch {
    fn drop(&mut self) {
        let _ = self.cancel.try_send(());
    }
}

impl AtvvTransport for SystemBoundaries {
    fn next_event(&mut self) -> io::Result<AtvvEvent> {
        let mut attachment = std::mem::take(&mut self.attachment);
        let result = attachment.next_event(self).map_err(io::Error::other);
        self.attachment = attachment;
        result
    }
}

impl AtvvGatt for SystemBoundaries {
    fn snapshot(&mut self) -> io::Result<BluezSnapshot> {
        self.managed_objects()
    }

    fn watch_connection(&mut self, device_path: &str) -> io::Result<()> {
        self.connection_watch = None;
        let connection = self.bluez_connection()?.inner().clone();
        let device_path = device_path.to_owned();
        let (changes_tx, changes_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::sync_channel(0);
        let (cancel_tx, cancel_rx) = async_channel::bounded(1);
        thread::spawn(move || {
            async_io::block_on(async move {
                let proxy = match async_device_proxy(&connection, &device_path).await {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                let mut connected = proxy.receive_property_changed::<bool>("Connected").await;
                let mut services_resolved = proxy
                    .receive_property_changed::<bool>("ServicesResolved")
                    .await;
                if ready_tx.send(Ok(())).is_err() {
                    return;
                }
                loop {
                    let connection_changed = future::race(
                        async {
                            Some(
                                future::race(connected.next(), services_resolved.next())
                                    .await
                                    .is_some(),
                            )
                        },
                        async {
                            let _ = cancel_rx.recv().await;
                            None
                        },
                    )
                    .await;
                    match connection_changed {
                        None => return,
                        Some(stream_open) => {
                            if changes_tx.send(()).is_err() || !stream_open {
                                return;
                            }
                        }
                    }
                }
            });
        });
        ready_rx
            .recv()
            .map_err(|_| io::Error::other("ATVV connection monitor stopped during setup"))??;
        self.connection_watch = Some(ConnectionWatch {
            changes: changes_rx,
            cancel: cancel_tx,
        });
        Ok(())
    }

    fn subscribe(&mut self, characteristic_path: &str) -> io::Result<()> {
        let connection = self.bluez_connection()?;
        let proxy = characteristic_proxy(&connection, characteristic_path)?;
        proxy
            .call::<_, _, ()>("StartNotify", &())
            .map_err(io::Error::other)
    }

    fn get_capabilities(&mut self, tx_path: &str, control_path: &str) -> io::Result<Vec<u8>> {
        let connection = self.bluez_connection()?.inner().clone();
        let tx_path = tx_path.to_owned();
        let control_path = control_path.to_owned();
        async_io::block_on(async move {
            let tx = async_characteristic_proxy(&connection, &tx_path).await?;
            let control = async_characteristic_proxy(&connection, &control_path).await?;
            let mut value_changes = control.receive_property_changed::<Vec<u8>>("Value").await;
            let options: HashMap<&str, zbus::zvariant::Value<'_>> = HashMap::new();
            tx.call::<_, _, ()>("WriteValue", &(vec![0x0A_u8], options))
                .await
                .map_err(io::Error::other)?;

            future::race(
                async {
                    while let Some(change) = value_changes.next().await {
                        let value = change.get().await.map_err(io::Error::other)?;
                        if value.first() == Some(&0x0B) {
                            return Ok(value);
                        }
                    }
                    Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "ATVV control notifications ended",
                    ))
                },
                async {
                    async_io::Timer::after(Duration::from_secs(5)).await;
                    Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "ATVV capability response timed out",
                    ))
                },
            )
            .await
        })
    }

    fn wait_for_change(&mut self) -> io::Result<AtvvChange> {
        if let Some(watch) = self.connection_watch.as_ref() {
            watch.changes.recv().map_err(|_| {
                io::Error::new(io::ErrorKind::BrokenPipe, "ATVV connection monitor stopped")
            })?;
            return Ok(AtvvChange::ConnectionChanged);
        }
        thread::sleep(Duration::from_millis(250));
        Ok(AtvvChange::TopologyChanged)
    }
}

impl SystemBoundaries {
    fn bluez_connection(&mut self) -> io::Result<zbus::blocking::Connection> {
        if self.bluez.is_none() {
            self.bluez = Some(zbus::blocking::Connection::system().map_err(io::Error::other)?);
        }
        Ok(self.bluez.as_ref().expect("connection initialized").clone())
    }
}

fn characteristic_proxy<'a>(
    connection: &zbus::blocking::Connection,
    path: &'a str,
) -> io::Result<zbus::blocking::Proxy<'a>> {
    zbus::blocking::Proxy::new(
        connection,
        "org.bluez",
        path,
        "org.bluez.GattCharacteristic1",
    )
    .map_err(io::Error::other)
}

async fn async_characteristic_proxy<'a>(
    connection: &zbus::Connection,
    path: &'a str,
) -> io::Result<zbus::Proxy<'a>> {
    zbus::Proxy::new(
        connection,
        "org.bluez",
        path,
        "org.bluez.GattCharacteristic1",
    )
    .await
    .map_err(io::Error::other)
}

async fn async_device_proxy<'a>(
    connection: &zbus::Connection,
    path: &'a str,
) -> io::Result<zbus::Proxy<'a>> {
    zbus::Proxy::new(connection, "org.bluez", path, "org.bluez.Device1")
        .await
        .map_err(io::Error::other)
}

impl BluezClient for SystemBoundaries {
    fn managed_objects(&mut self) -> io::Result<BluezSnapshot> {
        let connection = self.bluez_connection()?;
        let proxy = zbus::blocking::fdo::ObjectManagerProxy::builder(&connection)
            .destination("org.bluez")
            .map_err(io::Error::other)?
            .path("/")
            .map_err(io::Error::other)?
            .build()
            .map_err(io::Error::other)?;
        let objects = proxy.get_managed_objects().map_err(io::Error::other)?;
        let mut snapshot = BluezSnapshot::default();

        for (path, interfaces) in objects {
            let path = path.to_string();
            for (interface, properties) in interfaces {
                match interface.as_str() {
                    "org.bluez.Device1" => {
                        let (Some(address), Some(connected), Some(services_resolved)) = (
                            string_property(&properties, "Address"),
                            bool_property(&properties, "Connected"),
                            bool_property(&properties, "ServicesResolved"),
                        ) else {
                            continue;
                        };
                        snapshot.devices.push(Device {
                            path: path.clone(),
                            address,
                            connected,
                            services_resolved,
                        });
                    }
                    "org.bluez.GattService1" => {
                        let (Some(uuid), Some(device_path)) = (
                            string_property(&properties, "UUID"),
                            path_property(&properties, "Device"),
                        ) else {
                            continue;
                        };
                        snapshot.services.push(GattService {
                            path: path.clone(),
                            device_path,
                            uuid,
                        });
                    }
                    "org.bluez.GattCharacteristic1" => {
                        let (Some(uuid), Some(service_path)) = (
                            string_property(&properties, "UUID"),
                            path_property(&properties, "Service"),
                        ) else {
                            continue;
                        };
                        snapshot.characteristics.push(GattCharacteristic {
                            path: path.clone(),
                            service_path,
                            uuid,
                        });
                    }
                    _ => {}
                }
            }
        }
        Ok(snapshot)
    }
}

fn string_property(
    properties: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    name: &str,
) -> Option<String> {
    <&str>::try_from(properties.get(name)?)
        .ok()
        .map(str::to_owned)
}

fn bool_property(
    properties: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    name: &str,
) -> Option<bool> {
    bool::try_from(properties.get(name)?).ok()
}

fn path_property(
    properties: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    name: &str,
) -> Option<String> {
    <&zbus::zvariant::ObjectPath<'_>>::try_from(properties.get(name)?)
        .ok()
        .map(ToString::to_string)
}

impl ProcessExecutor for SystemBoundaries {
    fn execute(&mut self, command: &Command) -> io::Result<CommandOutput> {
        let output = process::Command::new(&command.program)
            .args(&command.args)
            .output()?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

impl Storage for SystemBoundaries {
    fn read_optional_config(&mut self, path: &Path) -> io::Result<Option<String>> {
        match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn prepare_wav_dir(&mut self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)?;
        if !fs::metadata(path)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path is not a directory",
            ));
        }

        let probe = path.join(format!(
            ".atvv-bridge-write-probe-{}-{}",
            process::id(),
            PROBE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(&probe)?;
        drop(file);
        fs::remove_file(probe)
    }
}

impl Clock for SystemBoundaries {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

impl OperationalEvents for SystemBoundaries {
    fn emit(&mut self, event: OperationalEvent) {
        match event {
            OperationalEvent::DaemonStarted {
                at,
                max_duration_secs,
                wav_dir,
                keep_wav,
            } => eprintln!(
                "event=daemon_started at_unix_ms={} max_duration_secs={} wav_dir={:?} keep_wav={}",
                unix_millis(at),
                max_duration_secs,
                wav_dir,
                keep_wav
            ),
            OperationalEvent::WaitingForRemote { at } => {
                eprintln!("event=waiting_for_remote at_unix_ms={}", unix_millis(at))
            }
            OperationalEvent::RemoteReady { at, address } => eprintln!(
                "event=remote_ready at_unix_ms={} address={:?}",
                unix_millis(at),
                address
            ),
            OperationalEvent::DaemonStopped { at } => {
                eprintln!("event=daemon_stopped at_unix_ms={}", unix_millis(at))
            }
        }
    }
}

fn unix_millis(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
