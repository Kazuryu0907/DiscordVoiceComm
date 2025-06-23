# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

DiscordVoiceCommは、ゲーム大会運営で使用することを目的としたTauriアプリケーションです。実況VCから選手の各VCを一方的に聞くことができます。

## アーキテクチャ

**フロントエンド (React + TypeScript)**
- `src/` - Reactアプリケーション
- `src/components/ui/` - UI コンポーネント（button、select、slider）
- Tailwind CSS + Radix UI使用

**バックエンド (Rust + Tauri)**
- `src-tauri/src/` - Rustバックエンド
- `src-tauri/src/vc/` - Discord音声通信のコアロジック
  - `vc_client.rs` - メインのVCクライアント
  - `voice_manager.rs` - 音声管理
  - `dis_pub.rs` / `dis_sub.rs` - DiscordのPub/Subクライアント
  - `config.rs` - 設定管理
  - `types.rs` - 型定義

**依存関係**
- Serenity - Discord API
- Songbird - Discord音声処理
- confy - 設定ファイル管理

## 開発コマンド

**フロントエンド**
```bash
npm run dev          # 開発サーバー起動
npm run build        # TypeScriptコンパイル + Viteビルド
npm run preview      # プレビューサーバー
```

**Tauri**
```bash
npm run tauri dev    # Tauriアプリ開発モード起動
npm run tauri build  # アプリケーションビルド
```

**Rust**
```bash
cd src-tauri
cargo build          # Rustプロジェクトビルド
cargo run            # Rustアプリ実行
```

## 設定ファイル

`.env`ファイルが必要（プロジェクトルートに配置）:
```
guild_id=YOUR_GUILD_ID
speaker1_api=BOT_TOKEN_1
speaker2_api=BOT_TOKEN_2
listener_api=BOT_TOKEN_3
```

## 重要な機能

- **音声ミキシング**: 複数のDiscordボットから音声を受信し、リアルタイムでミキシング
- **音量制御**: ユーザーごとの音量調整（設定は自動保存）
- **自動アップデート**: tauri-plugin-updaterによる自動更新機能
- **エラーハンドリング**: API認証エラー時のダイアログ表示

## パフォーマンス計測

**計測機能**
- 音声処理時間（平均・最大値）の自動計測
- バッファプール効率（ヒット率・アロケーション数）
- ユーザー名キャッシュ効率（ヒット率・HTTP呼び出し数）
- 音声パケット処理統計
- 1分間隔でのJSON形式ログ出力

**計測制御**
```bash
# 計測機能有効（デフォルト）
cargo build --features metrics

# 計測機能無効（リリース用）
cargo build --no-default-features
```

## 開発時の注意点

- Discordボットが3体必要（選手VC用2体、実況VC用1体）
- 各ボットには`Server Members Intent`と`Message Content Intent`が必要
- 音声処理はRustで実行、UIはTypeScript/React
- ログファイルは`logfile.log`と`stderr.log`に出力される
- パフォーマンス統計は`logfile.log`に1分間隔で出力される