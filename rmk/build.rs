#[path = "./build_common.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    // Set the compilation target configuration
    let mut cfgs = common::CfgSet::new();
    common::set_target_cfgs(&mut cfgs);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=KEYBOARD_TOML_PATH");

    // Compute build hash and write to constants.rs
    let build_hash = compute_build_hash();
    let mut constants = format!(
        "#[allow(clippy::redundant_static_lifetimes)]\npub(crate) const BUILD_HASH: u32 = {build_hash:#010x};\n"
    );
    constants.push_str(&language_layer_table());

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("constants.rs");
    fs::write(&dest_path, constants).expect("Failed to write constants.rs file");
}

/// Emit the `(layer, macro)` table from `[behavior.language_layer]` of
/// keyboard.toml. `DF(n)` on a table layer deactivates all other layers and
/// runs the bound macro; layers outside the table keep the plain default-layer
/// switch. Empty when the section is absent or KEYBOARD_TOML_PATH is not set
/// (host builds of the rmk crate).
fn language_layer_table() -> String {
    const EMPTY: &str = "pub const LANGUAGE_LAYER_TABLE: [(u8, u8); 0] = [];\n";
    let Some(path) = env::var_os("KEYBOARD_TOML_PATH") else {
        return EMPTY.to_string();
    };
    let path = PathBuf::from(path);
    println!("cargo:rerun-if-changed={}", path.display());

    let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("Cannot read {}: {e}", path.display()));
    let doc: toml::Table = toml::from_str(&source).unwrap_or_else(|e| panic!("Cannot parse {}: {e}", path.display()));
    let Some(section) = doc
        .get("behavior")
        .and_then(|behavior| behavior.get("language_layer"))
        .and_then(|v| v.as_table())
    else {
        return EMPTY.to_string();
    };

    let mut table: Vec<(i64, i64)> = section
        .iter()
        .map(|(name, value)| {
            let layer = language_layer_num(&path, name, value, "layer");
            let macro_idx = language_layer_num(&path, name, value, "macro");
            (layer, macro_idx)
        })
        .collect();
    // Sort by (layer, macro) so the table order is independent of key order
    table.sort();
    if table.len() != section.len() {
        panic!("{}: duplicate layer in [behavior.language_layer]", path.display());
    }
    let entries = table
        .iter()
        .map(|(l, m)| format!("({l}, {m})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "pub const LANGUAGE_LAYER_TABLE: [(u8, u8); {}] = [{}];\n",
        table.len(),
        entries
    )
}

/// Read an integer field of a `[behavior.language_layer]` entry, range-checked for u8
fn language_layer_num(path: &Path, name: &str, entry: &toml::Value, field: &str) -> i64 {
    let value = entry.get(field).and_then(toml::Value::as_integer).unwrap_or_else(|| {
        panic!(
            "{}: behavior.language_layer.{name}: `{field}` must be an integer",
            path.display()
        )
    });
    if !(0..=u8::MAX as i64).contains(&value) {
        panic!(
            "{}: behavior.language_layer.{name}: `{field}` must fit in u8, got {value}",
            path.display()
        );
    }
    value
}
fn compute_build_hash() -> u32 {
    // Get the short hash of the latest Git commit. Use "unknown" if it fails
    let commit_id = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get and format current local time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    // Combine data and compute CRC32
    let combined = format!("{commit_id}_{now}");
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(combined.as_bytes());
    hasher.finalize()
}
