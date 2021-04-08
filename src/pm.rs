use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
};

use crate::fl;
use crate::i18n::I18N_LOADER;
use crate::solv::Task;
use crate::{network::make_new_client, parser::list_installed};
use crate::{
    network::{fetch_manifests, get_arch_name, TopicManifest, TopicManifests},
    solv::{calculate_deps, populate_pool, PackageAction, PackageMeta, Pool, Transaction},
};
use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use libc::c_int;
use libsolv_sys::ffi::{SOLVER_DISTUPGRADE, SOLVER_SOLVABLE_ALL, SOLVER_UPDATE};
use serde::{Deserialize, Serialize};
use serde_json::{from_slice, to_string};

const SOURCE_HEADER: &[u8] = b"# Generated by AOSC Topic Manager. DO NOT EDIT THIS FILE!\n";
const SOURCE_PATH: &str = "/etc/apt/sources.list.d/atm.list";
const STATE_PATH: &str = "/var/lib/atm/state";
const STATE_DIR: &str = "/var/lib/atm/";
const DPKG_STATE: &str = "/var/lib/dpkg/status";
const APT_CACHE_PATH: &str = "/var/cache/apt/archives";
const APT_GEN_LIST_STATUS: &str = "/var/lib/apt/gen/status.json";
const MIRRORS_DATA: &str = "/usr/share/distro-repository-data/mirrors.yml";
const DEFAULT_REPO_URL: &str = "https://repo.aosc.io";

