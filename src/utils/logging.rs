//! Runtime-switchable log level.
//!
//! The tracing subscriber registers a [`reload::Layer`] as its first layer so the
//! active [`EnvFilter`] can be swapped at runtime — this is what the `debug`
//! command uses to turn debug logging on and off without a restart.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use tracing::{error, info, warn};
use tracing_subscriber::{reload, EnvFilter, Registry};

/// Reloadable filter layer — must be registered as the first layer of the subscriber stack.
pub type FilterLayer = reload::Layer<EnvFilter, Registry>;

/// Handle used to swap the active [`EnvFilter`] at runtime.
pub type FilterHandle = reload::Handle<EnvFilter, Registry>;

/// Handle to the active filter, stored once during subscriber setup.
static FILTER: OnceLock<FilterHandle> = OnceLock::new();

/// Mirrors the current filter state (`EnvFilter` cannot be introspected).
static DEBUG: AtomicBool = AtomicBool::new(false);

/// Build the reloadable filter layer from the configured debug flag.
/// Call once, during subscriber setup, and register the returned layer first.
pub fn build(debug: bool) -> FilterLayer {
    let (layer, handle) = reload::Layer::new(filter_for(debug));
    if FILTER.set(handle).is_err() {
        warn!("[Debug] Logger already initialized; the `debug` command may not work.");
    }
    DEBUG.store(debug, Ordering::Relaxed);
    layer
}

fn filter_for(debug: bool) -> EnvFilter {
    EnvFilter::new(if debug { "debug" } else { "info" })
}

/// Whether debug logging is currently enabled.
pub fn is_debug() -> bool {
    DEBUG.load(Ordering::Relaxed)
}

/// Enable or disable debug logging at runtime. Returns the state actually in effect.
pub fn set_debug(enabled: bool) -> bool {
    if is_debug() == enabled {
        return enabled;
    }

    let Some(handle) = FILTER.get() else {
        warn!(
            "[Debug] The log filter is not reloadable in this process; restart the relay to apply."
        );
        return is_debug();
    };

    // `modify` takes the reload lock; never log from inside the closure.
    match handle.modify(|filter| *filter = filter_for(enabled)) {
        Ok(()) => {
            DEBUG.store(enabled, Ordering::Relaxed);
            info!(
                "[Debug] Debug logging {}.",
                if enabled { "enabled" } else { "disabled" }
            );
            enabled
        }
        Err(e) => {
            error!("[Debug] Failed to update the log filter: {e}");
            is_debug()
        }
    }
}

/// Flip the current debug state. Returns the new state.
pub fn toggle_debug() -> bool {
    set_debug(!is_debug())
}
