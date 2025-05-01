use serenity::all::{ChannelId, GuildId};

#[derive(Clone, Copy, Debug)]
pub struct JoinInfo {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}

impl Default for JoinInfo {
    fn default() -> Self {
        JoinInfo {
            guild_id: GuildId::new(1),
            channel_id: ChannelId::new(1),
        }
    }
}

pub type VoiceSenderType = tokio::sync::mpsc::Sender<Vec<i16>>;
pub type VoiceReceiverType = tokio::sync::mpsc::Receiver<Vec<i16>>;
