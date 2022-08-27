mod frontend;
mod i18n;
mod network;
mod parser;
mod pk;
mod pm;
mod desktop;

use i18n::I18N_LOADER;

fn main() {
    let cli_result = frontend::cli::cli_main();
    if !cli_result {
        frontend::tui::tui_main();
    }
}
