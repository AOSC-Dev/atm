//! Packagekit related functions

mod packagekit;
mod packagekit_tx;

use std::{collections::HashMap, future::Future, sync::mpsc::Sender, time::Duration};

use anyhow::{anyhow, Result};
pub use packagekit::PackageKitProxy;
use packagekit_tx::TransactionProxy;
use serde::Deserialize;
use zbus::{
    dbus_proxy,
    export::futures_util::StreamExt,
    zvariant::{Signature, Type},
    Connection, Result as zResult,
};

const PACKAGEKIT_DEST: &str = "org.freedesktop.PackageKit";

#[derive(Deserialize, Debug)]
pub struct PkPackage {
    pub info: u32,
    pub package_id: String,
    pub summary: String,
}

impl Type for PkPackage {
    fn signature() -> zbus::zvariant::Signature<'static> {
        Signature::from_static_str("(uss)").unwrap()
    }
}

#[derive(Deserialize)]
pub struct PkProgress {
    pub id: String,
    pub status: u32,
    pub percentage: u32,
}

impl Type for PkProgress {
    fn signature() -> zbus::zvariant::Signature<'static> {
        Signature::from_static_str("(suu)").unwrap()
    }
}

#[derive(Deserialize)]
pub struct PkError {
    pub code: u32,
    pub details: String,
}

impl Type for PkError {
    fn signature() -> zbus::zvariant::Signature<'static> {
        Signature::from_static_str("(us)").unwrap()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PkPackgeId<'a> {
    pub name: &'a str,
    pub version: &'a str,
    arch: &'a str,
    data: &'a str,
}

#[derive(Clone, Debug)]
pub struct PkTaskList<'a> {
    pub hold: Vec<PkPackgeId<'a>>,
    pub upgrade: Vec<PkPackgeId<'a>>,
    pub install: Vec<PkPackgeId<'a>>,
    pub downgrade: Vec<PkPackgeId<'a>>,
    pub erase: Vec<PkPackgeId<'a>>,
}

#[dbus_proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    /// OnBattery property
    #[dbus_proxy(property)]
    fn on_battery(&self) -> zResult<bool>;
}

#[dbus_proxy(
    interface = "org.freedesktop.login1",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1 {
    /// Inhibit method
    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zResult<zbus::zvariant::OwnedFd>;
}

// PackageKit enumeration constants (could be OR'ed)
const PK_FILTER_ENUM_NEWEST: u32 = 1 << 16;
const PK_FILTER_ENUM_ARCH: u32 = 1 << 18;
const PK_FILTER_ENUM_NOT_SOURCE: u32 = 1 << 21;
const PK_TRANSACTION_FLAG_ENUM_SIMULATE: u32 = 1 << 2;
const PK_TRANSACTION_FLAG_ENUM_ALLOW_REINSTALL: u32 = 1 << 4;
const PK_TRANSACTION_FLAG_ENUM_ALLOW_DOWNGRADE: u32 = 1 << 6;
// PackageKit informational constants (literal values)
const PK_NETWORK_ENUM_MOBILE: u8 = 5;
// PackageKit status constants
// pub const PK_STATUS_ENUM_WAIT: u8 = 1;
pub const PK_STATUS_ENUM_SETUP: u8 = 2;
pub const PK_STATUS_ENUM_DOWNLOAD: u8 = 8;
pub const PK_STATUS_ENUM_INSTALL: u8 = 9;
const PK_INFO_ENUM_INSTALLED: u8 = 1;
// const PK_INFO_ENUM_AVAILABLE: u8 = 2;
const PK_INFO_ENUM_UPDATING: u8 = 11;
const PK_INFO_ENUM_INSTALLING: u8 = 12;
const PK_INFO_ENUM_REMOVING: u8 = 13;
// const PK_INFO_ENUM_OBSOLETING: u8 = 15;
const PK_INFO_ENUM_REINSTALLING: u8 = 19;
const PK_INFO_ENUM_DOWNGRADING: u8 = 20;

#[derive(Debug)]
pub enum PkDisplayProgress {
    /// Individual package progress (package_id, PK_STATUS, progress %)
    Package(String, u8, u32),
    /// Overall transaction progress (progress %)
    Overall(u32),
    /// Sentinel for transaction
    Done,
}

#[inline]
pub fn humanize_package_id(package_id: &str) -> String {
    let result = parse_package_id(package_id);
    if let Some(result) = result {
        format!("{} ({}) [{}]", result.name, result.version, result.arch)
    } else {
        "? (?)".to_string()
    }
}

fn parse_package_id<'a>(package_id: &'a str) -> Option<PkPackgeId<'a>> {
    let mut splitted = package_id.splitn(4, ';');

    Some(PkPackgeId {
        name: splitted.next()?,
        version: splitted.next()?,
        arch: splitted.next()?,
        data: splitted.next()?,
    })
}

