//! Packagekit related functions

mod packagekit;
mod packagekit_tx;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};
use dbus::{
    blocking::{
        stdintf::org_freedesktop_dbus::Properties,
        Connection, Proxy,
    },
    Message,
};
use packagekit::*;
use packagekit_tx::*;

use crate::fl;

pub type PkPackage = OrgFreedesktopPackageKitTransactionPackage;
pub type PkError = OrgFreedesktopPackageKitTransactionErrorCode;
pub type PkProgress = OrgFreedesktopPackageKitTransactionItemProgress;

const PACKAGEKIT_DEST: &str = "org.freedesktop.PackageKit";
const PACKAGEKIT_PATH: &str = "/org/freedesktop/PackageKit";
const LOGIN1_DEST: &str = "org.freedesktop.login1";
const LOGIN1_PATH: &str = "/org/freedesktop/login1";
const UPOWER_DEST: &str = "org.freedesktop.UPower";
const UPOWER_PATH: &str = "/org/freedesktop/UPower";

// PackageKit enumeration constants (could be OR'ed)
const PK_FILTER_ENUM_NEWEST: u32 = 1 << 16;
const PK_FILTER_ENUM_ARCH: u32 = 1 << 18;
const PK_FILTER_ENUM_NOT_SOURCE: u32 = 1 << 21;
const PK_TRANSACTION_FLAG_ENUM_SIMULATE: u32 = 1 << 2;
const PK_TRANSACTION_FLAG_ENUM_ALLOW_REINSTALL: u32 = 1 << 4;
const PK_TRANSACTION_FLAG_ENUM_ALLOW_DOWNGRADE: u32 = 1 << 6;
// PackageKit informational constants (literal values)
const PK_NETWORK_ENUM_MOBILE: u8 = 5;
// pub const PK_STATUS_ENUM_SETUP: u8 = 2;
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

pub struct PkPackgeId<'a> {
    name: &'a str,
    version: &'a str,
    arch: &'a str,
    data: &'a str,
}

pub enum PkDisplayProgress {
    /// Individual package progress (package_id, PK_STATUS, progress %)
    Package(String, u8, u32),
    /// Overall transaction progress (progress %)
    Overall(u32),
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

fn register_exit_handler(
    run: Arc<AtomicBool>,
    error: Sender<PkError>,
    proxy: &Proxy<&Connection>,
) -> Result<()> {
    let run_clone = run.clone();
    proxy.match_signal(
        move |_: OrgFreedesktopPackageKitTransactionDestroy, _: &Connection, _: &Message| {
            run.fetch_and(false, Ordering::SeqCst);
            true
        },
    )?;
    let run = run_clone.clone();
    proxy.match_signal(
        move |_: OrgFreedesktopPackageKitTransactionFinished, _: &Connection, _: &Message| {
            run_clone.fetch_and(false, Ordering::SeqCst);
            true
        },
    )?;
    proxy.match_signal(move |h: PkError, _: &Connection, _: &Message| {
        error.send(h).unwrap();
        run.fetch_and(false, Ordering::SeqCst);
        true
    })?;

    Ok(())
}

fn collect_packages<F: FnOnce(&Proxy<&Connection>) -> Result<()>>(
    proxy: &Proxy<&Connection>,
    func: F,
) -> Result<Vec<PkPackage>> {
    let run = Arc::new(AtomicBool::new(true));
    let (error_tx, error_rx) = channel();
    let (packages_tx, packages_rx) = channel();
    register_exit_handler(run.clone(), error_tx, proxy)?;
    // register package collection handler
    proxy.match_signal(move |h: PkPackage, _: &Connection, _: &Message| {
        packages_tx.send(h).unwrap();
        true
    })?;
    // execute the callback function to start the transaction
    func(proxy)?;
    // process incoming payloads
    while run.load(Ordering::SeqCst) {
        // TODO: have a hard limit on timeouts to prevent deadlock
        proxy.connection.process(Duration::from_millis(1000))?;
    }
    if let Ok(e) = error_rx.try_recv() {
        return Err(anyhow!("({}) {}", e.code, e.details));
    }
    // collect all the packages in the receiver buffer
    let packages = packages_rx.try_iter().collect();

    Ok(packages)
}

fn wait_for_exit_signal<F: FnOnce(&Proxy<&Connection>) -> Result<()>>(
    proxy: &Proxy<&Connection>,
    func: F,
) -> Result<()> {
    let run = Arc::new(AtomicBool::new(true));
    let (error_tx, error_rx) = channel();
    register_exit_handler(run.clone(), error_tx, proxy)?;
    // execute the callback function to start the transaction
    func(proxy)?;
    // process incoming payloads
    while run.load(Ordering::SeqCst) {
        // TODO: have a hard limit on timeouts to prevent deadlock
        proxy.connection.process(Duration::from_millis(1000))?;
    }
    if let Ok(e) = error_rx.try_recv() {
        return Err(anyhow!("{}: {}", e.code, e.details));
    }

    Ok(())
}

/// Connect to the D-Bus system bus
pub fn create_dbus_connection() -> Result<Connection> {
    Ok(Connection::new_system()?)
}

/// Connect to the packagekit backend
pub fn connect_packagekit(conn: &Connection) -> Result<Proxy<&Connection>> {
    let proxy = conn.with_proxy(PACKAGEKIT_DEST, PACKAGEKIT_PATH, Duration::from_secs(3));

    Ok(proxy)
}

/// A convient function to create a new PackageKit transaction session
pub fn create_transaction<'a>(proxy: &'a Proxy<&Connection>) -> Result<Proxy<'a, &'a Connection>> {
    let path = proxy.create_transaction()?;
    let tx_proxy = proxy
        .connection
        .with_proxy(PACKAGEKIT_DEST, path, Duration::from_secs(3));

    Ok(tx_proxy)
}

