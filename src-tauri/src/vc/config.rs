use std::{collections::HashMap, sync::Mutex};

use confy::ConfyError;
use serde::{Deserialize, Serialize};
use serenity::all::{GuildId, UserId};

#[derive(Serialize, Deserialize, Clone)]
pub struct MyConfig {
    pub guild_id: GuildId,
    pub speaker1_api: String,
    pub speaker2_api: String,
    pub listener_api: String,
    pub user_volumes: HashMap<UserId, f32>,
}

impl ::std::default::Default for MyConfig {
    fn default() -> Self {
        Self {
            guild_id: GuildId::new(1),
            speaker1_api: "API_HERE".to_owned(),
            speaker2_api: "API_HERE".to_owned(),
            listener_api: "API_HERE".to_owned(),
            user_volumes: HashMap::new(),
        }
    }
}

// static CFG:LazyLock<Arc<Mutex<MyConfig>>> = LazyLock::new(|| Arc::new(Mutex::new(confy::load_path::<MyConfig>(ENV_PATH).unwrap())));

pub struct ConfigManager {
    path: String,
    cfg: Mutex<MyConfig>,
}
impl ConfigManager {
    pub fn new(path: String) -> Self {
        let cfg = confy::load_path::<MyConfig>(&path)
            .unwrap_or_else(|e| {
                eprintln!("Warning: Could not load config from {}: {}. Using default config.", path, e);
                MyConfig::default()
            });
        ConfigManager {
            path: path.clone(),
            cfg: Mutex::new(cfg),
        }
    }
    pub fn get_cfg(&self) -> MyConfig {
        let cfg = self.cfg.lock().unwrap_or_else(|poisoned| {
            eprintln!("Warning: Config mutex was poisoned, recovering...");
            poisoned.into_inner()
        });
        cfg.clone()
    }
    pub fn update_volume(&self, user_id: UserId, volume: f32) -> Result<(), ConfyError> {
        let mut cfg = self.cfg.lock().unwrap_or_else(|poisoned| {
            eprintln!("Warning: Config mutex was poisoned during volume update, recovering...");
            poisoned.into_inner()
        });
        cfg.user_volumes.insert(user_id, volume);
        let cfg_cpy = cfg.clone();
        confy::store_path(&self.path, cfg_cpy)
    }
}
