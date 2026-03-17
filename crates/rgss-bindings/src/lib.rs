//! Placeholder crate for the RGSS <-> Rust bridge.

use anyhow::Result;
use tracing::info;

/// Represents the embedded Ruby VM state.
pub struct RubyVm {
    booted: bool,
}

impl RubyVm {
    pub fn new() -> Self {
        Self { booted: false }
    }

    pub fn boot(&mut self) -> Result<()> {
        // In the future this will set up MRI, load scripts, and register native classes.
        self.booted = true;
        info!(target: "rgss", "Ruby VM boot placeholder");
        Ok(())
    }

    pub fn is_booted(&self) -> bool {
        self.booted
    }
}
