use std::{collections::HashSet, env::consts::ARCH};

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const PATH_TO_MANIFEST: &str = "debs/manifest/topics.json";
const APT_GEN_LIST_STATUS: &str = "/var/lib/apt/gen/status.json";
pub const DEFAULT_REPO_URL: &str = "https://repo.aosc.io";

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

#[derive(Deserialize, Debug)]
struct AptGenListStatus {
    mirror: IndexMap<String, String>,
}

pub(crate) type TopicManifests = Vec<TopicManifest>;

#[inline]
pub(crate) fn get_arch_name() -> Option<&'static str> {
    match ARCH {
        "x86_64" => Some("amd64"),
        "x86" => Some("i486"),
        "aarch64" => Some("arm64"),
        "powerpc64" => Some("ppc64el"),
        "mips64" => Some("loongson3"),
        "riscv64" => Some("riscv64"),
        "loongarch64" => Some("loongarch64"),
        _ => None,
    }
}

pub fn create_http_client() -> Result<Client, reqwest::Error> {
    Client::builder().user_agent(USER_AGENT).build()
}

async fn test_mirror<'a>(
    client: &Client,
    url: reqwest::Url,
    mirror: &'a str,
) -> Result<&'a str, reqwest::Error> {
    // HEAD request works better but some mirrors do not support it correctly
    client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .map(|_| mirror)
}

async fn get_best_mirror_url_inner(client: &Client) -> Result<String> {
    let mut file = File::open(APT_GEN_LIST_STATUS).await?;
    let mut buffer = Vec::with_capacity(1024);
    file.read_to_end(&mut buffer).await?;
    let mirrors: AptGenListStatus = serde_json::from_slice(&buffer)?;
    if mirrors.mirror.len() < 2 {
        // you don't have many choices here
        return Err(anyhow!(""));
    }
    let mut tasks = Vec::with_capacity(mirrors.mirror.len());
    for mirror in mirrors.mirror.iter() {
        let test_url = match reqwest::Url::parse(mirror.1) {
            Ok(v) => v.join(PATH_TO_MANIFEST).unwrap(),
            Err(_) => continue,
        };
        tasks.push(Box::pin(test_mirror(client, test_url, mirror.1)));
    }
    if tasks.is_empty() {
        // well, that sad, none of the mirrors have a correct URL
        return Err(anyhow!(""));
    }
    // start the test
    let result = futures::future::select_ok(tasks).await?;

    Ok(result.0.to_owned())
}

pub async fn get_best_mirror_url(client: &Client) -> String {
    get_best_mirror_url_inner(client)
        .await
        .unwrap_or_else(|_| DEFAULT_REPO_URL.to_owned())
}

fn get_sensible_mirror_url_inner() -> Result<String> {
    let mirrors: AptGenListStatus =
        serde_json::from_reader(std::fs::File::open(APT_GEN_LIST_STATUS)?)?;
    mirrors
        .mirror
        .first()
        .map(|v| v.1.to_owned())
        .ok_or_else(|| anyhow!(""))
}

/// Get a sensible mirror URL (async not needed)
pub fn get_sensible_mirror_url() -> String {
    get_sensible_mirror_url_inner().unwrap_or_else(|_| DEFAULT_REPO_URL.to_owned())
}

pub async fn fetch_topics(client: &Client, mirror_url: &str) -> Result<TopicManifests> {
    let url = reqwest::Url::parse(mirror_url)?.join(PATH_TO_MANIFEST)?;
    let resp = client.get(url).send().await?.error_for_status()?;
    let topics: TopicManifests = resp.json().await?;

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
