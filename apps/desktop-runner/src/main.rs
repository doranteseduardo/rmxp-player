use anyhow::Result;
use engine_core::{run, AppConfig};

fn main() -> Result<()> {
    let config = AppConfig::default();
    run(config)
}
