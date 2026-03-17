use anyhow::{Context, Result};
use rmxp_data::{
    load_file, parse_map, parse_map_infos, parse_system, MapData, MapInfoEntry, RubyValue,
    SystemData,
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
        let system = match &system_raw {
            Some(value) => Some(parse_system(value).context("parsing System.rxdata")?),
            None => None,
        };
        let map_infos = match &map_infos_raw {
            Some(value) => parse_map_infos(value).context("parsing MapInfos.rxdata")?,
            None => Vec::new(),
        };
        Ok(GameDatabase {
            system_raw,
            map_infos_raw,
            system,
            map_infos,
        })
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
