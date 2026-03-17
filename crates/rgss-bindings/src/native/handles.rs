use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
};

pub struct HandleStore<T> {
    next: AtomicU32,
    entries: Mutex<HashMap<u32, T>>,
}

impl<T> Default for HandleStore<T> {
    fn default() -> Self {
        Self {
            next: AtomicU32::new(1),
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl<T> HandleStore<T> {
    pub fn insert(&self, value: T) -> u32 {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut map) = self.entries.lock() {
            map.insert(id, value);
        }
        id
    }

    #[allow(dead_code)]
    pub fn remove(&self, id: u32) -> Option<T> {
        self.entries.lock().ok()?.remove(&id)
    }

    pub fn with<R>(&self, id: u32, func: impl FnOnce(&T) -> R) -> Option<R> {
        let map = self.entries.lock().ok()?;
        map.get(&id).map(func)
    }

    pub fn with_mut<R>(&self, id: u32, func: impl FnOnce(&mut T) -> R) -> Option<R> {
        let mut map = self.entries.lock().ok()?;
        map.get_mut(&id).map(func)
    }

    #[allow(dead_code)]
    pub fn snapshot(&self) -> Vec<(u32, T)>
    where
        T: Clone,
    {
        self.entries
            .lock()
            .map(|map| map.iter().map(|(&id, value)| (id, value.clone())).collect())
            .unwrap_or_default()
    }
}

pub fn init() {}
