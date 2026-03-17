use anyhow::Result;
use engine_core::{run, AppConfig};
use tracing::info;

/// Entry point placeholder for future Android/iOS launchers.
pub fn launch_on_mobile() -> Result<()> {
    info!(target: "mobile", "launching placeholder mobile shell");
    let config = AppConfig::default();
    run(config)
}
