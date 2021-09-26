use anyhow::Result;
use time::{format_description::FormatItem, macros::format_description};

pub mod cli;
pub mod tui;

const DATE_FORMAT: &[FormatItem] = format_description!("[year]-[month repr:numerical]-[day]");

#[inline]
pub(crate) fn format_timestamp(t: i64) -> Result<String> {
    Ok(time::OffsetDateTime::from_unix_timestamp(t)?.format(&DATE_FORMAT)?)
}