async fn wait_for_exit_signal<Fut: Future<Output = zResult<()>>>(
    proxy: &TransactionProxy<'_>,
    func: Fut,
) -> Result<()> {
    let mut signal_stream = proxy.receive_all_signals().await?;
    // poll the future to start the transaction
    func.await?;
    while let Some(signal) = signal_stream.next().await {
        let name = signal.member();
        if let Some(name) = name {
            match name.as_str() {
                "ErrorCode" => {
                    let e: PkError = signal.body()?;
                    return Err(anyhow!("({}) {}", e.code, e.details));
                }
                "Finished" | "Destroy" => break,
                _ => continue,
            }
        }
    }

    Ok(())
}

async fn collect_packages<Fut: Future<Output = zResult<()>>>(
    proxy: &TransactionProxy<'_>,
    func: Fut,
) -> Result<Vec<PkPackage>> {
    let mut packages: Vec<PkPackage> = Vec::new();
    let mut signal_stream = proxy.receive_all_signals().await?;
    // poll the future to start the transaction
    func.await?;
    while let Some(signal) = signal_stream.next().await {
        let name = signal.member();
        if let Some(name) = name {
            match name.as_str() {
                "Package" => packages.push(signal.body()?),
                "ErrorCode" => {
                    let e: PkError = signal.body()?;
                    return Err(anyhow!("({}) {}", e.code, e.details));
                }
                "Finished" | "Destroy" => break,
                _ => continue,
            }
        }
    }

    Ok(packages)
}

/// Connect to the D-Bus system bus
pub async fn create_dbus_connection() -> zResult<Connection> {
    Connection::system().await
}

/// Connect to the packagekit backend
pub async fn connect_packagekit(conn: &Connection) -> zResult<PackageKitProxy> {
    PackageKitProxy::new(conn).await
}

/// A convient function to create a new PackageKit transaction session
pub async fn create_transaction<'a>(
    proxy: &'a PackageKitProxy<'a>,
) -> zResult<TransactionProxy<'a>> {
    let path = proxy.create_transaction().await?;

    TransactionProxy::builder(proxy.connection())
        .path(path)?
        .destination(PACKAGEKIT_DEST)?
        .build()
        .await
}

/// Refresh repository cache (forcibly refreshes the caches)
pub async fn refresh_cache(proxy: &TransactionProxy<'_>) -> Result<()> {
    wait_for_exit_signal(proxy, async move { proxy.refresh_cache(true).await }).await
}

/// Fetch all the updatable packages (requires transaction proxy)
pub async fn get_updated_packages(proxy: &TransactionProxy<'_>) -> Result<Vec<PkPackage>> {
    collect_packages(proxy, async move {
        proxy.get_updates(PK_FILTER_ENUM_NEWEST as u64).await
    })
    .await
}

/// Find the package ID of the stable version of the given packages, returns (not found, found) (requires transaction proxy)
pub async fn find_stable_version_of(
    proxy: &TransactionProxy<'_>,
    packages: &[&str],
) -> Result<(Vec<String>, Vec<String>)> {
    if packages.is_empty() {
        return Ok((vec![], vec![]));
    }

    let candidates = collect_packages(proxy, async move {
        proxy
            .resolve(
                (PK_FILTER_ENUM_ARCH | PK_FILTER_ENUM_NOT_SOURCE) as u64,
                packages,
            )
            .await
    })
    .await?;

    let mut candidates_map: HashMap<String, PkPackage> = HashMap::new();
    candidates_map.reserve(candidates.len());

    for candidate in candidates {
        let candidate_parsed =
            parse_package_id(&candidate.package_id).ok_or_else(|| anyhow!("Invalid package id"))?;
        // skip packages that are not in the stable branch
        if !candidate_parsed.data.starts_with("aosc-stable-")
            && !candidate_parsed.data.starts_with("installed:aosc-stable-")
        {
            continue;
        }

        if candidates_map.contains_key(candidate_parsed.name) {
            continue;
        }
        candidates_map.insert(candidate_parsed.name.to_string(), candidate);
    }

    let mut result = Vec::new();
    let mut not_found = Vec::new();
    for package in packages {
        if let Some(candidate) = candidates_map.get(*package) {
            if candidate.info == PK_INFO_ENUM_INSTALLED as u32 {
                // if the package is already installed and is at the latest stable version,
                // then just skip it
                continue;
            }
            result.push(candidate.package_id.clone());
            continue;
        }
        // else:
        not_found.push(package.to_string());
    }

    Ok((not_found, result))
}

/// Get the list of transaction steps (what need to be done)
pub async fn get_transaction_steps(
    proxy: &TransactionProxy<'_>,
    package_ids: &[&str],
) -> Result<Vec<PkPackage>> {
    if package_ids.is_empty() {
        return Ok(vec![]);
    }

    collect_packages(proxy, async move {
        proxy
            .install_packages(
                (PK_TRANSACTION_FLAG_ENUM_SIMULATE
                    | PK_TRANSACTION_FLAG_ENUM_ALLOW_REINSTALL
                    | PK_TRANSACTION_FLAG_ENUM_ALLOW_DOWNGRADE) as u64,
                package_ids,
            )
            .await
    })
    .await
}

