use std::{fs::File, io::Read, path::Path, process};

use anyhow::{anyhow, Result};
use argh::FromArgs;
use sha2::Digest;

use super::format_timestamp;
use crate::{fl, network, pm};

#[derive(FromArgs, PartialEq, Debug)]
/// enroll into a new topic
#[argh(subcommand, name = "add")]
pub(crate) struct TopicAdd {
    /// name of the topic
    #[argh(positional)]
    pub name: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// exit from a topic
#[argh(subcommand, name = "remove")]
pub(crate) struct TopicRemove {
    /// name of the topic
    #[argh(positional)]
    pub name: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// refresh APT configurations
#[argh(subcommand, name = "refresh")]
pub(crate) struct RefreshList {
    /// filename of the topic list file (optional)
    #[argh(option, short = 'f')]
    pub filename: Option<String>,
    /// checksum of the topic list file (optional)
    #[argh(option, short = 'c')]
    pub checksum: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// list current topics and available topics
#[argh(subcommand, name = "list")]
pub(crate) struct TopicList {}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
pub(crate) enum ATMCommand {
    List(TopicList),
    Refresh(RefreshList),
    Add(TopicAdd),
    Remove(TopicRemove),
}

#[derive(FromArgs, PartialEq, Debug)]
/// AOSC Topic Manager
pub(crate) struct ATM {
    #[argh(subcommand)]
    pub command: Option<ATMCommand>,
}

// === end of argh constructs

#[inline]
fn needs_root() -> Result<()> {
    use nix::unistd::geteuid;

    if !geteuid().is_root() {
        Err(anyhow!(fl!("needs-root")))
    } else {
        Ok(())
    }
}

/// Escalate permissions using Polkit-1 and write configuration file
pub fn privileged_write_source_list(topics: &[&network::TopicManifest]) -> Result<()> {
    use nix::unistd::geteuid;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tempfile::NamedTempFile;

    if geteuid().is_root() {
        // already root
        return pm::write_source_list(topics);
    }
    if std::env::var("DISPLAY").is_err() {
        return Err(anyhow!(fl!("headless-sudo-unsupported")));
    }
    let my_name = std::env::current_exe()?;
    let xfer_content = serde_json::to_vec(topics)?;
    // calculate hash and pass the hash to the privileged process prevent hijack attacks
    let mut chksum = sha2::Sha256::new();
    chksum.update(&xfer_content);
    let chksum = format!("{:02x}", chksum.finalize());
    // create a temporary file to transfer the states
    let mut f = NamedTempFile::new()?;
    f.write_all(&xfer_content)?;
    // pass the temporary file to the privileged process
    let cmd = Command::new("pkexec")
        .arg(my_name)
        .args(&["refresh", "-c", chksum.as_str(), "-f"])
        .arg(f.path())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| anyhow!(fl!("sudo-failure")))?
        .wait_with_output()?;
    if !cmd.status.success() {
        let error_message = std::str::from_utf8(&cmd.stderr).unwrap_or_default();
        return Err(anyhow!(fl!(
            "authentication-failure",
            reason = error_message
        )));
    }

    Ok(())
}

fn fetch_available_topics() -> Result<network::TopicManifests> {
    let topics = network::fetch_topics(&format!(
        "{}{}",
        pm::MIRROR_URL.to_string(),
        "debs/manifest/topics.json"
    ))?;

    network::filter_topics(topics)
}

fn format_manifests(topics: network::TopicManifests) {
    use std::io::Write;

    let mut formatter = tabwriter::TabWriter::new(std::io::stderr());
    write!(
        &mut formatter,
        "  {}\t{}\t{}\n",
        fl!("name"),
        fl!("date"),
        fl!("description")
    )
    .unwrap();
    for topic in topics {
        write!(
            &mut formatter,
            "{} {}\t{}\t{}\n",
            if topic.enabled { "*" } else { " " },
            topic.name,
            format_timestamp(topic.date).unwrap_or_else(|_| "?".to_string()),
            topic.description.unwrap_or_default()
        )
        .unwrap();
    }
    formatter.flush().unwrap();
}

fn list_topics() {
    let mut fallback = false;
    eprintln!("{}", fl!("refresh_manifest"));
    let available = fetch_available_topics().unwrap_or_else(|_| {
        fallback = true;
        Vec::new()
    });
    let mut topics = pm::get_display_listing(available);
    topics.sort_unstable_by_key(|t| !t.enabled);
    format_manifests(topics);
    if fallback {
        eprintln!("{}", fl!("fetch-error-fallback"));
    } else {
        eprintln!("\n{}", fl!("topic-table-hint"));
    }
}

fn refresh_topics<P: AsRef<Path>>(filename: Option<P>, chksum: &Option<String>) -> Result<()> {
    needs_root()?;
    let topics = match filename {
        Some(filename) => {
            let mut f = File::open(filename)?;
            let mut buffer = Vec::new();
            buffer.reserve(1024);
            f.read_to_end(&mut buffer)?;
            if let Some(chksum) = chksum {
                let mut hasher = sha2::Sha256::new();
                hasher.update(&buffer);
                if &format!("{:02x}", hasher.finalize()) != chksum {
                    return Err(anyhow!("Hash mismatch."));
                }
            }

            serde_json::from_slice(&buffer)?
        }
        None => {
            let mut topics = pm::get_display_listing(Vec::new());
            topics.iter_mut().for_each(|t| t.enabled = true);

            topics
        }
    };
    let topics_ref = topics.iter().map(|t| t).collect::<Vec<_>>();
    pm::write_source_list(&topics_ref)?;
    println!("{}", fl!("apt_finished"));

    Ok(())
}

fn add_topics(topics_to_add: &[String]) -> Result<()> {
    needs_root()?;
    eprintln!("{}", fl!("refresh_manifest"));
    let available = fetch_available_topics()?;
    let mut topics = pm::get_display_listing(available);
    for topic in topics.iter_mut() {
        topic.enabled = topics_to_add.contains(&topic.name);
    }
    let topics_ref = topics.iter().map(|t| t).collect::<Vec<_>>();
    pm::write_source_list(&topics_ref)?;
    println!("{}", fl!("apt_finished"));

    Ok(())
}

fn remove_topics(topics_to_remove: &[String]) -> Result<()> {
    needs_root()?;
    let mut topics = pm::get_display_listing(Vec::new());
    topics
        .iter_mut()
        .for_each(|t| t.enabled = !topics_to_remove.contains(&t.name));
    let topics_ref = topics.iter().map(|t| t).collect::<Vec<_>>();
    pm::write_source_list(&topics_ref)?;
    println!("{}", fl!("apt_finished"));

    Ok(())
}

/// CLI parser and main function.
/// Returns `false` if no command-line argument is provided.
pub fn cli_main() -> bool {
    let args: ATM = argh::from_env();
    if args.command.is_none() {
        return false;
    }
    let commands = args.command.unwrap();
    match commands {
        ATMCommand::List(_) => list_topics(),
        ATMCommand::Refresh(args) => {
            if let Err(e) = refresh_topics(args.filename, &args.checksum) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        ATMCommand::Add(topics) => {
            if let Err(e) = add_topics(&topics.name) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
        ATMCommand::Remove(topics) => {
            if let Err(e) = remove_topics(&topics.name) {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }

    true
}
