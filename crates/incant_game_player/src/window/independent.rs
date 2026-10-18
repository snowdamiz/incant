//! Independent deadlines never use CADisplayLink's future target time.
use std::time::Instant;

pub(super) fn ready(now: Instant, simulation: Instant, display: bool) -> bool {
    display || now >= simulation
}
#[cfg(any(all(feature = "native-display-link", target_os = "macos"), test))]
pub(super) fn wait_until(simulation: Instant, display_health: Instant, retry: Instant) -> Instant {
    simulation.max(retry).min(display_health)
}

#[cfg(test)]
mod tests;
