use const_gen::*;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::{env, fs};
use xz2::read::XzEncoder;

fn main() {
    // Test-only cache-busting version for the runtime encoder settings build.
    const FIRMWARE_VERSION: &str = "0.2.0";
    const FIRMWARE_VERSION_BCD: &str = "0x0200";

    let vial_path = configured_path("VIAL_JSON_PATH", "vial.json");
    let keyboard_path = configured_path("KEYBOARD_TOML_PATH", "keyboard.toml");

    println!("cargo:rerun-if-env-changed=VIAL_JSON_PATH");
    println!("cargo:rerun-if-env-changed=KEYBOARD_TOML_PATH");
    println!("cargo:rerun-if-changed={}", vial_path.display());
    println!("cargo:rerun-if-changed={}", keyboard_path.display());
    println!("cargo:rustc-env=RMK_FIRMWARE_VERSION={FIRMWARE_VERSION}");
    println!("cargo:rustc-env=RMK_FIRMWARE_VERSION_BCD={FIRMWARE_VERSION_BCD}");
    println!("cargo:rustc-env=RMK_VIAL_DEVICE_SETTINGS_FN=crate::encoder_device_settings::vial_device_settings");
    println!("cargo:rustc-env=RMK_BLE_HOST_POWER_CONFIG_FN=crate::ble_host_power::ble_host_power_config");

    generate_vial_config(&vial_path);

    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rustc-link-arg=--nmagic");
    println!("cargo:rustc-link-arg=-Tlink.x");
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-linker=flip-link");
}

fn configured_path(variable: &str, default: &str) -> PathBuf {
    env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

fn generate_vial_config(vial_path: &Path) {
    let out_file = Path::new(&env::var_os("OUT_DIR").unwrap()).join("config_generated.rs");

    let mut content = String::new();
    match File::open(vial_path) {
        Ok(mut file) => {
            file.read_to_string(&mut content)
                .unwrap_or_else(|e| panic!("Cannot read {}: {e}", vial_path.display()));
        }
        Err(e) => panic!("Cannot find {}: {e}", vial_path.display()),
    };

    let parsed = json::parse(&content).expect("Cannot parse vial.json");
    let vial_cfg = json::stringify(parsed.clone());
    let mut keyboard_def_compressed: Vec<u8> = Vec::new();
    XzEncoder::new(vial_cfg.as_bytes(), 6)
        .read_to_end(&mut keyboard_def_compressed)
        .unwrap();

    let product_id = parsed["productId"]
        .as_str()
        .and_then(|v| u16::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok())
        .unwrap_or_else(|| panic!("{} productId must be a hexadecimal string", vial_path.display()));
    let keyboard_id: Vec<u8> = match product_id {
        0x0044 => vec![0xB9, 0xBC, 0x09, 0xB2, 0x9D, 0x37, 0x4C, 0xEA],
        0x0440 => vec![b'I', b'4', b'4', b'T', b'B', b'L', 0x04, 0x40],
        _ => panic!("Unsupported Imperial44 productId: 0x{product_id:04X}"),
    };
    let const_declarations = [
        const_declaration!(pub VIAL_KEYBOARD_DEF = keyboard_def_compressed),
        const_declaration!(pub VIAL_KEYBOARD_ID = keyboard_id),
    ]
    .map(|s| "#[allow(clippy::redundant_static_lifetimes)]\n".to_owned() + s.as_str())
    .join("\n");
    fs::write(out_file, const_declarations).unwrap();
}
