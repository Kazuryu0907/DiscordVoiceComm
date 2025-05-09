// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod vc;

use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, GuildChannel, GuildId, UserId};
use tauri::{AppHandle, State};
use tokio::{
    runtime::Runtime,
    sync::{Mutex, RwLock},
};
use vc::{types::PubIdentify, vc_client::VC};

struct Storage {
    vc: Mutex<VC>,
}

#[derive(Serialize, Deserialize, Clone)]
struct MyConfig {
    guild_id: GuildId,
    speaker1_api: String,
    speaker2_api: String,
    listener_api: String,
    user_volumes: HashMap<UserId, f32>,
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

#[tauri::command]
async fn get_voice_channels(storage: State<'_, Storage>) -> Result<Vec<GuildChannel>, String> {
    let vc = storage.vc.lock().await;
    let res = vc.get_voice_channels().await;
    Ok(res)
}

#[tauri::command(rename_all = "snake_case")]
async fn update_volume(
    user_id: UserId,
    volume: f32,
    storage: State<'_, Storage>,
) -> Result<(), String> {
    let vc = storage.vc.lock().await;
    vc.update_volume(user_id, volume).await;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
async fn update_is_listening(
    identify: PubIdentify,
    is_listening: bool,
    storage: State<'_, Storage>,
) -> Result<(), String> {
    let vc = storage.vc.lock().await;
    vc.update_is_listening(identify, is_listening).await;
    Ok(())
}
#[tauri::command(rename_all = "snake_case")]
async fn join(
    app: AppHandle,
    ch1: String,
    ch2: String,
    sub_ch: String,
    storage: State<'_, Storage>,
) -> Result<(), ()> {
    let vc = storage.vc.lock().await;
    vc.join(
        app,
        ChannelId::new(ch1.parse::<u64>().unwrap()),
        ChannelId::new(ch2.parse::<u64>().unwrap()),
        ChannelId::new(sub_ch.parse::<u64>().unwrap()),
    )
    .await;
    Ok(())
}

#[tauri::command]
async fn leave(storage: State<'_, Storage>) -> Result<(), ()> {
    let vc = storage.vc.lock().await;
    vc.leave().await;
    Ok(())
}

const ENV_PATH: &str = "./.env";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = confy::load_path::<MyConfig>(ENV_PATH).unwrap();
    let cfg_cpy = cfg.clone();
    let pub_token = cfg.speaker1_api;
    let pub_token2 = cfg.speaker2_api;
    let sub_token = cfg.listener_api;
    let guild_id = cfg.guild_id;
    let user_volumes = cfg.user_volumes;
    let user_volumes = Arc::new(RwLock::new(user_volumes));
    let mut vc = VC::new(guild_id, user_volumes.clone());
    let rt = Runtime::new().unwrap();
    rt.block_on(async { vc.start_bot(&pub_token, &pub_token2, &sub_token).await });

    let app = tauri::Builder::default()
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Storage { vc: Mutex::new(vc) })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            join,
            leave,
            get_voice_channels,
            update_volume,
            update_is_listening
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");
    let exit_code = app.run_return(move |_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let rt = Runtime::new().unwrap();
            let user_volumes = user_volumes.clone();
            let mut cfg_cpy = cfg_cpy.clone();
            rt.block_on(async move {
                let user_volumes_lock = user_volumes;
                let user_volumes = user_volumes_lock.read().await;
                cfg_cpy.user_volumes = user_volumes.clone();
                confy::store_path(ENV_PATH, cfg_cpy).unwrap();
            });
            api.prevent_exit();
        }
    });
    std::process::exit(exit_code);
}
