use std::env;
use std::process;

use crate::cache_pdb::get_rva;
use crate::explorer_modinfo::get_guid;

mod cache_pdb;
mod constants;
mod explorer_modinfo;
mod fetch_pdb;
mod inject;
mod parse_pdb;
mod startup;

fn prog() -> String {
    env::current_exe()
        .unwrap()
        .file_stem()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap()
}

fn help() {
    println!(include_str!("../help.txt"), env!("CARGO_PKG_VERSION"), prog())
}

fn rva() -> u32 {
    let guid = unsafe { get_guid() };
    let rva = get_rva(guid);
    println!("RVA is {rva:#x}");
    rva
}

fn do_inject() {
    unsafe {
        inject::inject(rva());
        inject::refresh();
    }
}

/// If not already elevated, re-launches with UAC and exits.
fn ensure_elevated() {
    if !startup::is_elevated() {
        println!("Requesting administrator privileges...");
        startup::relaunch_elevated();
        process::exit(0);
    }
}

/// If not running from the install directory, installs there, spawns the
/// installed copy, and exits — so the installed exe always does the injection.
fn maybe_install() {
    if !startup::is_running_from_install_dir() {
        println!("Installing...");
        startup::install();
        process::Command::new(startup::installed_exe())
            .spawn()
            .expect("failed to launch installed exe");
        process::exit(0);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("inject") => {
            ensure_elevated();
            maybe_install();
            do_inject();
        }
        Some("remove") => {
            ensure_elevated();
            startup::remove();
        }
        Some("help") => help(),
        Some("about") => println!(include_str!("../about.txt"), env!("CARGO_PKG_VERSION")),
        Some(err) => eprintln!(
            "Invalid argument `{err}`. Run `{} help` to see all commands.",
            prog()
        ),
    }
}