/// Get the ID of the Distro (e.g. debian;squeeze/sid;x86_64)
pub fn get_distro_id(proxy: &Proxy<&Connection>) -> Result<String> {
    Ok(proxy.distro_id()?)
}

/// Refresh repository cache (forcibly refreshes the caches)
pub fn refresh_cache(proxy: &Proxy<&Connection>) -> Result<()> {
    wait_for_exit_signal(proxy, |proxy| Ok(proxy.refresh_cache(true)?))
}

/// Fetch all the updatable packages (requires transaction proxy)
pub fn get_updated_packages(proxy: &Proxy<&Connection>) -> Result<Vec<PkPackage>> {
    collect_packages(proxy, |proxy| {
        proxy.get_updates(PK_FILTER_ENUM_NEWEST as u64)?;

        Ok(())
    })
}

/// Find the package ID of the stable version of the given packages, returns (not found, found) (requires transaction proxy)
pub fn find_stable_version_of(
    proxy: &Proxy<&Connection>,
    packages: &[&str],
) -> Result<(Vec<String>, Vec<String>)> {
    if packages.is_empty() {
        return Ok((vec![], vec![]));
    }
    let candidates = collect_packages(proxy, |proxy| {
        proxy.resolve(
            (PK_FILTER_ENUM_ARCH | PK_FILTER_ENUM_NOT_SOURCE) as u64,
            packages.to_vec(),
        )?;

        Ok(())
    })?;
    let mut candidates_map: HashMap<String, Vec<PkPackage>> = HashMap::new();
    for candidate in candidates {
        let candidate_parsed =
            parse_package_id(&candidate.package_id).ok_or_else(|| anyhow!("Invalid package id"))?;
        // skip packages that are not in the stable branch
        if !candidate_parsed.data.starts_with("aosc-stable-")
            && !candidate_parsed.data.starts_with("installed:aosc-stable-")
        {
            continue;
        }
        if let Some(packages) = candidates_map.get_mut(candidate_parsed.name) {
            packages.push(candidate);
        } else {
            candidates_map.insert(candidate_parsed.name.to_string(), vec![candidate]);
        }
    }
    let mut result = Vec::new();
    let mut not_found = Vec::new();
    for package in packages {
        if let Some(candidates) = candidates_map.get(*package) {
            if let Some(candidate) = candidates.first() {
                if candidate.info == PK_INFO_ENUM_INSTALLED as u32 {
                    // if the package is already installed and is at the latest stable version,
                    // then just skip it
                    continue;
                }
                result.push(candidate.package_id.clone());
                continue;
            }
        }
        // else:
        not_found.push(package.to_string());
    }

    Ok((not_found, result))
}

/// Get the list of transaction steps (what need to be done)
pub fn get_transaction_steps(
    proxy: &Proxy<&Connection>,
    package_ids: &[&str],
) -> Result<Vec<PkPackage>> {
    if package_ids.is_empty() {
        return Ok(vec![]);
    }

    collect_packages(proxy, |proxy| {
        proxy.install_packages(
            (PK_TRANSACTION_FLAG_ENUM_SIMULATE
                | PK_TRANSACTION_FLAG_ENUM_ALLOW_REINSTALL
                | PK_TRANSACTION_FLAG_ENUM_ALLOW_DOWNGRADE) as u64,
            package_ids.to_vec(),
        )?;

        Ok(())
    })
}

