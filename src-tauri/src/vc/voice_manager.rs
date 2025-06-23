use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, info};
use lru::LruCache;
use once_cell::sync::Lazy;
use serde::Serialize;
use serenity::model::id::UserId;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::vc::types::VoiceUserEvent;
use crate::vc::metrics::{METRICS, AudioProcessingTimer};

use super::types::{
    PubIdentify, SendEnum, UserInfo, UserVolumesType, VoiceManagerReceiverType, VoiceSenderType,
};
use songbird::model::id::UserId as VoiceUserId;

// バッファプール: 音声データ用の再利用可能バッファ
struct BufferPool {
    buffers: Vec<Vec<u8>>,
}

impl BufferPool {
    fn new() -> Self {
        Self {
            buffers: Vec::with_capacity(64), // 初期容量
        }
    }

    fn get_buffer(&mut self, min_capacity: usize) -> Vec<u8> {
        // 適切なサイズのバッファを探す
        if let Some(pos) = self.buffers.iter().position(|buf| buf.capacity() >= min_capacity) {
            let mut buffer = self.buffers.swap_remove(pos);
            buffer.clear(); // データをクリアするが容量は保持
            METRICS.record_buffer_pool_get(true); // 再利用
            buffer
        } else {
            // 適切なバッファがない場合は新規作成
            METRICS.record_buffer_pool_get(false); // 新規作成
            Vec::with_capacity(min_capacity.max(1024)) // 最小1KB確保
        }
    }

    fn return_buffer(&mut self, buffer: Vec<u8>) {
        if buffer.capacity() > 0 && self.buffers.len() < 128 { // 最大128個まで保持
            self.buffers.push(buffer);
            METRICS.record_buffer_pool_return();
        }
        // 容量がない、またはプールが満杯の場合は破棄（統計は記録しない）
    }
}

// グローバルバッファプール
static BUFFER_POOL: Lazy<Arc<Mutex<BufferPool>>> = Lazy::new(|| Arc::new(Mutex::new(BufferPool::new())));

// ユーザー名キャッシュ: LRU実装
static USER_NAME_CACHE: Lazy<Arc<Mutex<LruCache<UserId, String>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(LruCache::new(std::num::NonZeroUsize::new(1000).unwrap()))));

// Vec<i16>のpcmデータからpcm f32用のVec<u8>の音声データを作成
// 最適化: バッファプール使用 + 単一パスでi16→f32→u8変換 + パフォーマンス計測
async fn convert_voice_data(data: Vec<i16>, volume: f32) -> Vec<u8> {
    let _timer = AudioProcessingTimer::start(); // 処理時間を自動計測
    
    let required_capacity = data.len() * 4; // f32 = 4 bytes
    
    // バッファプールから再利用可能なバッファを取得
    let mut bytes = {
        let mut pool = BUFFER_POOL.lock().await;
        pool.get_buffer(required_capacity)
    };
    
    for sample in data {
        // i16→f32変換と音量調整を同時に実行
        let f32_sample = (sample as f32 / 32768.0) * volume;
        bytes.extend_from_slice(&f32_sample.to_le_bytes());
    }
    
    bytes
}

// バッファをプールに返却するヘルパー関数
async fn return_buffer_to_pool(buffer: Vec<u8>) {
    let mut pool = BUFFER_POOL.lock().await;
    pool.return_buffer(buffer);
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
    user_volumes: UserVolumesType,
    // cache:Arc<Cache>
}

impl VoiceManager {
    pub fn new(user_volumes: UserVolumesType) -> Self {
        let user_volumes = user_volumes.clone();
        VoiceManager { user_volumes }
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
            while let Some(d) = rx.recv().await {
                match d {
                    SendEnum::UserData(user_info) => {
                        // ~~普通に考えて，VC内で頻繁に出入りしなくない？~~
                        // 一定時間で再Hitする可能性はある
                        debug!("create user_id from {:?}", user_info.user_id);
                        let user_id = UserId::new(user_info.user_id.0);
                        
                        // 最適化: LRUキャッシュからユーザー名を取得 + 統計記録
                        let user_name = {
                            let mut cache = USER_NAME_CACHE.lock().await;
                            if let Some(cached_name) = cache.get(&user_id) {
                                METRICS.record_cache_hit();
                                cached_name.clone()
                            } else {
                                METRICS.record_cache_miss();
                                // キャッシュにない場合のみHTTP APIを呼び出し
                                drop(cache); // lockを早期解放
                                match http.get_user(user_id).await {
                                    Ok(user) => {
                                        METRICS.record_http_call();
                                        let name = user.name.clone();
                                        // キャッシュに保存
                                        let mut cache = USER_NAME_CACHE.lock().await;
                                        cache.put(user_id, name.clone());
                                        name
                                    }
                                    Err(e) => {
                                        METRICS.record_http_call(); // 失敗もHTTP呼び出しとしてカウント
                                        debug!("Failed to get user name for {}: {:?}", user_id, e);
                                        format!("User#{}", user_id.get())
                                    }
                                }
                            }
                        };
                        let emit_data = EmitData::new(user_info, user_name.clone());
                        app.emit("user-data-changed", emit_data).unwrap();
                        // 最適化: entry APIを使用して二重ロックを回避
                        {
                            let mut user_write = user_volumes.write().await;
                            user_write.entry(user_id).or_insert_with(|| {
                                info!("volume set {} to {}", user_name, 1.0);
                                1.0
                            });
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
                        METRICS.record_voice_packet_received();
                        let user_volumes = user_volumes.read().await;
                        let volume = match user_volumes.get(&UserId::from(u.user_id.0)) {
                            Some(v) => *v,
                            None => {
                                unreachable!()
                            }
                        };
                        let pcm = convert_voice_data(u.voice_data, volume).await;
                        tx.send(pcm).await.unwrap();
                        METRICS.record_voice_packet_processed();
                    }
                }
            }
        });
    }
    pub async fn update_volume(&self, user_id: UserId, volume: f32) {
        let user_volume = self.user_volumes.clone();
        let mut writer = user_volume.write().await;
        writer.insert(user_id, volume);
        info!("uesr:{} volume updated to {}", user_id, volume);
    }
}
