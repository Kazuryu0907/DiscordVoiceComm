use std::collections::HashMap;

use log::{debug, info};
use serde::Serialize;
use serenity::model::id::UserId;
use tauri::{AppHandle, Emitter};

use crate::vc::types::VoiceUserEvent;

use super::types::{
    PubIdentify, SendEnum, UserInfo, UserVolumesType, VoiceManagerReceiverType, VoiceSenderType,
};
use songbird::model::id::UserId as VoiceUserId;

// Optimized in-place conversion to avoid allocations
fn i16tof32_inplace(pcm_data: &[i16], output: &mut Vec<f32>) {
    output.clear();
    output.reserve(pcm_data.len());
    for &sample in pcm_data {
        output.push((sample as f32) / 32768.0);
    }
}
// Optimized voice data conversion with pre-allocated buffers
fn convert_voice_data(data: &[i16], volume: f32, f32_buffer: &mut Vec<f32>, output: &mut Vec<u8>) {
    // Convert i16 to f32 in-place
    i16tof32_inplace(data, f32_buffer);
    
    // Pre-allocate output buffer
    output.clear();
    output.reserve(f32_buffer.len() * 4); // f32 = 4 bytes
    
    // Apply volume and convert to bytes efficiently
    for &sample in f32_buffer {
        let adjusted_sample = sample * volume;
        output.extend_from_slice(&adjusted_sample.to_le_bytes());
    }
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
    user_volumes: UserVolumesType,
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
        backpressure_tx: tokio::sync::mpsc::Sender<()>,
    ) {
        // let http = self.http
        let user_volumes = self.user_volumes.clone();
        tokio::spawn(async move {
            let http = serenity::http::Http::new(&token);
            let mut id_name_map: HashMap<UserId, String> = HashMap::new();
            // Pre-allocate buffers for audio processing to avoid allocations in hot path
            let mut f32_buffer = Vec::<f32>::with_capacity(2048); // Pre-allocate for typical audio frame
            let mut output_buffer = Vec::<u8>::with_capacity(8192); // Pre-allocate for converted audio
            
            // User lookup cache to avoid blocking the voice processing thread
            let (user_lookup_tx, mut user_lookup_rx) = tokio::sync::mpsc::unbounded_channel::<(UserId, tokio::sync::oneshot::Sender<Option<String>>)>();
            
            // Spawn background task for user lookups
            let http_clone = http.clone();
            tokio::spawn(async move {
                while let Some((user_id, response_tx)) = user_lookup_rx.recv().await {
                    let user_name = match http_clone.get_user(user_id).await {
                        Ok(user) => Some(user.name),
                        Err(e) => {
                            log::error!("Failed to lookup user {}: {}", user_id, e);
                            None
                        }
                    };
                    let _ = response_tx.send(user_name); // Ignore if receiver dropped
                }
            });
            
            while let Some(d) = rx.recv().await {
                match d {
                    SendEnum::UserData(user_info) => {
                        // ~~普通に考えて，VC内で頻繁に出入りしなくない？~~
                        // 一定時間で再Hitする可能性はある
                        // Removed debug print for performance
                        let user_id = UserId::new(user_info.user_id.0);
                        let user_name = match id_name_map.get(&user_id) {
                            Some(user_name) => user_name.to_owned(),
                            None => {
                                // Non-blocking user lookup using background task
                                let (response_tx, response_rx) = tokio::sync::oneshot::channel();
                                if user_lookup_tx.send((user_id, response_tx)).is_err() {
                                    log::error!("User lookup channel closed");
                                    continue;
                                }
                                
                                // Use timeout to avoid blocking indefinitely
                                match tokio::time::timeout(
                                    std::time::Duration::from_millis(100),
                                    response_rx
                                ).await {
                                    Ok(Ok(Some(name))) => {
                                        // Cache the result for future use
                                        id_name_map.insert(user_id, name.clone());
                                        name
                                    },
                                    Ok(Ok(None)) => {
                                        log::warn!("User lookup failed for {}", user_id);
                                        format!("User-{}", user_id.get()) // Fallback name
                                    },
                                    _ => {
                                        log::warn!("User lookup timeout for {}", user_id);
                                        format!("User-{}", user_id.get()) // Fallback name
                                    }
                                }
                            }
                        };
                        let emit_data = EmitData::new(user_info, user_name.clone());
                        if let Err(e) = app.emit("user-data-changed", emit_data) {
                            log::error!("Failed to emit user data: {}", e);
                        }
                        // Optimized: use entry API to avoid double lookup and lock contention
                        {
                            let mut user_write = user_volumes.write().await;
                            user_write.entry(user_id).or_insert_with(|| {
                                info!("volume set {} to default 1.0", user_name);
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
                        // Optimize: minimize lock scope by extracting value immediately
                        let volume = {
                            let user_volumes_guard = user_volumes.read().await;
                            user_volumes_guard.get(&UserId::from(u.user_id.0)).copied().unwrap_or_else(|| {
                                log::warn!("No volume setting for user {}, using default 1.0", u.user_id.0);
                                1.0
                            })
                        };
                        // Use optimized conversion with pre-allocated buffers
                        convert_voice_data(&u.voice_data, volume, &mut f32_buffer, &mut output_buffer);
                        
                        // Check for backpressure before sending
                        if tx.capacity() == 0 {
                            // Signal backpressure monitoring
                            let _ = backpressure_tx.send(()).await;
                        }
                        
                        // Try to send with timeout to prevent blocking
                        match tokio::time::timeout(
                            std::time::Duration::from_millis(10),
                            tx.send(output_buffer.clone())
                        ).await {
                            Ok(Ok(())) => {}, // Successfully sent
                            Ok(Err(e)) => {
                                log::error!("Failed to send PCM data: {}", e);
                                break; // Channel is closed, exit the processing loop
                            },
                            Err(_) => {
                                log::warn!("Voice data send timeout - dropping frame to prevent blocking");
                                // Continue without sending to prevent blocking the voice processing
                            }
                        }
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