#[derive(Deserialize, Debug)]
struct AptGenListStatus {
    mirror: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Mirror {
    url: String,
}

pub fn get_mirror_url() -> Result<String> {
    let status_data = fs::read(APT_GEN_LIST_STATUS)?;
    let status_data: AptGenListStatus = serde_json::from_slice(&status_data)?;
    let mirror;
    if let Some(m) = status_data.mirror.get(0) {
        mirror = m.to_string();
    } else {
        mirror = "origin".to_string();
    }

    let mirrors_data = fs::read(MIRRORS_DATA)?;
    let mirror_data: HashMap<String, Mirror> = serde_yaml::from_slice(&mirrors_data)?;
    if let Some(mirror) = mirror_data.get(&mirror) {
        return Ok(mirror.url.to_string());
    }

    Ok(DEFAULT_REPO_URL.to_string())
}

lazy_static! {
    pub static ref MIRROR_URL: String =
        get_mirror_url().unwrap_or_else(|_| "https://repo.aosc.io/".to_string());
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreviousTopic {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub date: i64,
    pub packages: Vec<String>,
}

type PreviousTopics = Vec<PreviousTopic>;

/// Returns the packages need to be reinstalled
pub fn close_topics(topics: &[TopicManifest]) -> Result<Vec<String>> {
    let state_file = fs::read(DPKG_STATE)?;
    let installed = list_installed(&state_file)?;
    let mut remove = Vec::new();

    for topic in topics {
        for package in topic.packages.iter() {
            if installed.contains(package) {
                remove.push(package.clone());
            }
        }
    }

    Ok(remove)
}

fn get_previous_topics() -> Result<PreviousTopics> {
    let f = fs::read(STATE_PATH)?;

    Ok(from_slice(&f)?)
}

pub fn get_display_listing(current: TopicManifests) -> TopicManifests {
    let prev = get_previous_topics().unwrap_or_default();
    let mut lookup: HashMap<String, TopicManifest> = HashMap::new();
    let current_len = current.len();

    for topic in current.into_iter() {
        lookup.insert(topic.name.clone(), topic);
    }

    let mut concatenated = Vec::new();
    concatenated.reserve(prev.len() + current_len);
    for topic in prev {
        if let Some(topic) = lookup.get_mut(&topic.name) {
            topic.enabled = true;
            continue;
        }
        concatenated.push(TopicManifest {
            enabled: false,
            closed: true,
            name: topic.name.clone(),
            description: topic.description.clone(),
            date: topic.date,
            arch: HashSet::new(),
            packages: topic.packages.clone(),
        });
    }
    // consume the lookup table and append all the elements to the concatenated list
    for topic in lookup.into_iter() {
        concatenated.push(topic.1);
    }

    concatenated
}

fn save_as_previous_topics(current: &[&TopicManifest]) -> Result<String> {
    let mut previous_topics = Vec::new();
    for topic in current {
        if !topic.enabled {
            continue;
        }
        previous_topics.push(PreviousTopic {
            name: topic.name.clone(),
            description: topic.description.clone(),
            date: topic.date,
            packages: topic.packages.clone(),
        });
    }

    Ok(to_string(&previous_topics)?)
}

fn make_topic_list(topics: &[&TopicManifest]) -> String {
    let mut output = String::new();
    output.reserve(1024);

    for topic in topics {
        output.push_str(&format!(
            "# Topic `{}`\ndeb {} {} main\n",
            topic.name,
            format!("{}{}", MIRROR_URL.to_string(), "debs"),
            topic.name
        ));
    }

    output
}

pub fn write_source_list(topics: &[&TopicManifest]) -> Result<()> {
    let mut f = fs::File::create(SOURCE_PATH)?;
    f.write_all(SOURCE_HEADER)?;
    f.write_all(make_topic_list(topics).as_bytes())?;

    fs::create_dir_all(STATE_DIR)?;
    let mut f = fs::File::create(STATE_PATH)?;
    f.write_all(save_as_previous_topics(topics)?.as_bytes())?;

    Ok(())
}

pub fn make_resolve_request(remove: &[String]) -> Vec<Task> {
    let mut requests = remove
        .iter()
        .map(|x| Task {
            name: Some(x.clone()),
            flags: SOLVER_DISTUPGRADE as c_int,
        })
        .collect::<Vec<Task>>();
    requests.push(Task {
        name: None,
        flags: (SOLVER_DISTUPGRADE | SOLVER_SOLVABLE_ALL) as c_int,
    });

    requests
}

pub fn switch_topics(
    pool: &mut Pool,
    enabled: &[TopicManifest],
    closed: &[TopicManifest],
) -> Result<Transaction> {
    let client = make_new_client()?;
    let mut manifests = Vec::new();
    let url = format!("{}/debs", *MIRROR_URL);
    let arch = get_arch_name().ok_or_else(|| anyhow!(""))?;
    for topic in enabled {
        let result = fetch_manifests(&client, &url, &topic.name, &["all", arch], &["main"])?;
        manifests.extend(result);
    }
    let manifests: Vec<PathBuf> = manifests.into_iter().map(|x| PathBuf::from(x)).collect();
    populate_pool(pool, &manifests)?;
    let removed = close_topics(closed)?;
    let tasks = make_resolve_request(&removed);
    let transaction = calculate_deps(pool, &tasks)?;

    Ok(transaction)
}

pub fn get_task_summary(meta: &[PackageMeta]) -> String {
    let mut installs = 0usize;
    let mut updates = 0usize;
    let mut erases = 0usize;
    let mut summary = String::new();

    for m in meta {
        match m.action {
            PackageAction::Install(_) => installs += 1,
            PackageAction::Upgrade | PackageAction::Downgrade => updates += 1,
            PackageAction::Erase => erases += 1,
            _ => continue,
        }
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

    summary
}

pub fn get_task_details(meta: &[PackageMeta]) -> String {
    let mut output = fl!("tx_body");
    output.push_str("\n\n");

    for m in meta {
        let name = m.name.clone();
        let version = m.version.clone();
        match m.action {
            PackageAction::Install(_) => {
                output += &fl!("tx_install", package = name, version = version)
            }
            PackageAction::Upgrade => {
                output += &fl!("tx_upgrade", package = name, version = version)
            }
            PackageAction::Downgrade => {
                output += &fl!("tx_downgrade", package = name, version = version)
            }
            PackageAction::Erase => output += &fl!("tx_erase", package = name, version = version),
            PackageAction::Noop => (),
        }
        output.push('\n');
    }

    output
}

#[inline]
fn run_dpkg(args: &[&str]) -> Result<()> {
    let status = Command::new("dpkg")
        .args(args)
        .arg("--no-triggers")
        .status()?;
    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!(fl!("dpkg_error", status = code)));
    }

    Ok(())
}

pub fn execute_resolve_response(jobs: &[PackageMeta]) -> Result<()> {
    for job in jobs {
        match job.action {
            PackageAction::Noop => {}
            PackageAction::Install(_) => run_dpkg(&["--auto-deconfigure", "--unpack", &job.name])?,
            PackageAction::Erase => run_dpkg(&["--auto-deconfigure", "-r", &job.name])?,
            // copied from apt
            PackageAction::Downgrade | PackageAction::Upgrade => run_dpkg(&[
                "--auto-deconfigure",
                "--force-remove-protected",
                "--unpack",
                &job.name,
            ])?,
        }
    }
    // force configure all first
    run_dpkg(&[
        "--configure",
        "--pending",
        "--force-configure-any",
        "--force-depends",
    ])
    .ok();
    // configure anything that failed during the force configure step
    run_dpkg(&["--configure", "-a"]).ok();

    Ok(())
}
