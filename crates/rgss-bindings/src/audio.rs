use crate::fs;
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use rb_sys::{rb_define_module, rb_num2long, rb_string_value_cstr, VALUE};
use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::RwLock,
};
use tracing::warn;

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
}

const AUDIO_MODULE_NAME: &[u8] = b"Audio\0";

static AUDIO_HOOKS: Lazy<RwLock<AudioHooks>> = Lazy::new(|| RwLock::new(AudioHooks::default()));
static CURRENT_BGM: Lazy<RwLock<Option<BgmCommand>>> = Lazy::new(|| RwLock::new(None));
static MEMORIZED_BGM: Lazy<RwLock<Option<BgmCommand>>> = Lazy::new(|| RwLock::new(None));
static CURRENT_BGS: Lazy<RwLock<Option<BgsCommand>>> = Lazy::new(|| RwLock::new(None));
static MEMORIZED_BGS: Lazy<RwLock<Option<BgsCommand>>> = Lazy::new(|| RwLock::new(None));

pub fn init() -> Result<()> {
    unsafe {
        let module = rb_define_module(c_name(AUDIO_MODULE_NAME));
        if module == 0 {
            return Err(anyhow!("failed to define Audio module"));
        }
        define_method(module, b"bgm_play\0", audio_bgm_play, -1);
        define_method(module, b"bgm_stop\0", audio_bgm_stop, -1);
        define_method(module, b"bgm_fade\0", audio_bgm_fade, -1);
        define_method(module, b"bgm_memorize\0", audio_bgm_memorize, -1);
        define_method(module, b"bgm_restore\0", audio_bgm_restore, -1);

        define_method(module, b"bgs_play\0", audio_bgs_play, -1);
        define_method(module, b"bgs_stop\0", audio_bgs_stop, -1);
        define_method(module, b"bgs_fade\0", audio_bgs_fade, -1);
        define_method(module, b"bgs_memorize\0", audio_bgs_memorize, -1);
        define_method(module, b"bgs_restore\0", audio_bgs_restore, -1);

        define_method(module, b"me_play\0", audio_me_play, -1);
        define_method(module, b"me_stop\0", audio_me_stop, -1);
        define_method(module, b"me_fade\0", audio_me_fade, -1);

        define_method(module, b"se_play\0", audio_se_play, -1);
        define_method(module, b"se_stop\0", audio_se_stop, -1);
        define_method(module, b"se_fade\0", audio_se_fade, 1);
    }
    Ok(())
}

pub fn install_audio_hooks(hooks: AudioHooks) {
    if let Ok(mut guard) = AUDIO_HOOKS.write() {
        *guard = hooks;
    }
}

fn hooks<F>(f: F)
where
    F: FnOnce(&AudioHooks),
{
    if let Ok(guard) = AUDIO_HOOKS.read() {
        f(&*guard);
    }
}

#[derive(Clone, Debug)]
pub struct BgmCommand {
    pub path: PathBuf,
    pub volume: i32,
    pub pitch: i32,
    pub position: u32,
}

#[derive(Clone, Debug)]
pub struct BgsCommand {
    pub path: PathBuf,
    pub volume: i32,
    pub pitch: i32,
    pub position: u32,
}

#[derive(Clone, Debug)]
pub struct MeCommand {
    pub path: PathBuf,
    pub volume: i32,
    pub pitch: i32,
}

#[derive(Clone, Debug)]
pub struct SeCommand {
    pub path: PathBuf,
    pub volume: i32,
    pub pitch: i32,
}

pub struct AudioHooks {
    pub bgm_play: Hook<BgmCommand>,
    pub bgm_stop: Hook<()>,
    pub bgm_fade: Hook<u32>,
    pub bgs_play: Hook<BgsCommand>,
    pub bgs_stop: Hook<()>,
    pub bgs_fade: Hook<u32>,
    pub me_play: Hook<MeCommand>,
    pub me_stop: Hook<()>,
    pub me_fade: Hook<u32>,
    pub se_play: Hook<SeCommand>,
    pub se_stop: Hook<()>,
    pub se_fade: Hook<u32>,
}

