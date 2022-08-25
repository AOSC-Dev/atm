use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use cursive::utils::Counter;
use cursive::{align::HAlign, traits::*, views::*};
use cursive::{views::Dialog, Cursive, CursiveRunnable};

use anyhow::Result;
use cursive_async_view::AsyncView;
use cursive_table_view::{TableView, TableViewItem};

use super::cli::privileged_write_source_list;
use super::format_timestamp;
use crate::network::{TopicManifest, TopicManifests};
use crate::pk::{self, PkPackage, PkTaskList};
use crate::{fl, network, pm};

type MarksMap = HashMap<String, bool>;

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
            TopicColumn::Date => format_timestamp(self.date).unwrap_or_else(|_| "?".to_string()),
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

struct TUIContext {
    async_runner: tokio::runtime::Runtime,
    dbus_connection: zbus::Connection,
    mirror_url: String,
    client: reqwest::Client,
}

fn create_async_runner() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

impl TUIContext {
    fn new() -> Result<Self> {
        let async_runner = create_async_runner()?;
        let dbus_connection = async_runner.block_on(pk::create_dbus_connection())?;
        let client = network::create_http_client()?;
        let mirror_url = async_runner.block_on(network::get_best_mirror_url(&client));

        Ok(TUIContext {
            async_runner,
            dbus_connection,
            mirror_url,
            client,
        })
    }
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

fn show_message(siv: &mut Cursive, msg: &str) {
    siv.add_layer(
        Dialog::around(TextView::new(msg))
            .title(fl!("message"))
            .button(fl!("ok"), |s| {
                s.pop_layer();
            })
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn show_finished(siv: &mut Cursive) {
    show_message(siv, &fl!("apt_finished"));
}

fn check_network(siv: &mut Cursive, marks: Rc<RefCell<MarksMap>>) {
    let ctx = siv.user_data::<TUIContext>().unwrap();
    if !ctx.async_runner.block_on(async {
        pk::is_metered_network(&ctx.dbus_connection)
            .await
            .unwrap_or(false)
    }) {
        return check_battery_level(siv, marks.clone());
    }
    siv.add_layer(
        Dialog::around(TextView::new(fl!("pk_metered_network")))
            .title(fl!("message"))
            .button(fl!("proceed"), move |s| {
                s.pop_layer();
                check_battery_level(s, marks.clone());
            })
            .button(fl!("cancel"), |s| {
                s.pop_layer();
            })
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn check_battery_level(siv: &mut Cursive, marks: Rc<RefCell<MarksMap>>) {
    let ctx = siv.user_data::<TUIContext>().unwrap();
    if !ctx.async_runner.block_on(async {
        pk::is_using_battery(&ctx.dbus_connection)
            .await
            .unwrap_or(false)
    }) {
        return check_changes(siv, marks.clone());
    }
    siv.add_layer(
        Dialog::around(TextView::new(fl!("pk_battery")))
            .title(fl!("message"))
            .button(fl!("proceed"), move |s| {
                s.pop_layer();
                check_changes(s, marks.clone());
            })
            .button(fl!("cancel"), |s| {
                s.pop_layer();
            })
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn show_tx_details(tasks: &PkTaskList) -> String {
    let mut listing = String::with_capacity(1024);
    listing += &fl!("tx_body");
    listing.push('\n');

    for t in tasks.hold.iter() {
        listing += &fl!("tx_hold", package = t.name);
        listing.push('\n');
    }
    for t in tasks.erase.iter() {
        listing += &fl!("tx_erase", package = t.name, version = t.version);
        listing.push('\n');
    }
    for t in tasks.downgrade.iter() {
        listing += &fl!("tx_downgrade", package = t.name, version = t.version);
        listing.push('\n');
    }
    for t in tasks.upgrade.iter() {
        listing += &fl!("tx_upgrade", package = t.name, version = t.version);
        listing.push('\n');
    }
    for t in tasks.install.iter() {
        listing += &fl!("tx_install", package = t.name, version = t.version);
        listing.push('\n');
    }

    listing
}

fn commit_transactions(siv: &mut Cursive, packages: &[PkPackage]) {
    if packages.is_empty() {
        return siv.cb_sink().send(Box::new(show_finished)).unwrap();
    }

    let cb_sink = siv.cb_sink().clone();
    let (progress_tx, progress_rx) = std::sync::mpsc::channel();
    let package_ids = packages
        .iter()
        .map(|m| m.package_id.to_string())
        .collect::<Vec<_>>();
    // UI components
    let item_counter = Counter::new(0);
    let overall_counter = Counter::new(0);
    let mut status_message = TextView::new(&fl!("exe-prepare"));
    let status_text = Arc::new(status_message.get_shared_content());
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(status_message)
                .child(ProgressBar::new().max(100).with_value(item_counter.clone()))
                .child(TextView::new(fl!("exe-overall")))
                .child(
                    ProgressBar::new()
                        .max(100)
                        .with_value(overall_counter.clone()),
                ),
        )
        .title(fl!("exe-title")),
    );
    siv.set_autorefresh(true);
    // actual execution
    let ctx = siv.user_data::<TUIContext>().unwrap();
    let dbus_connection = ctx.dbus_connection.clone();
    let transaction_thread = thread::spawn(move || -> Result<()> {
        let runner = create_async_runner()?;
        runner.block_on(async {
            let cookie = pk::take_wake_lock(&dbus_connection, &fl!("pk_inhibit_message"))
                .await
                .ok();
            let proxy = pk::connect_packagekit(&dbus_connection).await?;
            let transaction = pk::create_transaction(&proxy).await?;
            let package_ids = package_ids.iter().map(|m| m.as_str()).collect::<Vec<_>>();
            pk::execute_transaction(&transaction, &package_ids, progress_tx).await?;
            drop(cookie);

            Ok(())
        })
    });
    thread::spawn(move || {
        // tracker
        let mut tracker = crate::desktop::select_best_tracker();
        tracker.set_general_description(&fl!("info-title"));
        loop {
            if let Ok(progress) = progress_rx.recv() {
                match progress {
                    pk::PkDisplayProgress::Package(id, status, pct) => {
                        let name = pk::humanize_package_id(&id);
                        let status_message = match status {
                            pk::PK_STATUS_ENUM_DOWNLOAD => fl!("exe_download", name = name),
                            pk::PK_STATUS_ENUM_INSTALL => fl!("exe-install", name = name),
                            pk::PK_STATUS_ENUM_SETUP => fl!("exe-setup", name = name),
                            _ => fl!("exe-install", name = name),
                        };
                        item_counter.set(pct as usize);
                        tracker.set_message(&fl!("info-status"), &status_message);
                        status_text.set_content(status_message);
                    }
                    pk::PkDisplayProgress::Overall(pct) => {
                        if pct < 101 {
                            tracker.set_percent(pct);
                            overall_counter.set(pct as usize);
                        }
                    }
                    pk::PkDisplayProgress::Done => break,
                }
            } else {
                tracker.terminate("");
                break;
            }
        }

        let result = transaction_thread.join().unwrap();
        match result {
            Ok(()) => cb_sink
                .send(Box::new(|s| {
                    s.set_autorefresh(false);
                    s.pop_layer();
                    show_finished(s);
                }))
                .unwrap(),
            Err(e) => cb_sink
                .send(Box::new(move |s| {
                    s.set_autorefresh(false);
                    show_error(s, &fl!("pk_comm_error_mid_tx", error = e.to_string()))
                }))
                .unwrap(),
        }
    });
}

fn show_summary(tasks: PkTaskList, packages: Rc<Vec<PkPackage>>) -> Dialog {
    let mut summary = String::with_capacity(128);
    let details = show_tx_details(&tasks);
    let updates = tasks.upgrade.len();
    if updates > 0 {
        summary += &fl!("update_count", count = updates);
        summary.push('\n');
    }
    let erase = tasks.erase.len();
    if erase > 0 {
        summary += &fl!("erase_count", count = erase);
        summary.push('\n');
    }
    let install = tasks.install.len();
    if install > 0 {
        summary += &fl!("install_count", count = install);
        summary.push('\n');
    }
    let hold = tasks.hold.len();
    if hold > 0 {
        summary += &fl!("no_stable_version", count = hold);
        summary.push('\n');
    }

    if summary.is_empty() {
        return Dialog::around(TextView::new(fl!("nothing"))).button(fl!("ok"), |s| {
            s.pop_layer();
        });
    }

    Dialog::around(TextView::new(summary))
        .title(fl!("message"))
        .button(fl!("exit"), |s| {
            s.pop_layer();
        })
        .button(fl!("details"), move |s| {
            s.add_layer(
                Dialog::around(TextView::new(details.clone()).scrollable().scroll_y(true))
                    .title(fl!("tx_title"))
                    .button(fl!("ok"), |s| {
                        s.pop_layer();
                    })
                    .padding_lrtb(2, 2, 1, 1),
            );
        })
        .button(fl!("proceed"), move |s| {
            let p = packages.clone();
            s.pop_layer();
            commit_transactions(s, &p);
        })
        .padding_lrtb(2, 2, 1, 1)
}

fn calculate_changes(siv: &mut Cursive, reinstall: TopicManifests) {
    let ctx = siv.user_data::<TUIContext>().unwrap();
    let dbus_connection = ctx.dbus_connection.clone();
    let loader = AsyncView::new_with_bg_creator(
        siv,
        move || {
            let runner = create_async_runner().map_err(|e| e.to_string())?;
            let mut tracker = crate::desktop::select_best_tracker();
            tracker.set_general_description(&fl!("refresh-apt"));

            runner.block_on(async {
                let proxy = pk::connect_packagekit(&dbus_connection)
                    .await
                    .map_err(|e| fl!("pk_comm_error", error = e.to_string()))?;
                let (not_found, tasks) = pm::switch_topics(&proxy, &reinstall)
                    .await
                    .map_err(|e| fl!("pk_tx_error", error = e.to_string()))?;
                let tx = pk::create_transaction(&proxy)
                    .await
                    .map_err(|e| fl!("pk_comm_error", error = e.to_string()))?;
                let tasks = tasks.iter().map(|t| t.as_str()).collect::<Vec<_>>();
                let transaction = pk::get_transaction_steps(&tx, &tasks)
                    .await
                    .map_err(|e| fl!("pk_tx_error", error = e.to_string()))?;

                Ok((not_found, transaction))
            })
        },
        |(nf, tx)| {
            let tx = Rc::new(tx);
            let details = pk::get_task_details(&nf, &tx);
            match details {
                Ok(details) => show_summary(details, Rc::clone(&tx)),
                Err(e) => Dialog::around(TextView::new(fl!("pk_invalid_id", name = e.to_string())))
                    .title(fl!("error"))
                    .button(fl!("exit"), |s| s.quit())
                    .padding_lrtb(2, 2, 1, 1),
            }
        },
    );
    siv.add_layer(loader);
}

fn check_changes(siv: &mut Cursive, marks: Rc<RefCell<MarksMap>>) {
    let cb_sink = siv.cb_sink().clone();
    let ctx = siv.user_data::<TUIContext>().unwrap();
    let mirror_url = ctx.mirror_url.clone();

    siv.call_on_name(
        "topic",
        |v: &mut TableView<network::TopicManifest, TopicColumn>| {
            let items = v.borrow_items();
            let marks_ref = RefCell::borrow(&marks);
            let mut enabled = Vec::with_capacity(marks_ref.len());
            let mut reinstall = Vec::with_capacity(marks_ref.len());

            for item in items.iter() {
                if item.enabled {
                    enabled.push(item);
                    continue;
                }
                if let Some(enable) = marks_ref.get(&item.name) {
                    if !enable {
                        reinstall.push(item.clone());
                    }
                }
            }

            drop(marks_ref); // end the immutable borrow here so that we can mutate it later
            if let Err(e) = privileged_write_source_list(&enabled, &mirror_url) {
                let message = e.to_string();
                cb_sink
                    .send(Box::new(move |s| show_error(s, &message)))
                    .unwrap();
            } else {
                RefCell::borrow_mut(&marks).clear();
                cb_sink
                    .send(Box::new(|s| {
                        calculate_changes(s, reinstall);
                    }))
                    .unwrap();
            }
        },
    );
}

fn build_topic_list_view(siv: &mut Cursive, manifest: Vec<TopicManifest>) {
    let map = HashMap::<String, bool>::with_capacity(std::cmp::min(manifest.len(), 10));
    let marks = Rc::new(RefCell::new(map));
    let marks_table = Rc::clone(&marks);
    let has_closed = manifest.iter().any(|x| x.closed);

    let view = TableView::<network::TopicManifest, TopicColumn>::new()
        .column(TopicColumn::Enabled, "", |c| {
            c.align(HAlign::Center)
                .width(4)
                .ordering(std::cmp::Ordering::Greater)
        })
        .column(TopicColumn::Name, fl!("name"), |c| {
            c.ordering(std::cmp::Ordering::Greater)
        })
        .column(TopicColumn::Date, fl!("date"), |c| c.width(12))
        .column(TopicColumn::Description, fl!("description"), |c| c)
        .items(manifest)
        .on_submit(move |siv, _, index| {
            siv.call_on_name(
                "topic",
                |v: &mut TableView<network::TopicManifest, TopicColumn>| {
                    if let Some(item) = v.borrow_item_mut(index) {
                        item.enabled = !item.enabled;
                        // update tracking information
                        if RefCell::borrow(&marks_table).contains_key(&item.name) {
                            RefCell::borrow_mut(&marks_table).remove(&item.name);
                        } else {
                            RefCell::borrow_mut(&marks_table)
                                .insert(item.name.clone(), item.enabled);
                        }
                        v.needs_relayout();
                    }
                },
            );
        })
        .with_name("topic")
        .min_width(106)
        .min_height(25)
        .scrollable();

    let mut top_view = LinearLayout::vertical();
    top_view.add_child(TextView::new(fl!("topic-selection-description")));
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
            .button(fl!("proceed"), move |siv| check_network(siv, marks.clone()))
            .padding_lrtb(2, 2, 1, 1),
    );
}

fn fetch_manifest(siv: &mut Cursive) {
    let ctx = siv.user_data::<TUIContext>().unwrap();
    let fetch_result = ctx
        .async_runner
        .block_on(network::fetch_topics(&ctx.client, &ctx.mirror_url));
    let filtered_list = fetch_result
        .and_then(|result| network::filter_topics(result))
        .map(|topics| pm::get_display_listing(topics));
    match filtered_list {
        Ok(filtered_list) => build_topic_list_view(siv, filtered_list),
        Err(e) => show_error(siv, &fl!("error-fetch-manifest", error = e.to_string())),
    }
}

fn initialize_context(siv: &mut CursiveRunnable) {
    show_blocking_message(siv, &fl!("refresh-manifest"));
    let cb_sink = siv.cb_sink().clone();
    thread::spawn(move || {
        // time-consuming and blocking operations
        let ctx = TUIContext::new();
        match ctx {
            Ok(ctx) => {
                cb_sink
                    .send(Box::new(move |s| {
                        s.set_user_data(ctx);
                        fetch_manifest(s);
                    }))
                    .unwrap();
            }
            Err(e) => {
                cb_sink
                    .send(Box::new(move |s| {
                        s.pop_layer();
                        show_error(s, &fl!("error-initialize", error = e.to_string()));
                    }))
                    .unwrap();
            }
        }
    });
}

pub fn tui_main() {
    let mut siv = cursive::default();
    initialize_context(&mut siv);
    siv.run();
}
