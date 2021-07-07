use std::{collections::HashSet, env::consts::ARCH, fs::File, path::Path};

use anyhow::{anyhow, Result};
use clap::crate_version;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

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