type Hook<T> = Box<dyn Fn(T) + Send + Sync + 'static>;

impl Default for AudioHooks {
    fn default() -> Self {
        Self {
            bgm_play: Box::new(|cmd: BgmCommand| warn_not_implemented("bgm_play", Some(cmd.path))),
            bgm_stop: Box::new(|_| warn_not_implemented("bgm_stop", None)),
            bgm_fade: Box::new(|_| warn_not_implemented("bgm_fade", None)),
            bgs_play: Box::new(|cmd: BgsCommand| warn_not_implemented("bgs_play", Some(cmd.path))),
            bgs_stop: Box::new(|_| warn_not_implemented("bgs_stop", None)),
            bgs_fade: Box::new(|_| warn_not_implemented("bgs_fade", None)),
            me_play: Box::new(|cmd: MeCommand| warn_not_implemented("me_play", Some(cmd.path))),
            me_stop: Box::new(|_| warn_not_implemented("me_stop", None)),
            me_fade: Box::new(|_| warn_not_implemented("me_fade", None)),
            se_play: Box::new(|cmd: SeCommand| warn_not_implemented("se_play", Some(cmd.path))),
            se_stop: Box::new(|_| warn_not_implemented("se_stop", None)),
            se_fade: Box::new(|_| warn_not_implemented("se_fade", None)),
        }
    }
}

fn warn_not_implemented(label: &str, path: Option<PathBuf>) {
    match path {
        Some(p) => warn!(
            target: "audio",
            method = %label,
            file = %p.display(),
            "Audio backend not installed"
        ),
        None => warn!(
            target: "audio",
            method = %label,
            "Audio backend not installed"
        ),
    }
}

