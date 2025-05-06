use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    },
};

use dashmap::DashMap;

use serenity::{
    all::{CacheHttp, ClientBuilder, Context, GuildId, VoiceState},
    async_trait,
    client::EventHandler,
    model::gateway::Ready,
    prelude::GatewayIntents,
    Client,
};

use songbird::{
    driver::DecodeMode,
    model::{
        id::UserId,
        payload::{ClientDisconnect, Speaking},
    },
    Call, Config, CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler, SerenityInit,
    Songbird,
};
use tokio::sync::RwLock;

use crate::vc::types::{
    JoinInfo, SendEnum, UserInfo, VoiceManagerSenderType, VoiceType, VoiceUserEvent,
};

use super::types::PubIdentify;

// 複数Speakerに対応するためのHashMap
// KeyはDiscordのusername
static CTXS: LazyLock<Arc<RwLock<HashMap<String, serenity::prelude::Context>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: serenity::prelude::Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let key = &ready.user.name;
        let mut ctxs = CTXS.write().await;
        ctxs.insert(key.to_owned(), ctx);
    }
    // async fn voice_state_update(&self, ctx: serenity::prelude::Context,old: Option<VoiceState>,new: VoiceState){
    //     println!("voice state:{} old:{}",new.user_id,old.is_some());
    //     if let Some(old) = old {
    //         if new.channel_id != old.channel_id {
    //             println!("channel changed!");
    //         }
    //     }
    // }
}

#[derive(Clone)]
struct Receiver {
    inner: Arc<InnerReceiver>,
    tx: VoiceManagerSenderType,
    identify: PubIdentify,
}

struct InnerReceiver {
    last_tick_was_empty: AtomicBool,
    known_ssrcs: DashMap<u32, UserId>,
}

