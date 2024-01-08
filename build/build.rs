extern crate autocfg;

use autocfg::emit;
use rustc_version::{version, version_meta, Channel, Result, Version};

fn main() -> Result<()> {
    // Set cfg flags depending on release channel
    match version_meta()?.channel {
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

    // When the rustc version is >= 1.51.0 and < 1.75.0, we can use the
    let version = version()?;
    if version >= Version::parse("1.51.0")? && version < Version::parse("1.75.0")? {
        println!("cargo:rustc-cfg=SPLIT_INCLUSIVE_COMPATIBLE");
    }

    // (optional) We don't need to rerun for anything external.
    // In order to see the compilation parameters at `cargo check --verbose` time, keep it.
    autocfg::rerun_path("build/build.rs");
    Ok(())
}
