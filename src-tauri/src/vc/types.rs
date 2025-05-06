use serde::Serialize;
use serenity::all::{ChannelId, GuildId, UserId};
use songbird::model::id::UserId as VoiceUserId;
#[derive(Clone, Copy, Debug)]
pub struct JoinInfo {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}

// pub struct UserInfo {
//     pub user_id: UserId,
//     pub user_name: String,
// }

impl Default for JoinInfo {
    fn default() -> Self {
        JoinInfo {
            guild_id: GuildId::new(1),
            channel_id: ChannelId::new(1),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum PubIdentify {
    Track1,
    Track2,
}

pub struct VoiceType {
    pub user_id: VoiceUserId,
    pub voice_data: Vec<i16>,
    pub identify: PubIdentify,
}
impl VoiceType {
    pub fn new(user_id: VoiceUserId, voice_data: Vec<i16>, identify: PubIdentify) -> Self {
        VoiceType {
            user_id,
            voice_data,
            identify,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum VoiceUserEvent {
    Join,
    Leave,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserInfo {
    pub user_id: VoiceUserId,
    pub event: VoiceUserEvent,
    pub identify: PubIdentify,
}

pub enum SendEnum {
    UserData(UserInfo),
    VoiceData(VoiceType),
}

pub type VoiceChannelType = SendEnum;
pub type VoiceManagerSenderType = tokio::sync::mpsc::Sender<VoiceChannelType>;
pub type VoiceManagerReceiverType = tokio::sync::mpsc::Receiver<VoiceChannelType>;
pub type VoiceSenderType = tokio::sync::mpsc::Sender<Vec<u8>>;
pub type VoiceReceiverType = tokio::sync::mpsc::Receiver<Vec<u8>>;
