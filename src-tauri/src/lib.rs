// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod vc;

use std::sync::Arc;

use serenity::all::{ChannelId, GuildChannel, UserId};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::{Mutex, RwLock};
use vc::{config::ConfigManager, types::PubIdentify, vc_client::VC};

struct Storage {
    vc: Mutex<VC>,
    config_manager: Mutex<ConfigManager>,
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
    {
        let vc = storage.vc.lock().await;
        vc.update_volume(user_id, volume).await;
    }
    {
        let cfg_manager = storage.config_manager.lock().await;
        if let Err(_e) = cfg_manager.update_volume(user_id, volume) {
            return Err("Config write error".to_string());
        }
    }
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
) -> Result<(), String> {
    // Parse channel IDs with proper error handling
    let ch1_id = ch1.parse::<u64>()
        .map_err(|e| format!("Invalid channel ID '{}': {}", ch1, e))?;
    let ch2_id = ch2.parse::<u64>()
        .map_err(|e| format!("Invalid channel ID '{}': {}", ch2, e))?;
    let sub_ch_id = sub_ch.parse::<u64>()
        .map_err(|e| format!("Invalid sub channel ID '{}': {}", sub_ch, e))?;
    
    let vc = storage.vc.lock().await;
    vc.join(
        app,
        ChannelId::new(ch1_id),
        ChannelId::new(ch2_id),
        ChannelId::new(sub_ch_id),
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
    let cfg_manager = ConfigManager::new(ENV_PATH.to_string());
    let cfg = cfg_manager.get_cfg();
    let pub_token = cfg.speaker1_api;
    let pub_token2 = cfg.speaker2_api;
    let sub_token = cfg.listener_api;
    let guild_id = cfg.guild_id;
    let user_volumes = cfg.user_volumes;
    let user_volumes = Arc::new(RwLock::new(user_volumes));
    let mut vc = VC::new(guild_id, user_volumes.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = update(handle).await {
                    eprintln!("Update check failed: {}", e);
                }
            });
            let res = tauri::async_runtime::block_on(async {
                vc.start_bot(&pub_token, &pub_token2, &sub_token).await
            });

            // Stateの登録
            app.manage(Storage {
                vc: Mutex::new(vc),
                config_manager: Mutex::new(cfg_manager),
            });
            // API関連でエラーが発生した場合
            if let Err(e) = res {
                // Explorer表示
                eprintln!("Error starting bot: {}", e);
                let shell = app.handle().shell();
                let pwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                let exp_shell = shell
                    .command("explorer.exe")
                    .arg(pwd);
                // Dialog表示
                let res = app.dialog()
                    .message("API認証エラー！\n .envファイルを再確認してください")
                    .kind(MessageDialogKind::Error)
                    .title("DiscordBot API認証エラー")
                    .blocking_show();
                if res {
                    if let Err(e) = exp_shell.spawn() {
                        eprintln!("Failed to open explorer: {}", e);
                    }
                }
                return Err("failed to start bot".to_string().into());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            join,
            leave,
            get_voice_channels,
            update_volume,
            update_is_listening
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Fatal error running application: {}", e);
            std::process::exit(1);
        });
}

async fn update(app: AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    println!("download finished");
                },
            )
            .await?;
        println!("update installed");
        app.restart();
    }
    Ok(())
}