/// Execute a transaction with progress monitoring
pub fn execute_transaction(
    proxy: &Proxy<&Connection>,
    package_ids: &[&str],
    progress_tx: Sender<PkDisplayProgress>,
) -> Result<()> {
    use dbus::arg::RefArg;
    use dbus::blocking::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;

    let run = Arc::new(AtomicBool::new(true));
    let progress_tx_clone = progress_tx.clone();
    let (error_tx, error_rx) = channel();
    register_exit_handler(run.clone(), error_tx, proxy)?;
    // handle individual transaction item (single package progress)
    proxy.match_signal(move |item: PkProgress, _: &Connection, _: &Message| {
        progress_tx
            .send(PkDisplayProgress::Package(
                item.id,
                item.status as u8,
                item.percentage,
            ))
            .unwrap();

        true
    })?;
    // handle overall transaction progress
    proxy.match_signal(
        move |prop: PropertiesPropertiesChanged, _: &Connection, _: &Message| {
            // get the "Percentage" properties from the change signal
            // if the changed_properties does not contain our interest, just ignore it
            if let Some(progress) = prop
                .changed_properties
                .get("Percentage")
                .and_then(|p| p.as_u64())
            {
                progress_tx_clone
                    .send(PkDisplayProgress::Overall(progress as u32))
                    .unwrap();
            }

            true
        },
    )?;
    // start transaction
    proxy.install_packages(
        (PK_TRANSACTION_FLAG_ENUM_ALLOW_REINSTALL | PK_TRANSACTION_FLAG_ENUM_ALLOW_DOWNGRADE)
            as u64,
        package_ids.to_vec(),
    )?;
    let mut last_received = Instant::now();
    // process incoming payloads
    while run.load(Ordering::SeqCst) {
        proxy.connection.process(Duration::from_secs(3))?;
        // Ping the PackageKit daemon if signal is not received in time
        // to see if it is still alive
        if last_received.elapsed() >= Duration::from_millis(2500) {
            // are ya crashing son?
            proxy.status()?;
            // update last_received
            last_received = Instant::now();
        }
    }
    if let Ok(e) = error_rx.try_recv() {
        return Err(anyhow!("({}): {}", e.code, e.details));
    }

    Ok(())
}

pub fn get_task_summary(not_found: &[String], meta: &[PkPackage]) -> String {
    let mut installs = 0usize;
    let mut updates = 0usize;
    let mut erases = 0usize;
    let mut summary = String::new();

    for m in meta {
        match m.info as u8 {
            PK_INFO_ENUM_INSTALLING | PK_INFO_ENUM_REINSTALLING => installs += 1,
            PK_INFO_ENUM_DOWNGRADING | PK_INFO_ENUM_UPDATING => updates += 1,
            PK_INFO_ENUM_REMOVING => erases += 1,
            _ => continue,
        }
    }

    if not_found.len() > 0 {
        summary += &fl!("no_stable_version", count = not_found.len());
        summary.push('\n');
    }
    if installs > 0 {
        summary += &fl!("install_count", count = installs);
        summary.push('\n');
    }
    if updates > 0 {
        summary += &fl!("update_count", count = updates);
        summary.push('\n');
    }
    if erases > 0 {
        summary += &fl!("erase_count", count = erases);
        summary.push('\n');
    }

    if installs < 1 && erases < 1 && updates < 1 {
        summary += &fl!("nothing");
        summary.push('\n');
    }

    summary
}

pub fn get_task_details(not_found: &[String], meta: &[PkPackage]) -> Result<String> {
    let mut output = fl!("tx_body");
    output.push_str("\n\n");

    for name in not_found {
        output += &fl!("tx_hold", package = name.as_str());
        output.push('\n');
    }

    for m in meta {
        let parsed =
            parse_package_id(&m.package_id).ok_or_else(|| anyhow!("Invalid package id"))?;
        let name = parsed.name;
        let version = parsed.version;
        match m.info as u8 {
            PK_INFO_ENUM_INSTALLING | PK_INFO_ENUM_REINSTALLING => {
                output += &fl!("tx_install", package = name, version = version)
            }
            PK_INFO_ENUM_UPDATING => {
                output += &fl!("tx_upgrade", package = name, version = version)
            }
            PK_INFO_ENUM_DOWNGRADING => {
                output += &fl!("tx_downgrade", package = name, version = version)
            }
            PK_INFO_ENUM_REMOVING => output += &fl!("tx_erase", package = name, version = version),
            _ => continue,
        }
        output.push('\n');
    }

    Ok(output)
}

/// Take the wake lock and prevent the system from sleeping. Drop the returned file handle to release the lock.
pub fn take_wake_lock() -> Result<dbus::arg::OwnedFd> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(LOGIN1_DEST, LOGIN1_PATH, Duration::from_secs(3));
    let (cookie,): (dbus::arg::OwnedFd,) = proxy.method_call(
        "org.freedesktop.login1.Manager",
        "Inhibit",
        ("shutdown:sleep", "atm", fl!("pk_inhibit_message"), "block"),
    )?;

    Ok(cookie)
}

pub fn is_using_battery() -> Result<bool> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(UPOWER_DEST, UPOWER_PATH, Duration::from_secs(3));
    let result: bool = proxy.get(UPOWER_DEST, "OnBattery")?;

    Ok(result)
}

pub fn is_metered_network() -> Result<bool> {
    let conn = Connection::new_system()?;
    let proxy = connect_packagekit(&conn)?;

    Ok(proxy.network_state()? == PK_NETWORK_ENUM_MOBILE as u32)
}