async fn send_packagekit_hints(proxy: &TransactionProxy<'_>) -> zResult<()> {
    proxy
        .set_hints(&["background=false", "interactive=true"])
        .await
}

/// Execute a transaction with progress monitoring
pub async fn execute_transaction(
    proxy: &TransactionProxy<'_>,
    package_ids: &[&str],
    progress_tx: Sender<PkDisplayProgress>,
) -> Result<()> {
    // safety guard
    if package_ids.is_empty() {
        return Ok(());
    }
    // start transaction
    let fut = async {
        send_packagekit_hints(proxy).await?;
        proxy
            .update_packages(
                (PK_TRANSACTION_FLAG_ENUM_ALLOW_REINSTALL
                    | PK_TRANSACTION_FLAG_ENUM_ALLOW_DOWNGRADE) as u64,
                package_ids,
            )
            .await
    };

    // start all the monitoring facilities
    tokio::select! {
        v = monitor_item_progress(proxy, &progress_tx, fut) => v,
        v = async {
            // handle overall transaction progress
            let mut stream = proxy.receive_percentage_changed().await;
            // get the "Percentage" properties from the change signal
            // if the changed_properties does not contain our interest, just ignore it
            while let Some(event) = stream.next().await {
                let progress = event.get().await?;
                progress_tx.send(PkDisplayProgress::Overall(progress as u32))?;
            }
            Ok(())
        } => v,
        v = async {
            // periodically check if PackageKit is still alive
            let mut timer = tokio::time::interval(Duration::from_secs(3));
            loop {
                proxy.status().await?;
                timer.tick().await;
            }
        } => v
    }
}

async fn monitor_item_progress<Fut: Future<Output = zResult<()>>>(
    proxy: &TransactionProxy<'_>,
    progress_tx: &Sender<PkDisplayProgress>,
    fut: Fut,
) -> Result<()> {
    let mut signal_stream = proxy.receive_all_signals().await?;
    fut.await?;
    while let Some(signal) = signal_stream.next().await {
        let name = signal.member();
        if let Some(name) = name {
            match name.as_str() {
                // handle individual transaction item (single package progress)
                "ItemProgress" => {
                    let item: PkProgress = signal.body()?;
                    progress_tx.send(PkDisplayProgress::Package(
                        item.id,
                        item.status as u8,
                        item.percentage,
                    ))?;
                }
                "ErrorCode" => {
                    let e: PkError = signal.body()?;
                    return Err(anyhow!("({}): {}", e.code, e.details));
                }
                "Finished" | "Destroy" => {
                    progress_tx.send(PkDisplayProgress::Done)?;
                    return Ok(());
                }
                _ => continue,
            }
        }
    }

    Ok(())
}

pub fn get_task_details<'a>(
    not_found: &'a [String],
    meta: &'a [PkPackage],
) -> Result<PkTaskList<'a>> {
    let mut output = PkTaskList {
        upgrade: Vec::with_capacity(meta.len() / 4),
        install: Vec::new(),
        downgrade: Vec::new(),
        erase: Vec::new(),
        hold: not_found
            .iter()
            .map(|name| PkPackgeId {
                name,
                version: "",
                arch: "",
                data: "",
            })
            .collect(),
    };

    for m in meta {
        let parsed =
            parse_package_id(&m.package_id).ok_or_else(|| anyhow!("({})", m.package_id))?;
        match m.info as u8 {
            PK_INFO_ENUM_INSTALLING | PK_INFO_ENUM_REINSTALLING => output.install.push(parsed),
            PK_INFO_ENUM_UPDATING => output.upgrade.push(parsed),
            PK_INFO_ENUM_DOWNGRADING => output.downgrade.push(parsed),
            PK_INFO_ENUM_REMOVING => output.erase.push(parsed),
            _ => continue,
        }
    }

    Ok(output)
}

/// Take the wake lock and prevent the system from sleeping. Drop the returned file handle to release the lock.
pub async fn take_wake_lock(conn: &Connection, why: &str) -> zResult<zbus::zvariant::OwnedFd> {
    let proxy = Login1Proxy::new(&conn).await?;

    proxy.inhibit("shutdown:sleep", "atm", why, "block").await
}

pub async fn is_using_battery(conn: &Connection) -> zResult<bool> {
    let proxy = UPowerProxy::new(conn).await?;

    proxy.on_battery().await
}

pub async fn is_metered_network(conn: &Connection) -> zResult<bool> {
    let proxy = connect_packagekit(conn).await?;

    Ok(proxy.network_state().await? == PK_NETWORK_ENUM_MOBILE as u32)
}
