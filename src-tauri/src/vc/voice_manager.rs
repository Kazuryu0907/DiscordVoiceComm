use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::{self, Duration},
};

use serde::Serialize;
use serenity::model::id::UserId;
use serenity::{
    all::{Cache, GuildId, Http},
    futures::lock::Mutex,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use crate::vc::types::VoiceUserEvent;

use super::types::{PubIdentify, SendEnum, UserInfo, VoiceManagerReceiverType, VoiceSenderType};
use songbird::model::id::UserId as VoiceUserId;

fn i16tof32(pcm_data: Vec<i16>) -> Vec<f32> {
    pcm_data
        .iter()
        .map(|sample| (*sample as f32) / 32768.0)
        .collect()
}
// Vec<i16>のpcmデータからpcm f32用のVec<u8>の音声データを作成
fn convert_voice_data(data: Vec<i16>, volume: f32) -> Vec<u8> {
    let raw = i16tof32(data);
    let bytes: Vec<u8> = raw
        .iter()
        .flat_map(|&sample| (sample * volume).to_le_bytes().to_vec())
        .collect();
    bytes
}

#[derive(Serialize, Clone)]
struct EmitData {
    pub user_id: VoiceUserId,
    pub event: VoiceUserEvent,
    pub identify: PubIdentify,
    pub name: String,
}

impl EmitData {
    pub fn new(user_info: UserInfo, name: String) -> Self {
        let UserInfo {
            user_id,
            event,
            identify,
        } = user_info;
        EmitData {
            user_id,
            event,
            identify,
            name,
        }
    }
}

pub struct VoiceManager {
    // user_volumes: Arc<Mutex<HashMap<UserId, f32>>>,
    // http: Http,
    user_volumes: Arc<RwLock<HashMap<UserId, f32>>>,
    // cache:Arc<Cache>
}

impl VoiceManager {
    pub fn new() -> Self {
        let user_volumes = Arc::new(RwLock::new(HashMap::new()));
        VoiceManager {
            user_volumes
        }
    }
    // Spawn manager task
    pub fn start(
        &self,
        app: AppHandle,
        token: String,
        mut rx: VoiceManagerReceiverType,
        tx: VoiceSenderType,
    ) {
        // let http = self.http
        let user_volumes = self.user_volumes.clone();
        tokio::spawn(async move {
            let http = serenity::http::Http::new(&token);
            let id_name_map: HashMap<UserId, String> = HashMap::new();
            while let Some(d) = rx.recv().await {
                // println!("len:{}",rx.len());
                match d {
                    SendEnum::UserData(user_info) => {
                        // ~~普通に考えて，VC内で頻繁に出入りしなくない？~~
                        // 一定時間で再Hitする可能性はある
                        println!("create user_id from {:?}", user_info.user_id);
                        let user_id = UserId::new(user_info.user_id.0);
                        let user_name = match id_name_map.get(&user_id) {
                            Some(user_name) => user_name.to_owned(),
                            None => {
                                let user = http.get_user(user_id).await.unwrap();
                                user.name
                            }
                        };
                        let emit_data = EmitData::new(user_info, user_name.clone());
                        app.emit("user-data-changed", emit_data).unwrap();
                        {
                            let user_lock = user_volumes.read().await;
                            let need_insert = user_lock.get(&user_id).is_none();
                            drop(user_lock);
                            if need_insert {
                                let mut user_write = user_volumes.write().await;
                                user_write.insert(user_id.to_owned(), 1.);
                                println!("volume set {} to {}", user_name, 1.);
                            }
                        }
                        // let user_id = UserId::new(user_info.user_id.0);
                        // let user = http.get_user(user_id).await;
                        // if let Ok(user) = user {
                        //     println!("user:{}",user.name);
                        // }
                        // let member = self.http.get_member(self.guild_id, UserId::from(user_info.user_id.0)).await;
                        // app.emit("user-data-changed", user_info).unwrap();
                        // println!("user:{user_info.user_id:?} has {user_info.event:?} from {user_info.identify:?}");
                    }
                    SendEnum::VoiceData(u) => {
                        let user_volumes = user_volumes.read().await;
                        let volume = match user_volumes.get(&UserId::from(u.user_id.0)) {
                            Some(v) => *v,
                            None => {
                                unreachable!()
                            }
                        };
                        let pcm = convert_voice_data(u.voice_data, volume);
                        tx.send(pcm).await.unwrap();
                    }
                }
            }
        });
    }
    pub async fn update_volume(&self, user_id: UserId, volume: f32) {
        let user_volume = self.user_volumes.clone();
        let mut writer = user_volume.write().await;
        writer.insert(user_id, volume);
        println!("uesr:{} volume updated to {}", user_id, volume);
    }
}
