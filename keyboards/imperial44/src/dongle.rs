#![no_main]
#![no_std]

//! Imperial44 trackball dongle — USB HID central with PMW3610 and six
//! trackball buttons, no screen. The two Imperial44 halves connect as BLE
//! peripherals. The sensor itself is wired from `keyboard_trackball.toml`.
//!
//! Build: `KEYBOARD_TOML_PATH=keyboard_trackball.toml VIAL_JSON_PATH=vial_trackball.json cargo build --release --bin dongle`

#[path = "../../common/ble_host_power.rs"]
mod ble_host_power;
#[path = "../../common/default_layer_names.rs"]
mod default_layer_names;
#[path = "../../common/encoder_device_settings.rs"]
mod encoder_device_settings;
#[path = "../../common/layer_names.rs"]
mod layer_names;
mod pointing;

const DEFAULT_LAYER_NAMES: [&str; 16] = [
    "Base", "Ru", "Symbols", "Scroll", "Sniper", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
];
const ENCODER_COUNT: usize = 2;
const ENCODER_SETTING_KEYS: &[u16] = &[
    200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 340, 341, 342, 343,
];

use rmk::macros::rmk_central;

#[rmk_central]
mod keyboard_central {
    #[register_processor(event)]
    fn encoder_settings_sync() -> crate::encoder_device_settings::EncoderSettingsSync {
        crate::encoder_device_settings::EncoderSettingsSync::new()
    }

    #[register_processor(event)]
    fn pointing_mode() -> crate::pointing::LayerPointingMode {
        crate::pointing::LayerPointingMode::new()
    }

    #[register_processor(poll)]
    fn ergohaven_user_keys() -> ::rmk::processor::builtin::ergohaven::ErgohavenUserKeys {
        ::rmk::processor::builtin::ergohaven::ErgohavenUserKeys::new()
    }
}
