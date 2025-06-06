# DiscordVoiceComm
[![License](https://img.shields.io/badge/License-MIT-green.svg)](#)
[![Rust](https://img.shields.io/badge/Rust-%23000000.svg?e&logo=rust&logoColor=white)](#)
[![Tauri](https://img.shields.io/badge/Tauri-%2324C8D8.svg?logo=tauri&logoColor=white)]()
[![Twitter badge][]][Twitter link]

**DiscordVoiceComm**はゲームの大会運営で使用することを目的とした，実況VCから選手の各VCを一方的に聞けるソフトウェアです
Rust + Tauriでできているので軽量で高速です．
![Image](https://github.com/user-attachments/assets/81887e81-7f30-4024-ac26-e3dd8d9a25bc)


# Features
- 1つの聞き手(実況VC)と2つの話し手(選手VC)を設定可能．試合中にVCの入室音を鳴らしません．
- ユーザーの音量調整機能を搭載．スライドバーで視覚的に調整でき，設定は**自動保存**されます．

# Getting Started
## 1. Discord Botの用意
[このページ](https://discordpy.readthedocs.io/ja/stable/discord.html)を参考に，Discord Botを3体作成しましょう．  
**Tokenもこの時メモしておきます．**  
各Botの`Server Members Intent`と`Message Content Intent`をONにしておきます．  
![Image](https://github.com/user-attachments/assets/ec1120b9-4ff2-442f-bdd5-de413c807097)

## 2. Configファイルを設定
Configファイルである`.env`ファイルには以下の設定事項があります．  
| 各パラメータ | 説明         | 
| ------------ | ------------ | 
| guild_id     | 大会で使用するDiscordサーバーID|
| speaker1_api | 選手VC用BotのToken | 
| speaker2_api | 選手VC用Bot2のToken| 
| listener_api | 実況VC用BotのToken | 

DiscordのサーバーIDは[公式サイト](https://support.discord.com/hc/ja/articles/206346498-%E3%83%A6%E3%83%BC%E3%82%B6%E3%83%BC-%E3%82%B5%E3%83%BC%E3%83%90%E3%83%BC-%E3%83%A1%E3%83%83%E3%82%BB%E3%83%BC%E3%82%B8ID%E3%81%AF%E3%81%A9%E3%81%93%E3%81%A7%E8%A6%8B%E3%81%A4%E3%81%91%E3%82%89%E3%82%8C%E3%82%8B)を参考に取得しましょう．  

`TOKEN_HERE`を各Token文字列に置き換えます．


[Twitter badge]: https://img.shields.io/twitter/url?label=kazuryu_rl&style=social&url=https%3A%2F%2Ftwitter.com%2Fkazuryu_rl
[Twitter link]: https://twitter.com/intent/follow?screen_name=kazuryu_rl
