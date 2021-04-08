use std::{
    collections::HashSet,
    env::consts::ARCH,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::sleep,
    time::Duration,
};

use anyhow::{anyhow, Result};
use clap::crate_version;
use rayon::prelude::*;
use reqwest::{blocking::Client, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::fl;
use crate::solv::{PackageAction, PackageMeta};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TopicManifest {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub closed: bool,
    pub name: String,
    pub description: Option<String>,
    pub date: i64,
    pub arch: HashSet<String>,
    pub packages: Vec<String>,
}

pub(crate) type TopicManifests = Vec<TopicManifest>;

/// Calculate the Sha256 checksum of the given stream
pub fn sha256sum<R: Read>(mut reader: R) -> Result<String> {
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher)?;

    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256sum_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;

    sha256sum(&mut f)
}

#[inline]
pub(crate) fn get_arch_name() -> Option<&'static str> {
    match ARCH {
        "x86_64" => Some("amd64"),
        "x86" => Some("i486"),
        "aarch64" => Some("arm64"),
        "powerpc64" => Some("ppc64el"),
        "mips64" => Some("loongson3"),
        _ => None,
    }
}

pub fn fetch_topics(url: &str) -> Result<TopicManifests> {
    let resp = Client::new().get(url).send()?;
    let topics: TopicManifests = resp.json()?;

    Ok(topics)
}

pub fn filter_topics(topics: TopicManifests) -> Result<TopicManifests> {
    let mut filtered: TopicManifests = Vec::new();
    filtered.reserve(topics.len());
    let arch = get_arch_name().ok_or_else(|| anyhow!("unknown architecture"))?;

    for topic in topics {
        if topic.arch.contains("all") || topic.arch.contains(arch) {
            filtered.push(topic);
        }
    }

    Ok(filtered)
}

pub fn make_new_client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(format!("ATM/{}", crate_version!()))
        .build()?)
}

pub fn fetch_url(client: &Client, url: &str, path: &Path) -> Result<()> {
    let mut f = File::create(path)?;
    let mut resp = client.get(url).send()?;
    resp.error_for_status_ref()?;
    resp.copy_to(&mut f)?;

    Ok(())
}

#[inline]
fn combination<'a, 'b>(a: &'a [&str], b: &'b [&str]) -> Vec<(&'a str, &'b str)> {
    let mut ret = Vec::new();
    for i in a {
        for j in b {
            ret.push((*i, *j));
        }
    }

    ret
}

pub fn fetch_manifests(
    client: &Client,
    mirror: &str,
    branch: &str,
    arches: &[&str],
    comps: &[&str],
) -> Result<Vec<String>> {
    let manifests = Arc::new(Mutex::new(Vec::new()));
    let manifests_clone = manifests.clone();
    let combined = combination(arches, comps);
    combined
        .par_iter()
        .try_for_each(move |(arch, comp)| -> Result<()> {
            let url = format!(
                "{}/dists/{}/{}/binary-{}/Packages",
                mirror, branch, comp, arch
            );
            let parsed = Url::parse(&url)?;
            let manifest_name = parsed.host_str().unwrap_or_default().to_string() + parsed.path();
            let manifest_name = manifest_name.replace('/', "_");
            let manifest_path = Path::new("/var/lib/apt/lists").join(manifest_name.clone());
            let result = fetch_url(client, &url, &manifest_path);
            if result.is_err() {
                if branch == "stable" {
                    return Err(result.unwrap_err());
                } else {
                    return Ok(());
                }
            }
            manifests_clone
                .lock()
                .unwrap()
                .push(manifest_path.to_string_lossy().to_string());

            Ok(())
        })?;

    Ok(Arc::try_unwrap(manifests).unwrap().into_inner().unwrap())
}

pub fn batch_download(pkgs: &[PackageMeta], mirror: &str, root: &Path) -> Result<()> {
    for i in 0..3 {
        if batch_download_inner(pkgs, mirror, root).is_ok() {
            return Ok(());
        }
        eprintln!("[{}/3] Retrying ...", i + 1);
        sleep(Duration::from_secs(2));
    }

    Err(anyhow!(fl!("exe_batch_error")))
}

fn batch_download_inner(pkgs: &[PackageMeta], mirror: &str, root: &Path) -> Result<()> {
    let client = make_new_client()?;
    let total = pkgs.len();
    let count = AtomicUsize::new(0);
    let error = AtomicBool::new(false);
    pkgs.par_iter().for_each_init(
        move || client.clone(),
        |client, pkg| {
            let filename = PathBuf::from(pkg.path.clone());
            let name = pkg.name.as_str();
            count.fetch_add(1, Ordering::SeqCst);
            println!(
                "{}",
                fl!(
                    "exe_download",
                    curr = count.load(Ordering::SeqCst),
                    total = total,
                    name = name
                )
            );
            match pkg.action {
                PackageAction::Erase | PackageAction::Noop => return,
                _ => {}
            }
            if let Some(filename) = filename.file_name() {
                let path = root.join(filename);
                if !path.is_file()
                    && fetch_url(client, &format!("{}/{}", mirror, pkg.path), &path).is_err()
                {
                    error.store(true, Ordering::SeqCst);
                    eprintln!("{}", fl!("exe_download_file_error", name = name));
                    return;
                }
                println!(
                    "{}",
                    fl!(
                        "exe_verify",
                        curr = count.load(Ordering::SeqCst),
                        total = total,
                        name = name
                    )
                );
                if let Ok(checksum) = sha256sum_file(&path) {
                    if checksum == pkg.sha256 {
                        return;
                    }
                }
                std::fs::remove_file(path).ok();
                error.store(true, Ordering::SeqCst);
                eprintln!("{}", fl!("exe_verify_error", name = name));
                return;
            } else {
                error.store(true, Ordering::SeqCst);
                eprintln!("{}", fl!("exe_path_error", name = name));
            }
        },
    );

    if error.load(Ordering::SeqCst) {
        return Err(anyhow!(fl!("exe_download_error")));
    }

    Ok(())
}

#[test]
fn test_filter() {
    get_arch_name().unwrap();
    let mut all = HashSet::new();
    all.insert("all".to_owned());
    let mut no = HashSet::new();
    no.insert("not".to_owned());
    let topics = vec![
        TopicManifest {
            enabled: false,
            closed: false,
            name: "test".to_string(),
            description: None,
            date: 0,
            arch: all.clone(),
            packages: vec![],
        },
        TopicManifest {
            enabled: false,
            closed: false,
            name: "test2".to_string(),
            description: None,
            date: 0,
            arch: no,
            packages: vec![],
        },
    ];
    assert_eq!(filter_topics(topics).unwrap().len(), 1);
}
