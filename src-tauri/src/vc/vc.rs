use serenity::Client;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::vc::dis_pub::Pub;
use crate::vc::sub::Sub;
use crate::vc::types::JoinInfo;
pub struct VC {
    pub_token: String,
    sub_token: String,
    dis_pub: Pub,
    dis_sub: Sub,
    pub_info: JoinInfo,
    sub_info: JoinInfo,
}

impl VC {
    pub fn new(pub_token: &str, sub_token: &str) -> Self {
        VC {
            pub_token: pub_token.to_owned(),
            sub_token: sub_token.to_owned(),
            dis_pub: Pub::new(),
            dis_sub: Sub::new(),
            pub_info: JoinInfo::default(),
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
        let sub_token = self.sub_token.to_owned();
        let mut client_sub = self.dis_sub.create_client(&sub_token).await;
        let mut client_pub = self.dis_pub.create_client(&pub_token).await;
        tokio::spawn(async move {
            if let Err(why) = client_pub.start().await {
                println!("Err with pub client: {:?}", why);
            }
        });

        tokio::spawn(async move {
            if let Err(why) = client_sub.start().await {
                println!("Err with sub client: {:?}", why);
            }
        })
    }
    pub async fn join(&mut self, pub_info: JoinInfo, sub_info: JoinInfo) {
        // let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<i16>>();
        let (tx, rx) = tokio::sync::mpsc::channel::<Vec<i16>>(1);
        self.dis_pub.join(pub_info, tx).await;
        self.dis_sub.join(sub_info, rx).await;
        self.pub_info = pub_info;
        self.sub_info = sub_info;
    }

    pub async fn leave(&self) {
        self.dis_pub.leave(self.pub_info.guild_id).await.unwrap();
        self.dis_sub.leave(self.sub_info.guild_id).await.unwrap();
        // let res = vec![pub_res,sub_res].iter().collect::<Result<Vec<Result<(),String>>,String>>();
    }
}
