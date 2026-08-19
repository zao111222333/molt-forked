use chrono::prelude::*;
use std::env;
use std::fs::File;
use std::io::Write;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("compile_info.rs");
    let mut f = File::create(&dest_path).unwrap();

    let compile_time = Utc::now().format("%Y-%m-%d").to_string();
    let compile_time = compile_time.trim();

    let rustc_version = String::from_utf8(
        Command::new("rustc")
            .arg("--version")
            .output()
            .expect("Failed to execute rustc")
            .stdout,
    )
    .unwrap_or_else(|_| "unknown".to_string());
    let mut rustc_version = rustc_version.lines().next().unwrap_or("unknown").trim();
    if let (Some(left), Some(_)) = (
        rustc_version.chars().position(|c| c == '('),
        rustc_version.chars().position(|c| c == ')'),
    ) {
        rustc_version = rustc_version[..left].trim();
    }
    let gcc_version = String::from_utf8(
        Command::new("gcc")
            .arg("--version")
            .output()
            .expect("Failed to execute gcc")
            .stdout,
    )
    .unwrap_or_else(|_| "unknown".to_string());
    let mut gcc_version = gcc_version.lines().next().unwrap_or("unknown").trim();
    if let (Some(left), Some(right)) = (
        gcc_version.chars().position(|c| c == '('),
        gcc_version.chars().position(|c| c == ')'),
    ) {
        gcc_version = gcc_version[left..(right + 1)].trim();
    }
    // let clang_version = Command::new("clang")
    //   .arg("--version")
    //   .output()
    //   .expect("Failed to execute clang")
    //   .stdout;
    let crate_version =
        env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());

    let compile_info = format!(
        "pub const COMPILE_TIME: &str = \"{}\";\n\
         pub const RUSTC_VERSION: &str = \"{}\";\n\
         pub const CRATE_VERSION: &str = \"v{}\";\n\
         pub const GCC_VERSION: &str = \"{}\";\n",
        compile_time, rustc_version, crate_version, gcc_version,
    );

    f.write_all(compile_info.as_bytes()).unwrap();
}
