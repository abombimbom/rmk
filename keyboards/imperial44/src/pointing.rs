//! Layer-driven pointing mode selection for the Imperial44 trackball dongle.
//!
//! Reproduces the ZMK per-layer input processors with the user's sensitivity
//! tuning: the cursor moves at 3/4 of the sensor counts on both axes, the
//! scroll layer maps motion to wheel with an inverted Y at 1/64, and the
//! sniper layer slows the cursor to 1/4. Ergohaven pointing-mode user keys
//! override the layer-derived mode while held; both the Velvet (10/11/12) and
//! K:04 (28/29/30) user-key numbers are accepted.

use rmk::event::{ActionEvent, LayerChangeEvent, PointingProcessorEvent, publish_event_async};
use rmk::input_device::pointing::{
    CaretConfig, CursorConfig, PointingMode, PointingModeKeyState, ScrollConfig, SniperConfig,
};
use rmk::macros::processor;
use rmk::types::action::Action;

const TRACKBALL_DEVICE_ID: u8 = 0;
// Scroll and sniper layers of the shipped layout.
const LAYER_SCROLL: u8 = 3;
const LAYER_SNIPER: u8 = 4;
// Velvet-numbered mode keys, plus the K:04 aliases used by the Ergohaven
// custom-keycode block.
const USER_SNIPER: u8 = 10;
const USER_SCROLL: u8 = 11;
const USER_TEXT: u8 = 12;
const K04_USER_SNIPER: u8 = 28;
const K04_USER_SCROLL: u8 = 29;
const K04_USER_TEXT: u8 = 30;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PointingModeKind {
    Sniper,
    Scroll,
    Text,
}

#[processor(subscribe = [ActionEvent, LayerChangeEvent])]
pub struct LayerPointingMode {
    layer_mode: Option<PointingModeKind>,
    mode_key: PointingModeKeyState<PointingModeKind>,
    published_mode: PointingMode,
}

impl LayerPointingMode {
    pub fn new() -> Self {
        Self {
            layer_mode: None,
            mode_key: PointingModeKeyState::new(),
            published_mode: PointingMode::default(),
        }
    }

    async fn on_layer_change_event(&mut self, LayerChangeEvent(layer): LayerChangeEvent) {
        self.layer_mode = match layer {
            LAYER_SCROLL => Some(PointingModeKind::Scroll),
            LAYER_SNIPER => Some(PointingModeKind::Sniper),
            _ => None,
        };
        self.publish_current().await;
    }

    async fn on_action_event(&mut self, event: ActionEvent) {
        let Action::User(id) = event.action else {
            return;
        };
        let mode = match id {
            USER_SNIPER | K04_USER_SNIPER => Some(PointingModeKind::Sniper),
            USER_SCROLL | K04_USER_SCROLL => Some(PointingModeKind::Scroll),
            USER_TEXT | K04_USER_TEXT => Some(PointingModeKind::Text),
            _ => None,
        };
        let Some(mode) = mode else {
            return;
        };

        // Mode keys are momentary: active while held, no sticky latching.
        self.mode_key.handle(mode, event.keyboard_event.pressed, false);
        self.publish_current().await;
    }

    fn current_mode(&self) -> PointingMode {
        match self.mode_key.mode_override().or(self.layer_mode) {
            Some(PointingModeKind::Scroll) => PointingMode::Scroll(ScrollConfig {
                divisor_x: 64,
                divisor_y: 64,
                invert_y: true,
                ..Default::default()
            }),
            Some(PointingModeKind::Sniper) => PointingMode::Sniper(SniperConfig {
                divisor: 4,
                ..Default::default()
            }),
            Some(PointingModeKind::Text) => PointingMode::Caret(CaretConfig {
                threshold: 16,
                ..Default::default()
            }),
            None => PointingMode::Cursor(CursorConfig {
                multiplier_x: 3,
                divisor_x: 4,
                multiplier_y: 3,
                divisor_y: 4,
                ..Default::default()
            }),
        }
    }

    async fn publish_current(&mut self) {
        let mode = self.current_mode();
        if self.published_mode != mode {
            publish_event_async(PointingProcessorEvent {
                device_id: TRACKBALL_DEVICE_ID,
                mode,
            })
            .await;
            self.published_mode = mode;
        }
    }
}
