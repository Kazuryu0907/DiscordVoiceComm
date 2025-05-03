// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod vc;

use serenity::all::{ChannelId, GuildId};
use tauri::State;
use tokio::{runtime::Runtime, sync::Mutex,};
use vc::{types::JoinInfo, vc::VC};

struct Storage {
    vc: Mutex<VC>,
}

#[tauri::command]
async fn greet(name: &str, storage: State<'_,Storage>) -> Result<String,()> {
    let mut vc = storage.vc.lock().await;
    let pub_info = JoinInfo {
        guild_id: GuildId::new(950683443266748416),
        channel_id: ChannelId::new(950683443266748420),
    };
    let sub_info = JoinInfo {
        guild_id: GuildId::new(950683443266748416),
        channel_id: ChannelId::new(1368044397731778710),
    };
    let pub_info2 = JoinInfo {
        guild_id: GuildId::new(950683443266748416),
        channel_id: ChannelId::new(951051352665104404),
    };
    vc.join(pub_info,pub_info2, sub_info).await;
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

#[tauri::command]
async fn leave(storage: State<'_,Storage>) -> Result<(),()>{
    let vc = storage.vc.lock().await;
    vc.leave().await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut vc = VC::new(
        "OTUwNjgxNDk1OTQ3ODQ5NzM4.GDYX5e.UFP6qLgEoRxPZ0McRY1xQFQTT6rkBd1awp8ios",
        "OTUxMDUyMDgwODc1OTY2NTE0.Gb2L5W.ycBGJKuodRIluJoFjjyBfgSXuz1ixpVvU69GQI",
        "OTU4MDQ1MTg1NzA5ODM4MzQ2.GJBCO9.NYRkja-klexwYf1moVh2pypPU4yI-1Xq4u6avk",
    );
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        vc.start_bot().await
    });

    tauri::Builder::default()
        .manage(Storage {vc: Mutex::new(vc)})
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet,leave])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
