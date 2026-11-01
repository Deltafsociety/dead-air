use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use nostr::prelude::*;

pub struct Outbox {
    queue: VecDeque<Event>,
    storage_path: PathBuf,
}

impl Outbox {
    pub fn new() -> Self {
        let storage_path = Self::path();
        let queue = Self::load(&storage_path);
        Self {
            queue,
            storage_path,
        }
    }

    pub fn push(&mut self, event: Event) {
        if !self.queue.iter().any(|e| e.id == event.id) {
            self.queue.push_back(event);
            self.persist();
        }
    }

    pub fn pending(&self) -> Vec<Event> {
        self.queue.iter().cloned().collect()
    }

    pub fn remove(&mut self, id: &EventId) {
        self.queue.retain(|e| &e.id != id);
        self.persist();
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("deadair").join("outbox.json")
        } else {
            PathBuf::from(".deadair_outbox.json")
        }
    }

    fn load(path: &PathBuf) -> VecDeque<Event> {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(events) = serde_json::from_str::<Vec<Event>>(&data) {
                return events.into();
            }
        }
        VecDeque::new()
    }

    fn persist(&self) {
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let list: Vec<Event> = self.queue.iter().cloned().collect();
        if let Ok(json) = serde_json::to_string_pretty(&list) {
            let _ = fs::write(&self.storage_path, json);
        }
    }
}
