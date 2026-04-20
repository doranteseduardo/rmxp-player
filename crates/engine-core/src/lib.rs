use anyhow::{Context, Result};
use audio::{AudioHandle, AudioSystem};
use game::GameState;
use image::RgbaImage;
use input::InputState;
use once_cell::sync::{Lazy, OnceCell};
use platform::{self, EngineConfig};
use project::{GameDatabase, GameProject};
use render::{AutotileTexture, Renderer, TileScene};
use rgss_bindings::{
    current_frame_rate, graphics_tick, install_audio_hooks, install_window_hooks, native_snapshot, request_hangup,
    set_config_dir as rgss_set_config_dir, set_game_title, set_platform_info, set_project_root,
    set_save_dir as rgss_set_save_dir, sync_graphics_size, update_frame_delta, update_input,
    AudioHooks, MainLoopOutcome, PlatformInfo, RubyVm, ScriptSection, WindowHooks,
};
use rmxp_data::{MapData, SystemData};
use std::{
    env,
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tracing::{error, info, warn};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, Event, Ime, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Fullscreen, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

mod game;
mod input;
mod project;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub title: Option<String>,
}

impl AppConfig {
    fn finalize(self, stored: EngineConfig) -> FinalizedConfig {
        FinalizedConfig {
            window_width: self.window_width.unwrap_or(stored.window_width),
            window_height: self.window_height.unwrap_or(stored.window_height),
            title: self.title.unwrap_or(stored.title),
        }
    }
}

struct FinalizedConfig {
    window_width: u32,
    window_height: u32,
    title: String,
}

static WINDOW_HANDLE: OnceCell<usize> = OnceCell::new();
static WINDOW_TITLE: Lazy<RwLock<String>> =
    Lazy::new(|| RwLock::new(String::from("RMXP Native Player")));

