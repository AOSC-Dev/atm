use std::time::Duration;
use std::{cmp::Ordering, collections::HashSet};
use std::{path::Path, sync::mpsc};

use chrono::prelude::*;
use cursive::{align::HAlign, traits::*, views::DummyView, views::LinearLayout};
use cursive::{views::Dialog, views::TextView, Cursive, CursiveRunner};
use cursive_async_view::AsyncView;
use cursive_table_view::{TableView, TableViewItem};

mod i18n;
mod network;
mod parser;
mod pk;
mod pm;

use i18n::I18N_LOADER;
use pk::PkPackage;

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
                    name.push(' ');
                    name.push_str(&fl!("closed"));
                }
                name
            }
            TopicColumn::Date => Utc.timestamp(self.date, 0).format("%Y-%m-%d").to_string(),
            TopicColumn::Description => self.description.clone().unwrap_or_default(),
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
            .title(fl!("message"))
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn show_error(siv: &mut Cursive, msg: &str) {
    siv.add_layer(
        Dialog::around(TextView::new(msg))
            .title(fl!("error"))
            .button(fl!("exit"), |s| s.quit())
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn show_tx_details(siv: &mut Cursive, meta: &[PkPackage]) {
    siv.add_layer(
        Dialog::around(
            TextView::new(unwrap_or_show_error!(siv, { pk::get_task_details(meta) }))
                .scrollable()
                .scroll_y(true),
        )
        .title(fl!("tx_title"))
        .button(fl!("ok"), |s| {
            s.pop_layer();
        })
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
            tx.send((pm::write_source_list(&enabled), items.clone()))
                .ok();
        },
    );
    let (result, items) = unwrap_or_show_error!(siv, { rx.recv_timeout(Duration::from_secs(10)) });
    unwrap_or_show_error!(siv, { result });
    siv.set_user_data(items);
    let loader = AsyncView::new_with_bg_creator(
        siv,
        move || {
            let conn = pk::create_dbus_connection()
                .map_err(|e| fl!("pk_dbus_error", error = e.to_string()))?;
            let proxy = pk::connect_packagekit(&conn)
                .map_err(|e| fl!("pk_comm_error", error = e.to_string()))?;
            let (not_found, tasks) =
                pm::switch_topics(&proxy, &reinstall).map_err(|e| e.to_string())?;
            let proxy = pk::create_transaction(&proxy)
                .map_err(|e| fl!("pk_comm_error", error = e.to_string()))?;
            let tasks = tasks.iter().map(|t| t.as_str()).collect::<Vec<_>>();
            let transaction =
                pk::get_transaction_steps(&proxy, &tasks).map_err(|e| e.to_string())?;

            Ok(transaction)
        },
        |t| {
            let summary = pk::get_task_summary(&t);
            Dialog::around(TextView::new(summary))
                .title(fl!("message"))
                .button(fl!("exit"), |s| {
                    s.pop_layer();
                })
                .button(fl!("details"), move |s| show_tx_details(s, &t))
                .button(fl!("proceed"), move |s| {
                    s.pop_layer();
                    todo!()
                })
                .padding_lrtb(2, 2, 1, 1)
        },
    );
    siv.add_layer(loader);
}

fn fetch_manifest(siv: &mut CursiveRunner<&mut Cursive>) {
    show_blocking_message(siv, &fl!("refresh_manifest"));
    siv.refresh();
    let manifest = unwrap_or_show_error!(siv, {
        network::fetch_topics(&format!(
            "{}{}",
            pm::MIRROR_URL.to_string(),
            "debs/manifest/topics.json"
        ))
    });
    let filtered = unwrap_or_show_error!(siv, {
        let topics = network::filter_topics(manifest);
        match topics {
            Ok(topics) => Ok(pm::get_display_listing(topics)),
            Err(e) => Err(e),
        }
    });
    let has_closed = filtered.iter().any(|x| x.closed);
    siv.set_user_data(filtered.clone());
    siv.refresh();
    let view = TableView::<network::TopicManifest, TopicColumn>::new()
        .column(TopicColumn::Enabled, "", |c| {
            c.align(HAlign::Center).width(4)
        })
        .column(TopicColumn::Name, fl!("name"), |c| {
            c.ordering(Ordering::Greater)
        })
        .column(TopicColumn::Date, fl!("date"), |c| c)
        .column(TopicColumn::Description, fl!("description"), |c| c)
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
    top_view.add_child(TextView::new(fl!("topic_selection_description")));
    top_view.add_child(DummyView {});
    if has_closed {
        top_view.add_child(TextView::new(fl!("topic_selection_closed_topic_warning")));
    }
    top_view.add_child(view.scroll_x(true));
    siv.pop_layer();
    siv.add_layer(
        Dialog::around(top_view)
            .title(fl!("topic_selection"))
            .button(fl!("exit"), |siv| siv.quit())
            .button(fl!("proceed"), |siv| commit_changes(siv))
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn main() {
    let mut siv = cursive::default();
    fetch_manifest(&mut siv.runner());
    siv.run();
}
