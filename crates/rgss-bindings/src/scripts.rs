use crate::{push_script_label, RubyVm};
use anyhow::Result;
use once_cell::sync::OnceCell;

static PRIMITIVES: OnceCell<()> = OnceCell::new();

const PRIMITIVES_SRC: &str = include_str!("ruby/primitives.rb");
const CLASSIC_SRC: &str = include_str!("ruby/preload/classic.rb");
const MODULE_RPG1_SRC: &str = include_str!("ruby/preload/module_rpg1.rb");
const MKXP_WRAP_SRC: &str = include_str!("ruby/preload/mkxp_wrap.rb");
const WIN32_SRC: &str = include_str!("ruby/preload/win32.rb");

pub fn load(vm: &RubyVm) -> Result<()> {
    PRIMITIVES
        .get_or_try_init(|| {
            // 1. Core RGSS runtime (Fiber loop, Hangup, Reset, data I/O helpers).
            {
                let _guard = push_script_label("rgss primitives");
                vm.eval_preload(PRIMITIVES_SRC, "rgss primitives")?;
            }
            // 2. Ruby 1.x -> 3.x compatibility aliases (TRUE/FALSE/NIL, Hash#index, etc.).
            vm.eval_preload(CLASSIC_SRC, "classic compat")?;
            // 3. RPG module — required by every vanilla RMXP game before user scripts run.
            vm.eval_preload(MODULE_RPG1_SRC, "module RPG")?;
            // 4. MKXP compatibility aliases.
            vm.eval_preload(MKXP_WRAP_SRC, "mkxp compat")?;
            // 5. Win32API shim — routes Win32 calls to native equivalents.
            vm.eval_preload(WIN32_SRC, "win32 compat")?;
            Ok(())
        })
        .map(|_| ())
}
