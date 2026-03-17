use anyhow::{Context, Result};
use image::{ImageReader, RgbaImage};
use rmxp_data::{
    load_file, parse_map, parse_map_infos, parse_system, parse_tilesets, MapData, MapInfoEntry,
    RubyValue, SystemData, TilesetData,
};
use std::env;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const GAME_PATH_ENV: &str = "RMXP_GAME_PATH";

pub struct GameProject {
    root: PathBuf,
}

pub struct GameDatabase {
    pub system_raw: Option<RubyValue>,
    pub map_infos_raw: Option<RubyValue>,
    pub system: Option<SystemData>,
    pub map_infos: Vec<MapInfoEntry>,
    pub tilesets_raw: Option<RubyValue>,
    pub tilesets: Vec<TilesetData>,
}

impl GameProject {
    pub fn from_env() -> Result<Self> {
        let value = env::var(GAME_PATH_ENV)
            .with_context(|| format!("{} env var not set", GAME_PATH_ENV))?;
        let root = PathBuf::from(value);
        if !root.exists() {
            anyhow::bail!("{} does not exist", root.display());
        }
        Ok(Self { root })
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("Data")
    }

    pub fn load_tileset_image(&self, name: &str) -> Result<RgbaImage> {
        self.load_graphic_image("Tilesets", name, "tileset")
    }

    pub fn load_autotile_image(&self, name: &str) -> Result<RgbaImage> {
        self.load_graphic_image("Autotiles", name, "autotile")
    }

    pub fn load_map(&self, map_id: i32) -> Result<MapData> {
        let filename = format!("Map{:0>3}.rxdata", map_id);
        let path = self.data_dir().join(filename);
        let value = load_file(&path).with_context(|| format!("loading {}", path.display()))?;
        parse_map(&value)
    }

    pub fn load_database(&self) -> Result<GameDatabase> {
        let data_dir = self.data_dir();
        let system_raw = load_optional(&data_dir, "System.rxdata");
        let map_infos_raw = load_optional(&data_dir, "MapInfos.rxdata");
        let tilesets_raw = load_optional(&data_dir, "Tilesets.rxdata");
        let system = match &system_raw {
            Some(value) => Some(parse_system(value).context("parsing System.rxdata")?),
            None => None,
        };
        let map_infos = match &map_infos_raw {
            Some(value) => parse_map_infos(value).context("parsing MapInfos.rxdata")?,
            None => Vec::new(),
        };
        let tilesets = match &tilesets_raw {
            Some(value) => parse_tilesets(value).context("parsing Tilesets.rxdata")?,
            None => Vec::new(),
        };
        Ok(GameDatabase {
            system_raw,
            map_infos_raw,
            system,
            map_infos,
            tilesets_raw,
            tilesets,
        })
    }
    fn load_graphic_image(&self, folder: &str, name: &str, kind: &str) -> Result<RgbaImage> {
        let path = self
            .resolve_graphics_asset(folder, name)
            .with_context(|| format!("locating {} {}", kind, name))?;
        let image = ImageReader::open(&path)
            .with_context(|| format!("opening {}", path.display()))?
            .decode()
            .with_context(|| format!("decoding {}", path.display()))?
            .to_rgba8();
        info!(
            target: "project",
            file = %path.display(),
            width = image.width(),
            height = image.height(),
            "{} loaded",
            kind
        );
        Ok(image)
    }
}

impl GameDatabase {
    pub fn log_summary(&self) {
        match (&self.system_raw, &self.system) {
            (Some(_), Some(system)) => {
                info!(
                    target: "project",
                    title = %system.game_title,
                    start_map = system.start_map_id,
                    start = %format!("{},{}", system.start_x, system.start_y),
                    party_size = system.party_members.len(),
                    "system data parsed"
                );
            }
            (Some(_), None) => warn!(target: "project", "System.rxdata loaded but not parsed"),
            (None, _) => {
                warn!(
                    target: "project",
                    "System.rxdata missing; set {} or place file in Data/",
                    GAME_PATH_ENV
                );
            }
        }
        if !self.map_infos.is_empty() {
            info!(
                target: "project",
                maps = self.map_infos.len(),
                "map infos parsed"
            );
        } else if self.map_infos_raw.is_some() {
            warn!(target: "project", "MapInfos.rxdata loaded but empty");
        } else {
            warn!(target: "project", "MapInfos.rxdata missing");
        }

        if !self.tilesets.is_empty() {
            info!(
                target: "project",
                tilesets = self.tilesets.len(),
                "tilesets parsed"
            );
        } else if self.tilesets_raw.is_some() {
            warn!(target: "project", "Tilesets.rxdata loaded but empty");
        } else {
            warn!(target: "project", "Tilesets.rxdata missing");
        }
    }
}

fn load_optional(dir: &Path, name: &str) -> Option<RubyValue> {
    let path = dir.join(name);
    if !path.exists() {
        return None;
    }
    match load_file(&path) {
        Ok(value) => Some(value),
        Err(err) => {
            warn!(target: "project", file = ?path, error = %err, "Failed to load data file");
            None
        }
    }
}

impl GameProject {
    fn resolve_graphics_asset(&self, folder: &str, name: &str) -> Result<PathBuf> {
        if name.trim().is_empty() {
            anyhow::bail!("graphic name is empty");
        }
        let base = self.root.join("Graphics").join(folder);
        let mut attempts = Vec::new();
        let provided = base.join(name);
        attempts.push(provided.clone());
        if Path::new(name).extension().is_none() {
            attempts.push(provided.with_extension("png"));
            attempts.push(provided.with_extension("PNG"));
        }
        for candidate in attempts {
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        anyhow::bail!(
            "graphic {} not found under {}",
            name,
            base.display()
        );
    }
}
