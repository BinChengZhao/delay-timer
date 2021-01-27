extern crate autocfg;

use autocfg::emit;
use rustc_version::{version, version_meta, Channel, Version};

fn main() {
    // Set cfg flags depending on release channel
    match version_meta().unwrap().channel {
        Channel::Stable => {
            println!("cargo:rustc-cfg=RUSTC_IS_STABLE");
        }
        Channel::Beta => {
            println!("cargo:rustc-cfg=RUSTC_IS_BETA");
        }
        Channel::Nightly => {
            emit("nightly");
            println!("cargo:rustc-cfg=RUSTC_IS_NIGHTLY");
        }
        Channel::Dev => {
            println!("cargo:rustc-cfg=RUSTC_IS_DEV");
        }
    }

    // Check for a minimum version
    if version().unwrap() < Version::parse("1.5.1").unwrap() {
        println!("cargo:rustc-cfg=SPLIT_INCLUSIVE_COMPATIBLE");
    }

    // (optional) We don't need to rerun for anything external.
    // In order to see the compilation parameters at `cargo check --verbose` time, keep it.
    autocfg::rerun_path("build.rs");
}
