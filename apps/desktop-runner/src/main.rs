use anyhow::Result;
use engine_core::{run, AppConfig};

fn main() -> Result<()> {
    eprintln!("[BUILD] desktop-runner build-id=WINDOWSKIN-XP-FIX-2026-04-29");
    let config = AppConfig::default();
    run(config)
}
