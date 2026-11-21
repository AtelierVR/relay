//! Global tick of the relay: the periodic maintenance that belongs to the whole process rather than
//! to a single instance (an instance has its own tick, at its own rate).
//!
//! The loop runs at a fixed rate and drives the maintenance steps in order. New periodic work goes
//! here instead of spawning another timer.

use std::{sync::Arc, time::Duration};

use crate::handlers::context::AppState;

/// Period of the global maintenance tick.
const TICK: Duration = Duration::from_secs(1);

/// Run the maintenance of the relay until the process stops.
pub async fn run(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(TICK);

    loop {
        interval.tick().await;

        // A `Transfer` is only a request: clients that do not apply it are removed once the grace
        // period expires (`transfer_timeout`, 0 = never enforce).
        crate::transfer::enforce(&state);
    }
}