impl Receiver {
    pub fn new(tx: VoiceManagerSenderType, identify: PubIdentify) -> Self {
        // You can manage state here, such as a buffer of audio packet bytes so
        // you can later store them in intervals.
        Self {
            inner: Arc::new(InnerReceiver {
                last_tick_was_empty: AtomicBool::default(),
                known_ssrcs: DashMap::new(),
            }),
            tx,
            identify,
        }
    }
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    #[allow(unused_variables)]
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use EventContext as Ctx;
        match ctx {
            Ctx::SpeakingStateUpdate(Speaking {
                speaking,
                ssrc,
                user_id,
                ..
            }) => {
                // Discord voice calls use RTP, where every sender uses a randomly allocated
                // *Synchronisation Source* (SSRC) to allow receivers to tell which audio
                // stream a received packet belongs to. As this number is not derived from
                // the sender's user_id, only Discord Voice Gateway messages like this one
                // inform us about which random SSRC a user has been allocated. Future voice
                // packets will contain *only* the SSRC.
                //
                // You can implement logic here so that you can differentiate users'
                // SSRCs and map the SSRC to the User ID and maintain this state.
                // Using this map, you can map the `ssrc` in `voice_packet`
                // to the user ID and handle their audio packets separately.
                println!(
                    "Speaking state update: user {:?} has SSRC {:?}, using {:?}",
                    user_id, ssrc, speaking,
                );

                if let Some(user) = user_id {
                    self.inner.known_ssrcs.insert(*ssrc, *user);
                    let user_data = UserInfo {
                        user_id: user.to_owned(),
                        event: VoiceUserEvent::Join,
                        identify: self.identify,
                    };
                    self.tx.send(SendEnum::UserData(user_data)).await.unwrap();
                }
            }
            Ctx::VoiceTick(tick) => {
                let speaking = tick.speaking.len();
                let total_participants = speaking + tick.silent.len();
                let last_tick_was_empty = self.inner.last_tick_was_empty.load(Ordering::SeqCst);

                if speaking == 0 && !last_tick_was_empty {
                    println!("No speakers");

                    self.inner.last_tick_was_empty.store(true, Ordering::SeqCst);
                } else if speaking != 0 {
                    self.inner
                        .last_tick_was_empty
                        .store(false, Ordering::SeqCst);

                    // println!("Voice tick ({speaking}/{total_participants} live):");

                    // You can also examine tick.silent to see users who are present
                    // but haven't spoken in this tick.
                    for (ssrc, data) in &tick.speaking {
                        let user = self.inner.known_ssrcs.get(ssrc);
                        // let user_id_str = if let Some(id) = user {
                        //     format!("{:?}", *id)
                        // } else {
                        //     "?".into()
                        // };
                        // ! ここでNoneが返ってきてる
                        // * userがssrcに登録される前に来たら，早期returnする
                        let user_id = if let Some(id) = user {
                            *id
                        } else {
                            return None;
                        };

                        // This field should *always* exist under DecodeMode::Decode.
                        // The `else` allows you to see how the other modes are affected.
                        if let Some(decoded_voice) = data.decoded_voice.as_ref() {
                            let voice_len = decoded_voice.len();
                            let audio_str = format!(
                                "first samples from {}: {:?}",
                                voice_len,
                                &decoded_voice[..voice_len.min(5)]
                            );
                            // let track = Track::new_with_data(Input::Live(decoded_voice,None), user_data);
                            let pcm = decoded_voice.to_vec();
                            let send_data = VoiceType::new(user_id, pcm, self.identify.clone());
                            self.tx
                                .send(SendEnum::VoiceData(send_data))
                                .await
                                .expect("tx send failed");
                            if let Some(packet) = &data.packet {
                                let rtp = packet.rtp();
                                // println!(
                                //     "\t{ssrc}/{user_id_str}: packet seq {} ts {} -- {audio_str}",
                                //     rtp.get_sequence().0,
                                //     rtp.get_timestamp().0
                                // );
                            } else {
                                // println!("\t{ssrc}/{user_id_str}: Missed packet -- {audio_str}");
                            }
                        } else {
                            // println!("\t{ssrc}/{user_id_str}: Decode disabled.");
                        }
                    }
                }
            }
            Ctx::RtpPacket(packet) => {
                // An event which fires for every received audio packet,
                // containing the decoded data.
                // let rtp = RtpPacket::new(&packet.packet).unwrap();
                let rtp = packet.rtp();
                // let crypto_mode = CryptoMode::Aes256Gcm;
                // let payload = rtp.payload();
                // let payload_offset = crypto_mode.payload_suffix_len();
                // let payload_end_pad = payload.len() - crypto_mode.payload_suffix_len();
                // let data = &rtp.payload()[payload_offset..payload_end_pad];
                // // println!("data:{:?}",data.len());
                // // println!("payload_len:{:?}",rtp.payload().len());
                // // println!("====");
                // let tx = self.tx8.clone();
                // tx.send(data.to_vec()).expect("tx send failed");
                // println!(
                //     "Received voice packet from SSRC {}, sequence {}, timestamp {} -- {}B long",
                //     rtp.get_ssrc(),
                //     rtp.get_sequence().0,
                //     rtp.get_timestamp().0,
                //     rtp.payload().len()
                // );
            }
            Ctx::RtcpPacket(data) => {
                // An event which fires for every received rtcp packet,
                // containing the call statistics and reporting information.
                // println!("RTCP packet received: {:?}", data.packet);
            }
            Ctx::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                // You can implement your own logic here to handle a user who has left the
                // voice channel e.g., finalise processing of statistics etc.
                // You will typically need to map the User ID to their SSRC; observed when
                // first speaking.
                let user_data = UserInfo {
                    user_id: user_id.to_owned(),
                    event: VoiceUserEvent::Leave,
                    identify: self.identify,
                };
                self.tx.send(SendEnum::UserData(user_data)).await.unwrap();
                println!("Client disconnected: user {:?}", user_id);
            }
            _ => {
                // We won't be registering this struct for any more event classes.
                unimplemented!()
            }
        }

        None
    }
}

pub struct Pub {
    user_name: String,
    identify: PubIdentify,
}

