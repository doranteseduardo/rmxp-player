use rb_sys_build::rb_config;

fn main() {
    let config = rb_config();
    emit_dir("rubylibdir", &config);
    emit_dir("archdir", &config);
    emit_dir("sitearchdir", &config);
    emit_dir("sitelibdir", &config);
    emit_dir("vendorlibdir", &config);
    emit_dir("vendorarchdir", &config);
}

fn emit_dir(key: &str, config: &rb_sys_build::RbConfig) {
    if let Some(path) = config.get(key) {
        if !path.is_empty() {
            println!("cargo:rustc-env=RGSS_RUBY_CFG_{key}={path}");
        }
    }
}
