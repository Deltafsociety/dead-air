use std::fs;
use std::path::PathBuf;
use nostr::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    secret_key_hex: String,
    #[serde(default)]
    name: Option<String>,
}

pub struct Identity {
    pub keys: Keys,
    pub npub: String,
    pub short_npub: String,
    pub name: String,
}

impl Identity {
    pub fn load_or_create() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let path = Self::identity_path()?;
        
        let (keys, name) = if path.exists() {
            let data = fs::read_to_string(&path)?;
            let stored: StoredIdentity = serde_json::from_str(&data)?;
            let secret_key = SecretKey::parse(&stored.secret_key_hex)?;
            let keys = Keys::new(secret_key);
            let name = stored.name.unwrap_or_else(|| {
                let pk = keys.public_key().to_hex();
                format!("station-{}", &pk[..6])
            });
            (keys, name)
        } else {
            let keys = Keys::generate();
            let pk = keys.public_key().to_hex();
            let name = format!("station-{}", &pk[..6]);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let stored = StoredIdentity {
                secret_key_hex: keys.secret_key().to_secret_hex(),
                name: Some(name.clone()),
            };
            fs::write(&path, serde_json::to_string_pretty(&stored)?)?;
            (keys, name)
        };

        let npub = keys.public_key().to_bech32().unwrap_or_else(|_| keys.public_key().to_hex());
        let short_npub = if npub.len() > 16 {
            format!("{}...{}", &npub[..8], &npub[npub.len() - 6..])
        } else {
            npub.clone()
        };

        Ok(Self {
            keys,
            npub,
            short_npub,
            name,
        })
    }

    pub fn set_name(&mut self, new_name: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.name = new_name.clone();
        let path = Self::identity_path()?;
        let stored = StoredIdentity {
            secret_key_hex: self.keys.secret_key().to_secret_hex(),
            name: Some(new_name),
        };
        fs::write(&path, serde_json::to_string_pretty(&stored)?)?;
        Ok(())
    }

    fn identity_path() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("deadair").join("identity.json"))
        } else {
            Ok(PathBuf::from(".deadair_identity.json"))
        }
    }
}
