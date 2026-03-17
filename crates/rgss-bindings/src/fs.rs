use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    env,
    path::{Component, Path, PathBuf},
    sync::RwLock,
};
use tracing::{debug, warn};
use walkdir::WalkDir;

static SEARCH: Lazy<RwLock<SearchState>> = Lazy::new(|| RwLock::new(SearchState::default()));

#[derive(Default)]
struct SearchState {
    base: Option<PathBuf>,
    mounts: Vec<MountEntry>,
    index: HashMap<String, PathBuf>,
}

#[derive(Clone, PartialEq, Eq)]
struct MountEntry {
    root: PathBuf,
    mountpoint: Option<PathBuf>,
}

impl SearchState {
    fn rebuild_index(&mut self) {
        self.index.clear();
        if let Some(base) = self.base.clone() {
            self.index_root(&base, None);
        }
        let mounts = self.mounts.clone();
        for entry in mounts {
            self.index_root(&entry.root, entry.mountpoint.as_ref());
        }
    }

    fn index_root(&mut self, root: &Path, mountpoint: Option<&PathBuf>) {
        if !root.exists() {
            warn!(target: "fs", path = %root.display(), "mount root missing during indexing");
            return;
        }
        let prefix = mountpoint.cloned().unwrap_or_default();
        // include the mount root itself for directory probes
        let key = key_for(&prefix);
        self.index.insert(key, root.to_path_buf());
        for entry in WalkDir::new(root).follow_links(false) {
            let entry = match entry {
                Ok(value) => value,
                Err(err) => {
                    warn!(target: "fs", error = %err, "walkdir error");
                    continue;
                }
            };
            let rel = match entry.path().strip_prefix(root) {
                Ok(path) => path,
                Err(_) => continue,
            };
            let mut virtual_path = prefix.clone();
            if !rel.as_os_str().is_empty() {
                virtual_path.push(rel);
            }
            let key = key_for(&virtual_path);
            self.index.insert(key, entry.path().to_path_buf());
        }
    }
}

pub fn set_base_root(path: &Path) {
    if let Ok(mut state) = SEARCH.write() {
        state.base = Some(path.to_path_buf());
        state.rebuild_index();
    }
}

pub fn data_root() -> Option<PathBuf> {
    SEARCH.read().ok()?.base.clone()
}

pub fn mount_path(root: PathBuf, mountpoint: Option<PathBuf>) -> bool {
    if !root.exists() {
        warn!(target: "fs", path = %root.display(), "mount target missing");
        return false;
    }
    if let Ok(mut state) = SEARCH.write() {
        let entry = MountEntry {
            root: root.clone(),
            mountpoint: mountpoint.clone(),
        };
        if state.mounts.contains(&entry) {
            return true;
        }
        state.mounts.push(entry);
        state.rebuild_index();
        debug!(
            target: "fs",
            root = %root.display(),
            mountpoint = mountpoint.as_ref().map(|p| p.display().to_string()),
            "mount installed"
        );
        return true;
    }
    false
}

pub fn unmount_path(root: &Path, mountpoint: Option<&Path>) -> bool {
    if let Ok(mut state) = SEARCH.write() {
        let before = state.mounts.len();
        state.mounts.retain(|entry| {
            if !same_root(entry, root) {
                return true;
            }
            match mountpoint {
                None => false,
                Some(target) => match &entry.mountpoint {
                    Some(existing) => !same_virtual(existing, target),
                    None => true,
                },
            }
        });
        if state.mounts.len() != before {
            state.rebuild_index();
            return true;
        }
    }
    false
}

pub fn reload() {
    if let Ok(mut state) = SEARCH.write() {
        state.rebuild_index();
    }
}

pub fn resolve(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    if path.is_absolute() {
        return path.exists().then(|| path.to_path_buf());
    }
    let normalized = normalize_relative(path);
    let key = key_for(&normalized);
    let state = SEARCH.read().ok()?;

    if let Some(base) = &state.base {
        let candidate = base.join(&normalized);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // mounted roots without mountpoint behave like additional bases
    for entry in &state.mounts {
        if entry.mountpoint.is_none() {
            let candidate = entry.root.join(&normalized);
            if candidate.exists() {
                return Some(candidate);
            }
        } else if let Some(prefix) = &entry.mountpoint {
            if normalized.starts_with(prefix) {
                if let Ok(stripped) = normalized.strip_prefix(prefix) {
                    let candidate = entry.root.join(stripped);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    state.index.get(&key).cloned()
}

pub fn exists(path: &str) -> bool {
    resolve(path).is_some()
}

pub fn desensitize(path: &str) -> Option<PathBuf> {
    if path.trim().is_empty() {
        return None;
    }
    resolve(path).or_else(|| {
        let candidate = PathBuf::from(path);
        candidate.canonicalize().ok()
    })
}

pub fn resolve_mount_source(path: &str) -> Option<PathBuf> {
    let provided = PathBuf::from(path);
    let absolute = if provided.is_absolute() {
        provided
    } else if let Some(base) = data_root() {
        base.join(provided)
    } else {
        env::current_dir().ok()?.join(path)
    };
    absolute.canonicalize().ok().or_else(|| Some(absolute))
}

pub fn clean_mountpoint(value: &str) -> PathBuf {
    let trimmed = value.trim().trim_start_matches(|c| c == '/' || c == '\\');
    let mut path = PathBuf::new();
    if !trimmed.is_empty() {
        path.push(trimmed);
    }
    normalize_relative(&path)
}

fn same_root(entry: &MountEntry, root: &Path) -> bool {
    entry.root == root
}

fn same_virtual(a: &Path, b: &Path) -> bool {
    key_for(a) == key_for(b)
}

fn normalize_relative(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push("..");
                }
            }
            Component::Normal(seg) => normalized.push(seg),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn key_for(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    text.trim_matches('/')
        .trim_start_matches("./")
        .to_lowercase()
}