impl Pub {
    pub fn new(identify: PubIdentify) -> Self {
        Pub {
            user_name: "".to_string(),
            identify,
        }
    }
    pub async fn create_client(&mut self, token: &str) -> Client {
        let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
        let songbird_config = Config::default().decode_mode(DecodeMode::Decode);

        let client = ClientBuilder::new(token, intents)
            .event_handler(Handler)
            .register_songbird_from_config(songbird_config)
            .await
            .expect("Err creating client");
        let user_name = client.http.get_current_user().await.unwrap().name.clone();
        self.user_name = user_name;
        client
    }
    pub async fn join(&self, join_info: JoinInfo, tx: VoiceManagerSenderType) {
        println!("info:{:?}", join_info);
        let manager = self.get_manager().await;
        let manager = match manager {
            None => {
                println!("songbird get error");
                return;
            }
            Some(manager) => manager,
        };
        {
            let handler_lock = manager.clone().get_or_insert(join_info.guild_id);
            let mut handler = handler_lock.lock().await;
            self.add_handler_event(&mut handler, tx.clone()).await;
        }
        self._join_vc(manager, join_info).await;
        // {
        //     let ctx = self.get_ctx().await.unwrap();
        //     let guild = ctx.http().get_guild(join_info.guild_id).await.unwrap();
        //     let channels = guild.channels(ctx.http()).await.unwrap();
        //     let channel = channels.get(&join_info.channel_id).unwrap();

        //     let my_id = ctx.cache.current_user().id;
        //     let members = channel.members(ctx.cache().unwrap()).unwrap();
        //     let member_datas: Vec<UserInfo> = members
        //         .iter()
        //         .filter(|m| m.user.id != my_id)
        //         .map(|m| UserInfo {
        //             user_id: m.user.id,
        //             user_name: m.display_name().to_owned(),
        //         })
        //         .collect();
        //     tx.send(SendEnum::UserData(member_datas)).await.unwrap();
        // }
    }
    async fn get_ctx(&self) -> Option<Context> {
        let ctx_hash_map = CTXS.read().await;
        println!("ctx key:{}", self.user_name);
        let ctx = ctx_hash_map.get(&self.user_name);
        let ctx = match ctx {
            None => None,
            Some(ctx) => Some(ctx.clone()),
        };
        ctx
    }
    async fn get_manager(&self) -> Option<Arc<Songbird>> {
        let ctx = self.get_ctx().await;
        let ctx = match ctx {
            None => {
                println!("ctx None");
                return None;
            }
            Some(ctx) => ctx,
        };

        songbird::get(&ctx).await
    }
    pub async fn leave(&self, guild_id: GuildId) -> Result<(), String> {
        let manager = self.get_manager().await;
        let manager = match manager {
            None => {
                return Err("songbird get error".to_owned());
            }
            Some(manager) => manager,
        };
        match manager.get(guild_id) {
            Some(handler_lock) => {
                // handlerのEvent初期化
                {
                    let mut handler = handler_lock.lock().await;
                    handler.remove_all_global_events();
                }
                if let Err(e) = manager.remove(guild_id).await {
                    return Err(e.to_string());
                }
            }
            None => return Err("Not in VC".to_owned()),
        }
        Ok(())
    }
    async fn add_handler_event(&self, handler: &mut Call, tx: VoiceManagerSenderType) {
        let evt_receiver = Receiver::new(tx.clone(), self.identify.clone());
        handler.add_global_event(CoreEvent::SpeakingStateUpdate.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::RtpPacket.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::RtcpPacket.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::ClientDisconnect.into(), evt_receiver.clone());
        handler.add_global_event(CoreEvent::VoiceTick.into(), evt_receiver);
    }
    async fn _join_vc(&self, manager: Arc<Songbird>, join_info: JoinInfo) {
        if let Err(e) = manager.join(join_info.guild_id, join_info.channel_id).await {
            // Although we failed to join, we need to clear out existing event handlers on the call.
            _ = manager.remove(join_info.guild_id).await;
            println!("failed to join vc:{:?}", e);
        }
    }
}