pub fn run(config: AppConfig) -> Result<()> {
    platform::init_logging();
    let paths = platform::app_paths().context("initializing app paths")?;
    rgss_set_config_dir(&paths.config_dir);
    rgss_set_save_dir(&paths.save_dir);
    let stored = platform::load_or_default();
    let cfg = config.finalize(stored);
    set_game_title(cfg.title.clone());
    set_platform_info(detect_platform_info());

    let project = match GameProject::from_env() {
        Ok(project) => {
            set_project_root(project.root());
            // Change the process working directory to the game root so that
            // Ruby scripts can use relative paths (e.g. "Data/Map001.rxdata").
            if let Err(err) = std::env::set_current_dir(project.root()) {
                warn!(target: "project", error = %err, "Failed to chdir to game root");
            }
            info!(target: "project", root = ?project.data_dir(), "Project path resolved");
            Some(project)
        }
        Err(err) => {
            warn!(
                target: "project",
                error = %err,
                "No RMXP project configured; set RMXP_GAME_PATH"
            );
            None
        }
    };

    let (_database, initial_scene) = if let Some(project_ref) = project.as_ref() {
        match project_ref.load_database() {
            Ok(db) => {
                db.log_summary();
                if let Some(system) = db.system.as_ref() {
                    set_game_title(system.game_title.clone());
                }
                let scene = build_initial_tile_scene(project_ref, &db);
                (Some(db), scene)
            }
            Err(err) => {
                warn!(target: "project", error = %err, "Failed to load project data");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    let (tile_scene, player_start) = match initial_scene {
        Some((scene, spawn)) => (Some(scene), spawn),
        None => (None, (0.0, 0.0)),
    };

    let mut ruby_vm = RubyVm::new();
    ruby_vm.boot()?;
    let mut loaded_scripts: Option<Vec<rmxp_data::ScriptEntry>> = None;
    if let Some(project_ref) = project.as_ref() {
        match project_ref.load_scripts() {
            Ok(scripts) => {
                info!(
                    target: "project",
                    scripts = scripts.len(),
                    "scripts parsed"
                );
                loaded_scripts = Some(scripts);
            }
            Err(err) => {
                warn!(target: "project", error = %err, "Failed to load Scripts.rxdata");
            }
        }
        if let Some(ref scripts) = loaded_scripts {
            let sections: Vec<_> = scripts
                .iter()
                .map(|entry| ScriptSection {
                    id: entry.id,
                    name: entry.name.as_str(),
                    source: entry.source.as_str(),
                })
                .collect();
            if let Err(err) = ruby_vm.run_scripts(&sections) {
                warn!(target: "rgss", error = %err, "Failed to evaluate RGSS scripts");
            }
            let snapshot = native_snapshot();
            info!(
                target: "rgss",
                bitmaps = snapshot.bitmaps,
                sprites = snapshot.sprites,
                viewports = snapshot.viewports,
                windows = snapshot.windows,
                "RGSS native bridge ready"
            );
        }
    }

    if env::var("RMXP_SKIP_RENDERER").is_ok() {
        info!(target: "engine", "RMXP_SKIP_RENDERER set; exiting after script load");
        return Ok(());
    }

    let mut game = GameState::new(
        tile_scene,
        player_start,
        (cfg.window_width, cfg.window_height),
    );

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let window = Box::new(
        WindowBuilder::new()
            .with_title(cfg.title.clone())
            .with_inner_size(LogicalSize::new(
                cfg.window_width as f64,
                cfg.window_height as f64,
            ))
            .build(&event_loop)?,
    );

    let window_ptr: *mut winit::window::Window = Box::into_raw(window);
    register_window_hooks(window_ptr, &cfg.title);
    let mut renderer = Renderer::new(unsafe { &*window_ptr }, cfg.window_width, cfg.window_height)?;
    sync_graphics_size(cfg.window_width, cfg.window_height);
    rgss_bindings::sync_graphics_size(cfg.window_width, cfg.window_height);
    let audio = AudioSystem::new()?;
    install_audio_hooks(bind_audio_hooks(audio.handle()));
    let _audio = audio;

    let mut input = WinitInputHelper::new();
    let mut input_state = InputState::default();
    let mut last_tick = Instant::now();
    let mut frame_index: u64 = 0;
    let mut exit_requested = false;
    let mut had_main_loop = ruby_vm.has_main_loop();

    event_loop.run(move |event, target| {
        let window_ref = unsafe { &*window_ptr };
        if input.update(&event) {
            input_state.update_from_helper(&input);
            if input.close_requested() || input.destroyed() {
                request_hangup();
                exit_requested = true;
            }
        }

        match event {
            Event::WindowEvent { window_id, event } if window_id == window_ref.id() => {
                match event {
                    WindowEvent::Ime(Ime::Commit(text)) => {
                        for ch in text.chars() {
                            input_state.push_text_char(ch);
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.state == ElementState::Pressed {
                            if let Key::Named(NamedKey::Backspace) = event.logical_key {
                                input_state.push_backspace();
                            }
                        }
                    }
                    WindowEvent::Resized(size) => renderer.resize(size),
                    WindowEvent::ScaleFactorChanged { .. } => {
                        renderer.resize(window_ref.inner_size())
                    }
                    WindowEvent::CloseRequested => {
                        request_hangup();
                        exit_requested = true;
                    }
                    WindowEvent::RedrawRequested => {
                        if let Err(err) = renderer.render(frame_index, game.render_frame()) {
                            warn!(target: "render", error = %err, "render error, exiting");
                            target.exit();
                            return;
                        }
                        frame_index = frame_index.wrapping_add(1);
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                let frame_start = Instant::now();
                let delta = frame_start - last_tick;
                last_tick = frame_start;
                update_frame_delta(delta.as_secs_f64());
                graphics_tick(delta.as_secs_f64());
                update_input(input_state.snapshot());
                match ruby_vm.resume_main_loop() {
                    Ok(MainLoopOutcome::Active) => {
                        had_main_loop = true;
                    }
                    Ok(MainLoopOutcome::Done) => {
                        if exit_requested || had_main_loop {
                            target.exit();
                            return;
                        }
                    }
                    Ok(MainLoopOutcome::Reset) => {
                        // Re-evaluate all scripts from scratch (F12 / game reset).
                        if let Some(ref scripts) = loaded_scripts {
                            let sections: Vec<_> = scripts
                                .iter()
                                .map(|entry| ScriptSection {
                                    id: entry.id,
                                    name: entry.name.as_str(),
                                    source: entry.source.as_str(),
                                })
                                .collect();
                            if let Err(err) = ruby_vm.run_scripts(&sections) {
                                warn!(target: "rgss", error = %err, "Reset: failed to re-evaluate scripts");
                                target.exit();
                                return;
                            }
                            had_main_loop = ruby_vm.has_main_loop();
                        }
                    }
                    Err(err) => {
                        warn!(
                            target: "rgss",
                            error = %err,
                            "Ruby main loop error; stopping updates"
                        );
                        target.exit();
                        return;
                    }
                }
                window_ref.request_redraw();
                // Throttle to the target frame rate set by Graphics.frame_rate=.
                let target_fps = current_frame_rate() as f64;
                if target_fps > 0.0 {
                    let target_frame = Duration::from_secs_f64(1.0 / target_fps);
                    let elapsed = frame_start.elapsed();
                    if elapsed < target_frame {
                        std::thread::sleep(target_frame - elapsed);
                    }
                }
            }
            Event::Suspended => {
                if let Err(err) = rgss_bindings::notify_suspend() {
                    warn!(target: "rgss", error = %err, "Failed to notify suspend");
                }
            }
            Event::Resumed => {
                if let Err(err) = rgss_bindings::notify_resume() {
                    warn!(target: "rgss", error = %err, "Failed to notify resume");
                }
            }
            _ => {}
        }
    })?;

    unsafe {
        let _ = Box::from_raw(window_ptr);
    }

    Ok(())
}

const START_MAP_ENV: &str = "RMXP_START_MAP";

fn build_initial_tile_scene(
    project: &GameProject,
    db: &GameDatabase,
) -> Option<(TileScene, (f32, f32))> {
    let system = db.system.as_ref()?;
    let map_id = resolve_start_map_id(system);
    match project.load_map(map_id) {
        Ok(map) => {
            let tileset_entry = match db.tilesets.iter().find(|ts| ts.id == map.tileset_id) {
                Some(entry) => entry,
                None => {
                    warn!(
                        target: "project",
                        tileset_id = map.tileset_id,
                        "No tileset entry for map {}",
                        map_id
                    );
                    return None;
                }
            };
            info!(
                target: "project",
                map_id,
                tileset = %tileset_entry.name,
                base = %tileset_entry.tileset_name,
                autotiles = tileset_entry.autotile_names.len(),
                "Building tile scene"
            );
            let tileset = match project.load_tileset_image(&tileset_entry.tileset_name) {
                Ok(image) => Arc::new(image),
                Err(err) => {
                    warn!(
                        target: "project",
                        error = ?err,
                        tileset = %tileset_entry.tileset_name,
                        "Failed to load tileset for map {}",
                        map_id
                    );
                    return None;
                }
            };
            let autotiles = load_autotile_textures(project, &tileset_entry.autotile_names);
            let priorities = tileset_entry
                .priorities
                .as_ref()
                .map(|table| {
                    table
                        .values
                        .iter()
                        .map(|v| (*v).clamp(0, 6) as u8)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            map_to_tile_scene(&map, tileset, autotiles, priorities)
                .map(|scene| (scene, (system.start_x as f32, system.start_y as f32)))
        }
        Err(err) => {
            warn!(
                target: "project",
                error = ?err,
                "Failed to load start map {}",
                map_id
            );
            None
        }
    }
}

fn resolve_start_map_id(system: &SystemData) -> i32 {
    if let Ok(value) = env::var(START_MAP_ENV) {
        match value.trim().parse::<i32>() {
            Ok(id) if id > 0 => {
                info!(
                    target: "project",
                    override_env = START_MAP_ENV,
                    map_id = id,
                    "Using start map override"
                );
                return id;
            }
            Ok(_) => {
                warn!(
                    target: "project",
                    override_env = START_MAP_ENV,
                    value = %value,
                    "Start map override must be positive; falling back to System.rxdata"
                );
            }
            Err(err) => {
                warn!(
                    target: "project",
                    override_env = START_MAP_ENV,
                    error = %err,
                    value = %value,
                    "Failed to parse start map override; falling back to System.rxdata"
                );
            }
        }
    }
    system.start_map_id.max(1)
}

fn bind_audio_hooks(audio: AudioHandle) -> AudioHooks {
    AudioHooks {
        bgm_play: {
            let audio = audio.clone();
            Box::new(move |cmd| {
                log_audio_result(
                    "bgm_play",
                    Some(&cmd.path),
                    audio.play_bgm(&cmd.path, cmd.volume, cmd.pitch, cmd.position),
                );
            })
        },
        bgm_stop: {
            let audio = audio.clone();
            Box::new(move |_| audio.stop_bgm())
        },
        bgm_fade: {
            let audio = audio.clone();
            Box::new(move |frames| audio.fade_bgm(frames))
        },
        bgs_play: {
            let audio = audio.clone();
            Box::new(move |cmd| {
                log_audio_result(
                    "bgs_play",
                    Some(&cmd.path),
                    audio.play_bgs(&cmd.path, cmd.volume, cmd.pitch, cmd.position),
                );
            })
        },
        bgs_stop: {
            let audio = audio.clone();
            Box::new(move |_| audio.stop_bgs())
        },
        bgs_fade: {
            let audio = audio.clone();
            Box::new(move |frames| audio.fade_bgs(frames))
        },
        me_play: {
            let audio = audio.clone();
            Box::new(move |cmd| {
                log_audio_result(
                    "me_play",
                    Some(&cmd.path),
                    audio.play_me(&cmd.path, cmd.volume, cmd.pitch),
                );
            })
        },
        me_stop: {
            let audio = audio.clone();
            Box::new(move |_| audio.stop_me())
        },
        me_fade: {
            let audio = audio.clone();
            Box::new(move |frames| audio.fade_me(frames))
        },
        se_play: {
            let audio = audio.clone();
            Box::new(move |cmd| {
                log_audio_result(
                    "se_play",
                    Some(&cmd.path),
                    audio.play_se(&cmd.path, cmd.volume, cmd.pitch),
                );
            })
        },
        se_stop: {
            let audio = audio.clone();
            Box::new(move |_| audio.stop_all_se())
        },
        se_fade: {
            let audio = audio.clone();
            Box::new(move |frames| audio.fade_all_se(frames))
        },
    }
}

fn log_audio_result(
    method: &str,
    path: Option<&Path>,
    result: std::result::Result<(), audio::AudioError>,
) {
    if let Err(err) = result {
        match path {
            Some(p) => error!(
                target: "audio",
                method = %method,
                file = %p.display(),
                error = %err,
                "Audio backend error"
            ),
            None => error!(
                target: "audio",
                method = %method,
                error = %err,
                "Audio backend error"
            ),
        }
    }
}

fn register_window_hooks(window_ptr: *mut winit::window::Window, initial_title: &str) {
    let _ = WINDOW_HANDLE.set(window_ptr as usize);
    if let Ok(mut guard) = WINDOW_TITLE.write() {
        *guard = initial_title.to_string();
    }
    install_window_hooks(WindowHooks {
        set_title: engine_set_window_title,
        get_title: engine_get_window_title,
        set_inner_size: engine_set_inner_size,
        get_display_size: engine_get_display_size,
        center: engine_center_window,
        set_fullscreen: engine_set_fullscreen,
        set_cursor_visible: engine_set_cursor_visible,
    });
}

fn with_window<F>(f: F)
where
    F: FnOnce(&winit::window::Window),
{
    if let Some(handle) = WINDOW_HANDLE.get() {
        let raw = *handle as *mut winit::window::Window;
        unsafe {
            if let Some(window) = raw.as_ref() {
                f(window);
            }
        }
    }
}

fn engine_set_window_title(title: &str) {
    {
        if let Ok(mut guard) = WINDOW_TITLE.write() {
            *guard = title.to_string();
        }
    }
    with_window(|window| window.set_title(title));
}

fn engine_get_window_title() -> String {
    WINDOW_TITLE
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_else(|_| "RMXP Native Player".into())
}

fn engine_set_inner_size(width: u32, height: u32) {
    with_window(|window| {
        let _ = window.request_inner_size(LogicalSize::new(width as f64, height as f64));
    });
}

fn engine_get_display_size() -> (u32, u32) {
    let mut size = (512u32, 384u32);
    with_window(|window| {
        let inner = window.inner_size();
        size = (inner.width.max(1), inner.height.max(1));
    });
    size
}

fn engine_center_window() {
    // All winit monitor-enumeration paths on macOS trigger an icrate-0.0.4
    // NSUInteger/NSInteger type-code mismatch panic.  Window centering is
    // cosmetic; skip it until icrate is updated or we switch to a CoreGraphics
    // path that bypasses NSScreen enumeration.
}

fn engine_set_fullscreen(enable: bool) {
    with_window(|window| {
        if enable {
            // Use primary_monitor() to avoid NSScreen fast-enumeration
            // icrate type-mismatch panic on macOS.
            let target_monitor = window.primary_monitor();
            window.set_fullscreen(Some(Fullscreen::Borderless(target_monitor)));
        } else {
            window.set_fullscreen(None);
        }
    });
}

fn engine_set_cursor_visible(visible: bool) {
    with_window(|window| window.set_cursor_visible(visible));
}

fn detect_platform_info() -> PlatformInfo {
    PlatformInfo {
        platform: detect_platform_name(),
        user_name: detect_user_name(),
        user_language: detect_user_language(),
    }
}

fn detect_platform_name() -> String {
    match env::consts::OS {
        "macos" => "macOS".into(),
        "windows" => "Windows".into(),
        "linux" => "Linux".into(),
        other => other.to_string(),
    }
}

fn detect_user_name() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "Player".into())
}

fn detect_user_language() -> String {
    const LANG_VARS: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];
    for var in LANG_VARS {
        if let Ok(value) = env::var(var) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return normalize_locale(trimmed);
            }
        }
    }
    "en_US".into()
}

fn normalize_locale(value: &str) -> String {
    let base = value.split('.').next().unwrap_or(value);
    if let Some((lang, region)) = base.split_once(['_', '-']) {
        format!("{}_{}", lang.to_lowercase(), region.to_uppercase())
    } else {
        base.to_lowercase()
    }
}

fn load_autotile_textures(project: &GameProject, names: &[String]) -> Vec<Option<AutotileTexture>> {
    names
        .iter()
        .map(|name| {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                return None;
            }
            match project.load_autotile_image(trimmed) {
                Ok(image) => {
                    let arc = Arc::new(image);
                    Some(AutotileTexture::new(arc))
                }
                Err(err) => {
                    warn!(
                        target: "project",
                        autotile = %trimmed,
                        error = ?err,
                        "Failed to load autotile"
                    );
                    None
                }
            }
        })
        .collect()
}

fn map_to_tile_scene(
    map: &MapData,
    tileset: Arc<RgbaImage>,
    autotiles: Vec<Option<AutotileTexture>>,
    priorities: Vec<u8>,
) -> Option<TileScene> {
    let width = map.width.max(1) as usize;
    let height = map.height.max(1) as usize;
    let mut layers = Vec::new();
    for z in 0..map.data.zsize {
        if let Some(plane) = map.data.plane(z) {
            layers.push(plane);
        }
    }
    if layers.is_empty() {
        return None;
    }
    Some(TileScene {
        map_width: width,
        map_height: height,
        tile_size: 32,
        tileset,
        autotiles,
        layers,
        priorities,
        flash_map: None,
        flash_phase: 0,
    })
}
