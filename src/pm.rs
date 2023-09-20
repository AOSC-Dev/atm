use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Write as WriteFmt,
    fs,
    io::Write,
};

use crate::network::{TopicManifest, TopicManifests};
use crate::parser::list_installed;
use crate::pk::{
    create_transaction, find_stable_version_of, get_updated_packages, refresh_cache,
    PackageKitProxy,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{from_reader, to_string};

const SOURCE_HEADER: &[u8] = b"# Generated by AOSC Topic Manager. DO NOT EDIT THIS FILE!\n";
const SOURCE_PATH: &str = "/etc/apt/sources.list.d/atm.list";
const STATE_PATH: &str = "/var/lib/atm/state";
const STATE_DIR: &str = "/var/lib/atm/";
const DPKG_STATE: &str = "/var/lib/dpkg/status";

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
    let installed = list_installed::<()>(&mut state_file.as_slice())?;
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

/// Returns the list of enrolled topics
fn get_previous_topics() -> Result<PreviousTopics> {
    Ok(from_reader(fs::File::open(STATE_PATH)?)?)
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

fn normalize_url(url: &str) -> Cow<str> {
    if url.ends_with('/') {
        Cow::Borrowed(url)
    } else {
        let mut url = Cow::from(url);
        url.to_mut().push('/');
        url
    }
}

fn make_topic_list(topics: &[&TopicManifest], mirror_url: &str) -> String {
    let mut output = String::with_capacity(1024);

    for topic in topics {
        writeln!(
            &mut output,
            "# Topic `{}`\ndeb {}debs {} main",
            topic.name,
            normalize_url(mirror_url),
            topic.name
        )
        .unwrap();
        // this will only happen when malloc() fails.
        // in which case, it's better to just panic
    }

    output
}

pub fn write_source_list(topics: &[&TopicManifest], mirror_url: &str) -> Result<()> {
    let mut f = fs::File::create(SOURCE_PATH)?;
    f.write_all(SOURCE_HEADER)?;
    f.write_all(make_topic_list(topics, mirror_url).as_bytes())?;

    fs::create_dir_all(STATE_DIR)?;
    let mut f = fs::File::create(STATE_PATH)?;
    f.write_all(save_as_previous_topics(topics)?.as_bytes())?;

    Ok(())
}

pub async fn switch_topics(
    proxy: &PackageKitProxy<'_>,
    closed: &[TopicManifest],
) -> Result<(Vec<String>, Vec<String>)> {
    let tx_proxy = create_transaction(proxy).await?;
    refresh_cache(&tx_proxy).await?;
    let removed = close_topics(closed)?;
    let removed = removed.iter().map(|x| x.as_str()).collect::<Vec<_>>();
    let tx_proxy = create_transaction(proxy).await?;
    let (not_found, tasks) = find_stable_version_of(&tx_proxy, &removed).await?;
    let tx_proxy = create_transaction(proxy).await?;
    let updated = get_updated_packages(&tx_proxy).await?;
    let mut updated = updated
        .into_iter()
        .map(|x| x.package_id)
        .collect::<Vec<_>>();
    updated.extend(tasks);

    Ok((not_found, updated))
}
