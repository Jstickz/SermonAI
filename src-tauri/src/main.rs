// Windows release builds must not open a console alongside the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sermonai_lib::run()
}
