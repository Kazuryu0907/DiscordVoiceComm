# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 開発コマンド

### フロントエンド (React + TypeScript + Vite)
- `bun run dev` - 開発サーバー起動 (localhost:1420)
- `bun run build` - TypeScriptコンパイル後にViteビルド実行
- `bun run preview` - ビルド結果のプレビュー

### Tauri (Rust バックエンド)
- `bun run tauri dev` - Tauriアプリの開発モード起動 (フロントエンドとバックエンドの両方)
- `bun run tauri build` - 本番用バイナリ作成 (Windows exe, macOS app, Linux AppImage等)

### 設定ファイル
- `.env` - Discord Bot API トークンと Guild ID を設定 (必須)
  - `guild_id`: Discord サーバーID
  - `speaker1_api`: 選手VC用Bot1のToken
  - `speaker2_api`: 選手VC用Bot2のToken
  - `listener_api`: 実況VC用BotのToken

## アーキテクチャ概要

### 全体構成
- **フロントエンド**: React + TypeScript + Tailwind CSS + shadcn/ui
- **バックエンド**: Rust + Tauri + Serenity (Discord API) + Songbird (音声処理)
- **アプリケーション種別**: デスクトップアプリ (Windows/macOS/Linux対応)

### 主要機能
1. **Discord VC音声リレー**: 実況VCから選手VCの音声を一方的に聞く
2. **音量調整**: ユーザー毎の音量調整とリアルタイム反映
3. **自動アップデート**: GitHub Releases経由でのアプリ更新

### Rustバックエンド構造 (src-tauri/src/)
- `lib.rs` - Tauriコマンド定義とアプリ初期化
- `main.rs` - エントリーポイント、ログ設定
- `vc/` - 音声処理関連モジュール
  - `vc_client.rs` - VCクライアントのメイン制御
  - `voice_manager.rs` - 音声データ処理と音量調整
  - `dis_pub.rs` / `dis_sub.rs` - Discord Bot Publisher/Subscriber
  - `config.rs` - 設定ファイル管理 (.env読み込み、音量設定保存)
  - `types.rs` - 共通型定義

### フロントエンド構造 (src/)
- `App.tsx` - メインコンポーネント、VC選択とユーザー音量調整UI
- `components/ui/` - shadcn/uiベースのUIコンポーネント

### 音声処理フロー
1. Discord Botが指定VCに接続
2. 選手VCの音声をSongbirdで受信
3. リアルタイム音量調整とPCM変換
4. 実況VCへリアルタイム送信

### 状態管理
- Rustサイド: `Arc<RwLock<HashMap>>` によるユーザー音量状態
- Reactサイド: `useState`とTauriイベントによる状態同期

### 設定の永続化
- ユーザー音量設定: `confy`クレートでローカル設定ファイルに自動保存
- Discord Bot設定: `.env`ファイルで管理

### 開発時の注意点
- Rust側の変更時は`tauri dev`の再起動が必要
- Discord Bot API認証エラー時はエクスプローラーが開き`.env`確認を促す
- ログファイル: `logfile.log`, `stderr.log`にRust側のログ出力

### ツール設定
- `timeout 600000ms` - 10分間のタイムアウト設定