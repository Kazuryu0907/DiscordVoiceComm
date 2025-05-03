use serenity::Client;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::vc::dis_pub::Pub;
use crate::vc::sub::Sub;
use crate::vc::types::JoinInfo;
pub struct VC {
    pub_token: String,
    pub_token2: String,
    sub_token: String,
    dis_pub: Pub,
    dis_pub2: Pub,
    dis_sub: Sub,
    pub_info: JoinInfo,
    pub_info2: JoinInfo,
    sub_info: JoinInfo,
}

impl VC {
    pub fn new(pub_token: &str,pub_token2: &str, sub_token: &str) -> Self {
        VC {
            pub_token: pub_token.to_owned(),
            pub_token2: pub_token2.to_owned(),
            sub_token: sub_token.to_owned(),
            dis_pub: Pub::new(),
            dis_pub2: Pub::new(),
            dis_sub: Sub::new(),
            pub_info: JoinInfo::default(),
            pub_info2: JoinInfo::default(),
            sub_info: JoinInfo::default(),
        }
    }
    pub fn event_handler(&self, mut rx: UnboundedReceiver<String>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(d) = rx.recv().await {
                println!("receive: {}", d);
            }
        })
    }
    pub async fn start_bot(&mut self) -> tokio::task::JoinHandle<()>{
        // spawn clients
        let pub_token = self.pub_token.to_owned();
        let pub_token2 = self.pub_token2.to_owned();
        let sub_token = self.sub_token.to_owned();
        let mut client_sub = self.dis_sub.create_client(&sub_token).await;
        let mut client_pub = self.dis_pub.create_client(&pub_token).await;
        let mut client_pub2 = self.dis_pub2.create_client(&pub_token2).await;
        tokio::spawn(async move {
            if let Err(why) = client_pub.start().await {
                println!("Err with pub client: {:?}", why);
            }
        });
        tokio::spawn(async move{
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
    pub async fn join(&mut self, pub_info: JoinInfo,pub_info2:JoinInfo, sub_info: JoinInfo) {
        // let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<i16>>();
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<i16>>(16);
        self.dis_pub.join(pub_info, tx.clone()).await;
        self.dis_pub2.join(pub_info2, tx).await;
        self.dis_sub.join(sub_info, rx).await;
        self.pub_info = pub_info;
        self.pub_info2 = pub_info2;
        self.sub_info = sub_info;
    }

    pub async fn leave(&self) {
        self.dis_pub.leave(self.pub_info.guild_id).await.unwrap();
        self.dis_pub2.leave(self.pub_info2.guild_id).await.unwrap();
        self.dis_sub.leave(self.sub_info.guild_id).await.unwrap();
    }
}
