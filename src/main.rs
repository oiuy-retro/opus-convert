#![windows_subsystem = "windows"]

mod aero;
mod bitrate;
mod config;
mod convert;
mod cover;
mod genre;
mod model;
mod names;
mod net;
mod paths;
mod probe;
mod readout;
mod run;
mod scan;
mod store;
mod tools;
mod ui;

fn main() {
    std::process::exit(ui::run());
}