unsafe extern "C" fn audio_bgm_play(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Some(cmd) = parse_bgm_command(argc, argv) {
        set_current_bgm(Some(cmd.clone()));
        hooks(|hook| (hook.bgm_play)(cmd));
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgm_stop(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    set_current_bgm(None);
    hooks(|hook| (hook.bgm_stop)(()));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgm_fade(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let duration = parse_duration(argc, argv).unwrap_or(0);
    hooks(|hook| (hook.bgm_fade)(duration));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgm_memorize(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Ok(current) = CURRENT_BGM.read() {
        if let Some(value) = current.clone() {
            let _ = MEMORIZED_BGM.write().map(|mut slot| *slot = Some(value));
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgm_restore(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Ok(memory) = MEMORIZED_BGM.read() {
        if let Some(value) = memory.clone() {
            set_current_bgm(Some(value.clone()));
            hooks(|hook| (hook.bgm_play)(value));
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgs_play(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Some(cmd) = parse_bgs_command(argc, argv) {
        set_current_bgs(Some(cmd.clone()));
        hooks(|hook| (hook.bgs_play)(cmd));
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgs_stop(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    set_current_bgs(None);
    hooks(|hook| (hook.bgs_stop)(()));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgs_fade(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let duration = parse_duration(argc, argv).unwrap_or(0);
    hooks(|hook| (hook.bgs_fade)(duration));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgs_memorize(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Ok(current) = CURRENT_BGS.read() {
        if let Some(value) = current.clone() {
            let _ = MEMORIZED_BGS.write().map(|mut slot| *slot = Some(value));
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_bgs_restore(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Ok(memory) = MEMORIZED_BGS.read() {
        if let Some(value) = memory.clone() {
            set_current_bgs(Some(value.clone()));
            hooks(|hook| (hook.bgs_play)(value));
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_me_play(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Some(cmd) = parse_me_command(argc, argv) {
        hooks(|hook| (hook.me_play)(cmd));
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_me_stop(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    hooks(|hook| (hook.me_stop)(()));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_me_fade(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let duration = parse_duration(argc, argv).unwrap_or(0);
    hooks(|hook| (hook.me_fade)(duration));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_se_play(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Some(cmd) = parse_se_command(argc, argv) {
        hooks(|hook| (hook.se_play)(cmd));
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_se_stop(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    hooks(|hook| (hook.se_stop)(()));
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn audio_se_fade(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let duration = parse_duration(argc, argv).unwrap_or(0);
    hooks(|hook| (hook.se_fade)(duration));
    rb_sys::Qnil as VALUE
}

fn set_current_bgm(value: Option<BgmCommand>) {
    if let Ok(mut slot) = CURRENT_BGM.write() {
        *slot = value;
    }
}

fn set_current_bgs(value: Option<BgsCommand>) {
    if let Ok(mut slot) = CURRENT_BGS.write() {
        *slot = value;
    }
}

unsafe fn parse_bgm_command(argc: c_int, argv: *const VALUE) -> Option<BgmCommand> {
    let args = slice_args(argc, argv);
    let path = parse_audio_path(args.get(0)?)?;
    let volume = parse_volume(args.get(1));
    let pitch = parse_pitch(args.get(2));
    let position = parse_position(args.get(3));
    Some(BgmCommand {
        path,
        volume,
        pitch,
        position,
    })
}

unsafe fn parse_bgs_command(argc: c_int, argv: *const VALUE) -> Option<BgsCommand> {
    let args = slice_args(argc, argv);
    let path = parse_audio_path(args.get(0)?)?;
    let volume = parse_volume(args.get(1));
    let pitch = parse_pitch(args.get(2));
    let position = parse_position(args.get(3));
    Some(BgsCommand {
        path,
        volume,
        pitch,
        position,
    })
}

unsafe fn parse_me_command(argc: c_int, argv: *const VALUE) -> Option<MeCommand> {
    let args = slice_args(argc, argv);
    let path = parse_audio_path(args.get(0)?)?;
    let volume = parse_volume(args.get(1));
    let pitch = parse_pitch(args.get(2));
    Some(MeCommand {
        path,
        volume,
        pitch,
    })
}

unsafe fn parse_se_command(argc: c_int, argv: *const VALUE) -> Option<SeCommand> {
    let args = slice_args(argc, argv);
    let path = parse_audio_path(args.get(0)?)?;
    let volume = parse_volume(args.get(1));
    let pitch = parse_pitch(args.get(2));
    Some(SeCommand {
        path,
        volume,
        pitch,
    })
}

unsafe fn parse_duration(argc: c_int, argv: *const VALUE) -> Option<u32> {
    let args = slice_args(argc, argv);
    args.get(0).map(|value| rb_num2long(*value).max(0) as u32)
}

unsafe fn parse_audio_path(value: &VALUE) -> Option<PathBuf> {
    let mut arg = *value;
    let ptr = rb_string_value_cstr(&mut arg);
    if ptr.is_null() {
        return None;
    }
    let raw = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    Some(resolve_audio_path(&raw))
}

fn resolve_audio_path(input: &str) -> PathBuf {
    if input.trim().is_empty() {
        return PathBuf::from(input);
    }
    match fs::resolve(input) {
        Some(path) => path,
        None => Path::new(input).to_path_buf(),
    }
}

fn parse_volume(value: Option<&VALUE>) -> i32 {
    value
        .map(|val| unsafe { rb_num2long(*val) as i32 })
        .unwrap_or(100)
        .clamp(0, 100)
}

fn parse_pitch(value: Option<&VALUE>) -> i32 {
    value
        .map(|val| unsafe { rb_num2long(*val) as i32 })
        .unwrap_or(100)
        .clamp(50, 150)
}

fn parse_position(value: Option<&VALUE>) -> u32 {
    value
        .map(|val| unsafe { rb_num2long(*val).max(0) as u32 })
        .unwrap_or(0)
}

unsafe fn slice_args<'a>(argc: c_int, argv: *const VALUE) -> &'a [VALUE] {
    if argc <= 0 || argv.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(argv, argc as usize)
    }
}

fn define_method(module: VALUE, name: &[u8], func: RubyFn, argc: c_int) {
    unsafe {
        rb_define_module_function(module, c_name(name), Some(func), argc);
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
