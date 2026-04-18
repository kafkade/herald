use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, AudioContextState, BiquadFilterType};

/// Manages Web Audio API sound effects for the split-flap tile flip animation.
///
/// Sound is generated programmatically (no audio files): a short white-noise
/// burst shaped by a band-pass filter and gain envelope to simulate the
/// mechanical "clack" of a split-flap tile.
#[derive(Clone)]
pub struct SoundEngine {
    context: Rc<RefCell<Option<AudioContext>>>,
    noise_buffer: Rc<RefCell<Option<web_sys::AudioBuffer>>>,
    /// User preference — persisted in localStorage.
    pub enabled: RwSignal<bool>,
    /// Whether the browser supports Web Audio.
    pub supported: bool,
}

const STORAGE_KEY: &str = "herald_sound_enabled";
const CLACK_DURATION_SECS: f32 = 0.03;
const FILTER_FREQUENCY: f32 = 1200.0;
const FILTER_Q: f32 = 2.0;
const ATTACK_SECS: f64 = 0.001;
const DECAY_SECS: f64 = 0.03;
const GAIN_PEAK: f32 = 0.3;
const CASCADE_STAGGER_SECS: f64 = 0.02;

impl SoundEngine {
    pub fn new() -> Self {
        let supported = web_sys::window()
            .map(|w| js_sys::Reflect::has(&w, &"AudioContext".into()).unwrap_or(false))
            .unwrap_or(false);

        // Read persisted preference (default: off)
        let stored = load_preference();

        // If prefers-reduced-motion is active, default to off regardless of stored preference
        let reduced_motion = prefers_reduced_motion();
        let initial_enabled = if reduced_motion { false } else { stored };

        Self {
            context: Rc::new(RefCell::new(None)),
            noise_buffer: Rc::new(RefCell::new(None)),
            enabled: RwSignal::new(initial_enabled),
            supported,
        }
    }

    /// Toggle sound on/off. Creates the AudioContext on first enable (requires user gesture).
    pub fn toggle(&self) {
        let new_state = !self.enabled.get_untracked();
        self.enabled.set(new_state);
        save_preference(new_state);

        if new_state {
            self.ensure_context();
        } else {
            self.suspend_context();
        }
    }

    /// Play staggered clack sounds for the given changed columns.
    pub fn play_cascade(&self, changed_cols: &[usize]) {
        if !self.enabled.get_untracked() || changed_cols.is_empty() {
            return;
        }
        if !self.ensure_context() {
            return;
        }

        let ctx = self.context.borrow();
        let ctx = match ctx.as_ref() {
            Some(c) if c.state() == AudioContextState::Running => c,
            _ => return,
        };

        let noise = self.noise_buffer.borrow();
        let noise = match noise.as_ref() {
            Some(b) => b,
            None => return,
        };

        let now = ctx.current_time();
        for &col in changed_cols {
            let time = now + (col as f64) * CASCADE_STAGGER_SECS;
            if let Err(e) = play_single_clack(ctx, noise, time) {
                log::warn!("Failed to play clack: {:?}", e);
                break;
            }
        }
    }

    /// Initialize the AudioContext if not already created.
    /// Returns true if context is ready to use.
    fn ensure_context(&self) -> bool {
        let mut ctx_ref = self.context.borrow_mut();
        if ctx_ref.is_some() {
            // Resume if suspended
            if let Some(ctx) = ctx_ref.as_ref() {
                if ctx.state() == AudioContextState::Suspended {
                    let _ = ctx.resume();
                }
            }
            return true;
        }

        match AudioContext::new() {
            Ok(ctx) => {
                // Create the reusable noise buffer
                match create_noise_buffer(&ctx) {
                    Ok(buf) => {
                        *self.noise_buffer.borrow_mut() = Some(buf);
                    }
                    Err(e) => {
                        log::error!("Failed to create noise buffer: {:?}", e);
                        return false;
                    }
                }
                *ctx_ref = Some(ctx);
                true
            }
            Err(e) => {
                log::error!("Failed to create AudioContext: {:?}", e);
                false
            }
        }
    }

    /// Suspend the AudioContext to save CPU when muted.
    fn suspend_context(&self) {
        let ctx = self.context.borrow();
        if let Some(ctx) = ctx.as_ref() {
            let _ = ctx.suspend();
        }
    }
}

/// Create a short white-noise buffer for the clack sound.
fn create_noise_buffer(ctx: &AudioContext) -> Result<web_sys::AudioBuffer, wasm_bindgen::JsValue> {
    let sample_rate = ctx.sample_rate();
    let length = (sample_rate * CLACK_DURATION_SECS) as u32;
    let buffer = ctx.create_buffer(1, length, sample_rate)?;

    let data: Vec<f32> = (0..length)
        .map(|_| js_sys::Math::random() as f32 * 2.0 - 1.0)
        .collect();
    buffer.copy_to_channel(&data, 0)?;

    Ok(buffer)
}

/// Play a single "clack" sound at the specified time.
fn play_single_clack(
    ctx: &AudioContext,
    noise: &web_sys::AudioBuffer,
    time: f64,
) -> Result<(), wasm_bindgen::JsValue> {
    let source = ctx.create_buffer_source()?;
    source.set_buffer(Some(noise));

    // Band-pass filter to shape the noise into a mechanical click
    let filter = ctx.create_biquad_filter()?;
    filter.set_type(BiquadFilterType::Bandpass);
    filter.frequency().set_value(FILTER_FREQUENCY);
    filter.q().set_value(FILTER_Q);

    // Gain envelope: quick attack, fast exponential decay
    let gain = ctx.create_gain()?;
    gain.gain().set_value_at_time(0.0, time)?;
    gain.gain()
        .linear_ramp_to_value_at_time(GAIN_PEAK, time + ATTACK_SECS)?;
    gain.gain()
        .exponential_ramp_to_value_at_time(0.001, time + ATTACK_SECS + DECAY_SECS)?;

    // Connect the audio graph: source → filter → gain → destination
    source
        .dyn_ref::<web_sys::AudioNode>()
        .unwrap()
        .connect_with_audio_node(filter.dyn_ref::<web_sys::AudioNode>().unwrap())?;
    filter
        .dyn_ref::<web_sys::AudioNode>()
        .unwrap()
        .connect_with_audio_node(gain.dyn_ref::<web_sys::AudioNode>().unwrap())?;
    gain.dyn_ref::<web_sys::AudioNode>()
        .unwrap()
        .connect_with_audio_node(ctx.destination().dyn_ref::<web_sys::AudioNode>().unwrap())?;

    source
        .dyn_ref::<web_sys::AudioScheduledSourceNode>()
        .unwrap()
        .start_with_when(time)?;

    Ok(())
}

/// Check if the user prefers reduced motion.
fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .map(|mq| mq.matches())
        .unwrap_or(false)
}

/// Load sound preference from localStorage.
fn load_preference() -> bool {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .map(|v| v == "true")
        .unwrap_or(false)
}

/// Save sound preference to localStorage.
fn save_preference(enabled: bool) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(STORAGE_KEY, if enabled { "true" } else { "false" });
    }
}
