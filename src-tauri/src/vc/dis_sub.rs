use serenity::{
    all::{EventHandler, GatewayIntents, GuildChannel, GuildId, Ready},
    async_trait, Client,
};
use songbird::{
    input::{codecs::RawReader, Input, RawAdapter},
    Config, SerenityInit, Songbird,
};
use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};
use symphonia::{
    core::{codecs::CodecRegistry, probe::Probe},
    default::{codecs::PcmDecoder, register_enabled_codecs, register_enabled_formats},
};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::vc::types::JoinInfo;

use super::types::VoiceReceiverType;

static CODEC_REGISTRY: OnceLock<CodecRegistry> = OnceLock::new();
static PROBE: OnceLock<Probe> = OnceLock::new();
static CTX: OnceLock<Arc<RwLock<serenity::prelude::Context>>> = OnceLock::new();

pub struct Sub {}

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: serenity::prelude::Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        CTX.set(Arc::new(RwLock::new(ctx))).unwrap();
    }
}

impl Sub {
    pub fn new() -> Self {
        Self {}
    }
    pub async fn create_client(&self, token: &str) -> Client {
        let intents = GatewayIntents::non_privileged()
            | GatewayIntents::MESSAGE_CONTENT
            // Channelsに必要
            | GatewayIntents::GUILDS;

        Client::builder(token, intents)
            .event_handler(Handler)
            .register_songbird()
            .await
            .expect("error creating sub client")
    }
    pub async fn join(&self, join_info: JoinInfo, mut rx: VoiceReceiverType) {
        let ctx = CTX.get();
        let ctx_lock = match ctx {
            None => {
                error!("ctx None");
                return;
            }
            Some(ctx) => ctx.clone(),
        };
        let ctx = ctx_lock.read().await;
        let manager = songbird::get(&ctx).await;
        let manager = match manager {
            None => {
                error!("songbird get error");
                return;
            }
            Some(manager) => manager,
        };
        if let Ok(handler_lock) = manager.join(join_info.guild_id, join_info.channel_id).await {
            {
                let mut handler = handler_lock.lock().await;
                let config = self.create_config();
                handler.set_config(config);
            }
            let handler_lock = handler_lock.clone();
            tokio::spawn(async move {
                while let Some(d) = rx.recv().await {
                    // println!("+len:{}",rx.len());
                    let pcm = d;
                    let adapter = RawAdapter::new(Cursor::new(pcm), 48000, 2);
                    let input = Input::from(adapter);
                    // handlerをロックしないように毎回dropさせる
                    let mut handler = handler_lock.lock().await;
                    handler.play_input(input);
                }
            });
        }
    }
    pub async fn leave(&self, guild_id: GuildId) -> Result<(), String> {
        let manager = self.get_manager().await;
        let manager = match manager {
            None => {
                return Err("songbird get error".to_owned());
            }
            Some(manager) => manager,
        };
        let has_handler = manager.get(guild_id).is_some();
        if has_handler {
            if let Err(e) = manager.remove(guild_id).await {
                return Err(e.to_string());
            }
        } else {
            return Err("Not in VC".to_string());
        }
        Ok(())
    }
    pub async fn get_voice_channels(&self, guild_id: GuildId) -> Result<Vec<GuildChannel>, String> {
        let ctx = CTX.get();
        let ctx_lock = match ctx {
            None => {
                return Err("ctx None".to_owned());
            }
            Some(ctx) => ctx.clone(),
        };
        let ctx = ctx_lock.read().await;
        let guild = ctx.http.get_guild(guild_id).await.unwrap();
        let channels = guild.channels(ctx.http.clone()).await.unwrap();
        let voice_channels: Vec<GuildChannel> = channels
            .values()
            .filter(|channel| channel.bitrate.is_some())
            .cloned()
            .collect();
        Ok(voice_channels)
    }
    async fn get_manager(&self) -> Option<Arc<Songbird>> {
        let ctx = CTX.get();
        let ctx_lock = match ctx {
            None => {
                error!("ctx None");
                return None;
            }
            Some(ctx) => ctx.clone(),
        };
        let ctx = ctx_lock.read().await;

        songbird::get(&ctx).await
    }

    fn create_config(&self) -> Config {
        let codec_registry = CODEC_REGISTRY.get_or_init(|| {
            let mut registry = CodecRegistry::new();
            register_enabled_codecs(&mut registry);
            registry.register_all::<PcmDecoder>();
            registry
        });
        let probe = PROBE.get_or_init(|| {
            let mut probe = Probe::default();
            probe.register_all::<RawReader>();
            register_enabled_formats(&mut probe);
            probe
        });

        Config::default()
            .codec_registry(codec_registry)
            .format_registry(probe)
    }
}
