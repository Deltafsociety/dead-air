use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayBookmark {
    pub url: String,
    pub name: String,
    pub auto_connect: bool,
}

pub struct BookmarkManager {
    storage_path: PathBuf,
    bookmarks: Vec<RelayBookmark>,
}

impl BookmarkManager {
    pub fn load() -> Self {
        let storage_path = Self::path();
        let bookmarks = if storage_path.exists() {
            if let Ok(data) = fs::read_to_string(&storage_path) {
                serde_json::from_str::<Vec<RelayBookmark>>(&data).unwrap_or_else(|_| Self::defaults())
            } else {
                Self::defaults()
            }
        } else {
            let defaults = Self::defaults();
            if let Some(parent) = storage_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(data) = serde_json::to_string_pretty(&defaults) {
                let _ = fs::write(&storage_path, data);
            }
            defaults
        };

        Self {
            storage_path,
            bookmarks,
        }
    }

    pub fn add(&mut self, url: String, name: String) {
        if let Some(existing) = self.bookmarks.iter_mut().find(|b| b.url == url) {
            existing.name = name;
        } else {
            self.bookmarks.push(RelayBookmark {
                url,
                name,
                auto_connect: true,
            });
        }
        self.persist();
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.bookmarks.len();
        self.bookmarks.retain(|b| b.url != key && b.name != key);
        let removed = self.bookmarks.len() < before;
        if removed {
            self.persist();
        }
        removed
    }

    pub fn list(&self) -> &[RelayBookmark] {
        &self.bookmarks
    }

    pub fn auto_connect_urls(&self) -> Vec<String> {
        self.bookmarks
            .iter()
            .filter(|b| b.auto_connect)
            .map(|b| b.url.clone())
            .collect()
    }

    fn defaults() -> Vec<RelayBookmark> {
        vec![
            RelayBookmark {
                url: "ws://127.0.0.1:4869".to_string(),
                name: "local-node".to_string(),
                auto_connect: true,
            },
        ]
    }

    fn path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("deadair").join("domestic_relays.json")
        } else {
            PathBuf::from(".deadair_domestic_relays.json")
        }
    }

    fn persist(&self) {
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(&self.bookmarks) {
            let _ = fs::write(&self.storage_path, data);
        }
    }
}
