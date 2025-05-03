use std::time::Duration;

use crate::vc::dis_pub::Pub;
use crate::vc::dis_sub::Sub;
use crate::vc::types::JoinInfo;
use serenity::{
    all::{ChannelId, GuildId},
    futures::future::join_all,
};
pub struct VC {
    guild_id: GuildId,
    dis_pub: Pub,
    dis_pub2: Pub,
    dis_sub: Sub,
}

impl VC {
    pub fn new(guild_id: GuildId) -> Self {
        VC {
            guild_id,
            dis_pub: Pub::new(),
            dis_pub2: Pub::new(),
            dis_sub: Sub::new(),
        }
    }
    pub async fn start_bot(
        &mut self,
        pub_token: &str,
        pub_token2: &str,
        sub_token: &str,
    ) -> tokio::task::JoinHandle<()> {
        // spawn clients
        let mut client_sub = self.dis_sub.create_client(sub_token).await;
        let mut client_pub = self.dis_pub.create_client(pub_token).await;
        let mut client_pub2 = self.dis_pub2.create_client(pub_token2).await;
        tokio::spawn(async move {
            if let Err(why) = client_pub.start().await {
                println!("Err with pub client: {:?}", why);
            }
        });
        tokio::spawn(async move {
            if let Err(why) = client_pub2.start().await {
                println!("Err with pub2 client: {:?}", why);
            }
        });
        tokio::spawn(async move {
            if let Err(why) = client_sub.start().await {
                println!("Err with sub client: {:?}", why);
            }
        })
    }
    pub async fn join(&self, pub_info: ChannelId, pub_info2: ChannelId, sub_info: ChannelId) {
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<i16>>(16);
        let futures = vec![
            self.dis_pub.join(
                JoinInfo {
                    guild_id: self.guild_id,
                    channel_id: pub_info,
                },
                tx.clone(),
            ),
            self.dis_pub2.join(
                JoinInfo {
                    guild_id: self.guild_id,
                    channel_id: pub_info2,
                },
                tx,
            ),
        ];
        join_all(futures).await;
        self.dis_sub
            .join(
                JoinInfo {
                    guild_id: self.guild_id,
                    channel_id: sub_info,
                },
                rx,
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
}
