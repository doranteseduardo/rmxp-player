use anyhow::{Context, Result};
use audio::AudioSystem;
use platform::{self, EngineConfig};
use project::{GameDatabase, GameProject};
use render::{Renderer, TileDebugView};
use rgss_bindings::RubyVm;
use rmxp_data::MapData;
use tracing::{info, warn};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

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

    let (_database, tile_view) = if let Some(project_ref) = project.as_ref() {
        match project_ref.load_database() {
            Ok(db) => {
                db.log_summary();
                let view = build_initial_tile_view(project_ref, &db);
                (Some(db), view)
            }
            Err(err) => {
                warn!(target: "project", error = %err, "Failed to load project data");
                (None, None)
            }
        }
    } else {
        (None, None)
    };

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
    let _audio = AudioSystem::new()?;

    let mut ruby_vm = RubyVm::new();
    ruby_vm.boot()?;

    let mut input = WinitInputHelper::new();
    let mut frame_index: u64 = 0;

    event_loop.run(move |event, target| {
        let window_ref = unsafe { &*window_ptr };
        if input.update(&event) {
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
                        if let Err(err) = renderer.render(frame_index, tile_view.as_ref()) {
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

fn build_initial_tile_view(project: &GameProject, db: &GameDatabase) -> Option<TileDebugView> {
    let system = db.system.as_ref()?;
    let map_id = system.start_map_id.max(1);
    match project.load_map(map_id) {
        Ok(map) => map_to_tile_view(&map),
        Err(err) => {
            warn!(
                target: "project",
                error = %err,
                "Failed to load start map {}",
                map_id
            );
            None
        }
    }
}

fn map_to_tile_view(map: &MapData) -> Option<TileDebugView> {
    let width = map.width.max(1) as usize;
    let height = map.height.max(1) as usize;
    let tiles = map.data.plane(0)?;
    Some(TileDebugView {
        width,
        height,
        tiles,
    })
}
