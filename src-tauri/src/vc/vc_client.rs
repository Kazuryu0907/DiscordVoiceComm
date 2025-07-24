use std::time::Duration;

use crate::vc::dis_pub::Pub;
use crate::vc::dis_sub::Sub;
use crate::vc::types::JoinInfo;
use log::error;
use serenity::{
    all::{ChannelId, GuildId, UserId},
    futures::future::join_all,
};
use tauri::AppHandle;

use super::{
    types::{PubIdentify, UserVolumesType, VoiceChannelType},
    voice_manager::VoiceManager,
};
pub struct VC {
    guild_id: GuildId,
    dis_pub: Pub,
    dis_pub2: Pub,
    dis_sub: Sub,
    voice_manager: VoiceManager,
    token: Option<String>,
}

impl VC {
    pub fn new(guild_id: GuildId, user_volumes: UserVolumesType) -> Self {
        VC {
            guild_id,
            dis_pub: Pub::new(PubIdentify::Track1),
            dis_pub2: Pub::new(PubIdentify::Track2),
            dis_sub: Sub::new(),
            voice_manager: VoiceManager::new(user_volumes),
            token: None,
        }
    }
    pub async fn start_bot(
        &mut self,
        pub_token: &str,
        pub_token2: &str,
        sub_token: &str,
    ) -> Result<(), String> {
        // spawn clients
        // APIで落ちる場合はここでエラーになる

        let mut client_sub = match self.dis_sub.create_client(sub_token).await {
            Ok(client) => client,
            Err(why) => return Err(format!("sub token error: {:?}", why)),
        };
        let mut client_pub = match self.dis_pub.create_client(pub_token).await {
            Ok(client) => client,
            Err(why) => return Err(format!("pub token error: {:?}", why)),
        };
        let mut client_pub2 = match self.dis_pub2.create_client(pub_token2).await {
            Ok(client) => client,
            Err(why) => return Err(format!("pub2 token error: {:?}", why)),
        };
        self.token = Some(pub_token.to_owned());

        tokio::spawn(async move {
            if let Err(why) = client_pub.start().await {
                error!("Err with pub client channle: {:?}", why);
            }
        });
        tokio::spawn(async move {
            if let Err(why) = client_pub2.start().await {
                error!("Err with pub2 client channel: {:?}", why);
            }
        });
        tokio::spawn(async move {
            if let Err(why) = client_sub.start().await {
                error!("Err with sub client channel: {:?}", why);
            }
        });
        Ok(())
    }
    pub async fn join(
        &self,
        app: AppHandle,
        pub_info: ChannelId,
        pub_info2: ChannelId,
        sub_info: ChannelId,
    ) {
        // Optimized channel sizing with backpressure monitoring
        let (manager_tx, manager_rx) = tokio::sync::mpsc::channel::<VoiceChannelType>(32); // Increased for better buffering
        let (vc_tx, vc_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(128); // Reduced to prevent memory buildup
        
        // Create monitoring channels for backpressure detection
        let (backpressure_tx, mut backpressure_rx) = tokio::sync::mpsc::channel::<()>(1);
        
        // Spawn backpressure monitoring task
        let vc_tx_monitor = vc_tx.clone();
        tokio::spawn(async move {
            let mut last_warning = std::time::Instant::now();
            while backpressure_rx.recv().await.is_some() {
                // Check channel capacity and warn if getting full
                if vc_tx_monitor.capacity() == 0 {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_warning) > std::time::Duration::from_secs(5) {
                        log::warn!("Voice channel at capacity - potential audio lag");
                        last_warning = now;
                    }
                }
            }
        });
        if self.token.is_none() {
            return;
        }
        let token = self.token.clone().unwrap();
        // Noneの時は上ではじいてるので，
        let futures = vec![
            self.dis_pub.join(
                JoinInfo {
                    guild_id: self.guild_id,
                    channel_id: pub_info,
                },
                manager_tx.clone(),
            ),
            self.dis_pub2.join(
                JoinInfo {
                    guild_id: self.guild_id,
                    channel_id: pub_info2,
                },
                manager_tx,
            ),
        ];
        join_all(futures).await;
        self.voice_manager.start(app, token, manager_rx, vc_tx, backpressure_tx);
        self.dis_sub
            .join(
                JoinInfo {
                    guild_id: self.guild_id,
                    channel_id: sub_info,
                },
                vc_rx,
            )
            .await;
    }

    pub async fn leave(&self) {
        let guild_id = self.guild_id;
        self.dis_pub.leave(guild_id).await.unwrap();
        self.dis_pub2.leave(guild_id).await.unwrap();
        self.dis_sub.leave(guild_id).await.unwrap();
    }

    pub async fn get_voice_channels(&self) -> Vec<serenity::all::GuildChannel> {
        loop {
            let res = self.dis_sub.get_voice_channels(self.guild_id).await;
            if let Ok(voice_channels) = res {
                return voice_channels;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn update_is_listening(&self, identify: PubIdentify, is_listening: bool) {
        match identify {
            PubIdentify::Track1 => {
                self.dis_pub.set_is_listening(is_listening).await;
            }
            PubIdentify::Track2 => {
                self.dis_pub2.set_is_listening(is_listening).await;
            }
        }
    }

    pub async fn update_volume(&self, user_id: UserId, volume: f32) {
        self.voice_manager.update_volume(user_id, volume).await;
    }
}
