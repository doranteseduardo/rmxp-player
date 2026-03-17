use crate::RubyVm;
use anyhow::Result;
use once_cell::sync::OnceCell;

static PRIMITIVES: OnceCell<()> = OnceCell::new();

const PRIMITIVES_SRC: &str = include_str!("ruby/primitives.rb");

pub fn load(vm: &RubyVm) -> Result<()> {
    PRIMITIVES
        .get_or_try_init(|| vm.eval(PRIMITIVES_SRC))
        .map(|_| ())
}
