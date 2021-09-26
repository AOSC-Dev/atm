use argh::FromArgs;

use crate::{network, pm};

#[derive(FromArgs, PartialEq, Debug)]
/// enroll into a new topic
#[argh(subcommand, name = "add")]
pub(crate) struct TopicAdd {
    /// name of the topic
    #[argh(option)]
    pub name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// exit from a topic
#[argh(subcommand, name = "remove")]
pub(crate) struct TopicRemove {
    /// name of the topic
    #[argh(option)]
    pub name: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// exit from a topic
#[argh(subcommand, name = "refresh")]
pub(crate) struct RefreshList {
    /// filename of the topic list file
    #[argh(option, short = 'f')]
    pub filename: String,
    /// checksum of the topic list file
    #[argh(option, short = 'c')]
    pub checksum: String,
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

/// CLI parser and main function.
/// Returns `false` if no command-line argument is provided.
pub fn cli_main() -> bool {
    let args: ATM = argh::from_env();
    if args.command.is_none() {
        return false;
    }

    true
}
