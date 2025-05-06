// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod vc;

use serenity::all::{ChannelId, GuildChannel, GuildId, UserId};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{runtime::Runtime, sync::Mutex};
use vc::vc::VC;

struct Storage {
    vc: Mutex<VC>,
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

const GUILD_ID: GuildId = GuildId::new(950683443266748416);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut vc = VC::new(GUILD_ID);
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        vc.start_bot(
            "OTUwNjgxNDk1OTQ3ODQ5NzM4.GDYX5e.UFP6qLgEoRxPZ0McRY1xQFQTT6rkBd1awp8ios",
            "OTUxMDUyMDgwODc1OTY2NTE0.Gb2L5W.ycBGJKuodRIluJoFjjyBfgSXuz1ixpVvU69GQI",
            "OTU4MDQ1MTg1NzA5ODM4MzQ2.GJBCO9.NYRkja-klexwYf1moVh2pypPU4yI-1Xq4u6avk",
        )
        .await
    });

    tauri::Builder::default()
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Storage { vc: Mutex::new(vc) })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            join,
            leave,
            get_voice_channels,
            update_volume
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
