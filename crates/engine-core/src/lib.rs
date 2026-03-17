use anyhow::{Context, Result};
use audio::AudioSystem;
use game::GameState;
use image::RgbaImage;
use input::InputState;
use platform::{self, EngineConfig};
use project::{GameDatabase, GameProject};
use render::{AutotileTexture, Renderer, TileScene};
use rgss_bindings::{sync_graphics_size, update_input, RubyVm, ScriptSection};
use rmxp_data::{MapData, SystemData};
use std::{
    env,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{info, warn};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
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

const FIXED_TIMESTEP: Duration = Duration::from_nanos(16_666_667);

struct FinalizedConfig {
    window_width: u32,
    window_height: u32,
    title: String,
}

pub fn run(config: AppConfig) -> Result<()> {
    platform::init_logging();
    platform::app_paths().context("initializing app paths")?;
    let stored = platform::load_or_default();
    let cfg = config.finalize(stored);

    let project = match GameProject::from_env() {
        Ok(project) => {
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
    let mut renderer = Renderer::new(unsafe { &*window_ptr }, cfg.window_width, cfg.window_height)?;
    sync_graphics_size(cfg.window_width, cfg.window_height);
    rgss_bindings::sync_graphics_size(cfg.window_width, cfg.window_height);
    let _audio = AudioSystem::new()?;

    let mut ruby_vm = RubyVm::new();
    ruby_vm.boot()?;
    if let Some(project_ref) = project.as_ref() {
        match project_ref.load_scripts() {
            Ok(scripts) => {
                info!(
                    target: "project",
                    scripts = scripts.len(),
                    "scripts parsed"
                );
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
            }
            Err(err) => {
                warn!(target: "project", error = %err, "Failed to load Scripts.rxdata");
            }
        }
    }

    let mut input = WinitInputHelper::new();
    let mut input_state = InputState::default();
    let mut accumulator = Duration::ZERO;
    let mut last_tick = Instant::now();
    let mut frame_index: u64 = 0;

    event_loop.run(move |event, target| {
        let window_ref = unsafe { &*window_ptr };
        if input.update(&event) {
            input_state.update_from_helper(&input);
            if input.close_requested() || input.destroyed() {
                target.exit();
                return;
            }
        }

        match event {
            Event::WindowEvent { window_id, event } if window_id == window_ref.id() => {
                match event {
                    WindowEvent::Resized(size) => renderer.resize(size),
                    WindowEvent::ScaleFactorChanged { .. } => {
                        renderer.resize(window_ref.inner_size())
                    }
                    WindowEvent::CloseRequested => target.exit(),
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
                let now = Instant::now();
                accumulator += now - last_tick;
                last_tick = now;
                while accumulator >= FIXED_TIMESTEP {
                    update_input(input_state.snapshot());
                    game.update(FIXED_TIMESTEP, &input_state);
                    accumulator -= FIXED_TIMESTEP;
                }
                window_ref.request_redraw();
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
    })
}
