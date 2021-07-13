mod frontend;
mod i18n;
mod network;
mod parser;
mod pk;
mod pm;

use i18n::I18N_LOADER;

fn main() {
    frontend::tui::tui_main();
}
