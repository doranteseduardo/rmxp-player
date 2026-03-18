use crate::{push_script_label, RubyVm};
use anyhow::Result;
use once_cell::sync::OnceCell;

static PRIMITIVES: OnceCell<()> = OnceCell::new();

const PRIMITIVES_SRC: &str = include_str!("ruby/primitives.rb");

pub fn load(vm: &RubyVm) -> Result<()> {
    PRIMITIVES
        .get_or_try_init(|| {
            let _guard = push_script_label("rgss primitives");
            vm.eval(PRIMITIVES_SRC)
        })
        .map(|_| ())
}
