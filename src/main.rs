use std::sync::mpsc;
use std::time::Duration;
use std::{cmp::Ordering, collections::HashSet};

use chrono::prelude::*;
use cursive::{align::HAlign, traits::*, views::DummyView, views::LinearLayout};
use cursive::{views::Dialog, views::TextView, Cursive};
use cursive_table_view::{TableView, TableViewItem};

mod network;
mod parser;
mod pm;

const DEFAULT_MANIFEST_URL: &str = "http://localhost:8080/debs/manifest/topics.json";

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum TopicColumn {
    Enabled,
    Name,
    Date,
    Description,
}

impl TableViewItem<TopicColumn> for network::TopicManifest {
    fn to_column(&self, column: TopicColumn) -> String {
        match column {
            TopicColumn::Enabled => {
                if self.enabled {
                    " ✓".to_string()
                } else {
                    " ".to_string()
                }
            }
            TopicColumn::Name => {
                let mut name = self.name.clone();
                if self.closed {
                    name.push_str(" [closed]");
                }
                name
            }
            TopicColumn::Date => Utc.timestamp(self.date, 0).format("%Y-%m-%d").to_string(),
            TopicColumn::Description => self.description.clone().unwrap_or(String::new()),
        }
    }
    fn cmp(&self, other: &Self, column: TopicColumn) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            TopicColumn::Enabled => self.enabled.cmp(&other.enabled),
            TopicColumn::Name => self.name.cmp(&other.name),
            TopicColumn::Date => self.date.cmp(&other.date),
            TopicColumn::Description => self.description.cmp(&other.description),
        }
    }
}

macro_rules! unwrap_or_show_error {
    ($siv:ident, $f:block) => {{
        let tmp = { $f };
        if let Err(e) = tmp {
            show_error($siv, &e.to_string());
            return;
        }
        tmp.unwrap()
    }};
    ($siv:ident, $x:ident) => {{
        if let Err(e) = $x {
            show_error($siv, &e.to_string());
            return;
        }
        $x.unwrap()
    }};
}

fn show_blocking_message(siv: &mut Cursive, msg: &str) {
    siv.add_layer(
        Dialog::around(TextView::new(msg))
            .title("Message")
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn show_error(siv: &mut Cursive, msg: &str) {
    siv.add_layer(
        Dialog::around(TextView::new(msg))
            .title("Error")
            .button("Exit", |s| s.quit())
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn commit_changes(siv: &mut Cursive) {
    let mut previous: Option<Vec<network::TopicManifest>> = None;
    if let Some(prev) = siv.user_data::<Vec<network::TopicManifest>>() {
        previous = Some(prev.clone());
    }
    let mut reinstall = Vec::new();
    let (tx, rx) = mpsc::channel();
    siv.call_on_name(
        "topic",
        |v: &mut TableView<network::TopicManifest, TopicColumn>| {
            let items = v.borrow_items();
            let mut enabled = Vec::new();
            let mut lookup = HashSet::new();
            for item in items {
                if item.enabled {
                    enabled.push(item);
                    lookup.insert(item.name.clone());
                }
            }
            // figure out what packages to re-install
            if let Some(previous) = previous {
                for item in previous {
                    if !lookup.contains(&item.name) {
                        reinstall.push(item);
                    }
                }
            }
            tx.send(pm::write_source_list(&enabled)).ok();
        },
    );
    let result = unwrap_or_show_error!(siv, { rx.recv_timeout(Duration::from_secs(10)) });
    unwrap_or_show_error!(siv, { result });
    let install_cmd: Vec<String> = unwrap_or_show_error!(siv, { pm::close_topics(&reinstall) });

    siv.add_layer(
        Dialog::around(TextView::new("APT configuration updated successfully."))
            .title("Message")
            .button("OK", |s| {
                s.pop_layer();
            })
            .padding_lrtb(2, 2, 1, 1),
    );
    // save and quit the current cursive session
    let dump = siv.dump();
    siv.quit();
    siv.set_user_data((install_cmd, dump));
}

fn fetch_manifest(siv: &mut Cursive) {
    show_blocking_message(siv, "Fetching manifest...");
    siv.refresh();
    siv.step();
    let manifest = unwrap_or_show_error!(siv, { network::fetch_topics(DEFAULT_MANIFEST_URL) });
    let filtered = unwrap_or_show_error!(siv, {
        let topics = network::filter_topics(manifest);
        match topics {
            Ok(topics) => Ok(pm::get_display_listing(topics)),
            Err(e) => Err(e),
        }
    });
    let has_closed = filtered.iter().find(|x| x.closed).is_some();
    siv.refresh();
    let view = TableView::<network::TopicManifest, TopicColumn>::new()
        .column(TopicColumn::Enabled, "", |c| {
            c.align(HAlign::Center).width(4)
        })
        .column(TopicColumn::Name, "Name", |c| c.ordering(Ordering::Greater))
        .column(TopicColumn::Date, "Date", |c| c)
        .column(TopicColumn::Description, "Description", |c| c)
        .items(filtered)
        .on_submit(|siv, _, index| {
            siv.call_on_name(
                "topic",
                |v: &mut TableView<network::TopicManifest, TopicColumn>| {
                    if let Some(item) = v.borrow_item_mut(index) {
                        item.enabled = !item.enabled;
                        v.needs_relayout();
                    }
                },
            );
        })
        .with_name("topic")
        .min_width(106)
        .min_height(30)
        .scrollable();

    let mut top_view = LinearLayout::vertical();
    top_view.add_child(TextView::new("Here below is a list of active update topics available for early adoption.\nSelect one or more topic to enroll in update testing, deselect to withdraw and rollback to stable packages."));
    top_view.add_child(DummyView {});
    if has_closed {
        top_view.add_child(TextView::new("Closed/graduated topics detected, ATM will refresh all packages affected by these topics with versions found in the stable repository."));
    }
    top_view.add_child(view.scroll_x(true));
    siv.add_layer(
        Dialog::around(top_view)
            .title("Topic Selection")
            .button("Exit", |siv| siv.quit())
            .button("Proceed", |siv| commit_changes(siv))
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn main() {
    let mut siv = cursive::default();
    fetch_manifest(&mut siv);
    siv.run();

    loop {
        let dump = siv.take_user_data::<(Vec<String>, cursive::Dump)>();
        if let Some((reinstall, dump)) = dump {
            drop(siv);
            println!("Refreshing APT databases ...");
            std::process::Command::new("apt")
                .arg("update")
                .status()
                .unwrap();
            if !reinstall.is_empty() {
                println!("Reverting packages to stable ...");
                std::process::Command::new("apt")
                    .arg("install")
                    .arg("-y")
                    .args(&reinstall)
                    .status()
                    .unwrap();
            }
            println!("Please upgrade your system: \n");
            std::process::Command::new("apt")
                .arg("full-upgrade")
                .status()
                .unwrap();
            // create a fresh Cursive instance and load previous state
            siv = cursive::default();
            siv.restore(dump);
            siv.run();
        } else {
            break;
        }
    }
}
