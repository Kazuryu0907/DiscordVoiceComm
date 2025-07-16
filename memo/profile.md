# Voice Profiling Implementation

## 概要
`dis_sub.rs`でDiscord VCに送信している音声データの詳細なプロファイリング機能を実装。

## 実装内容

### 1. 新規ファイル作成

#### `src-tauri/src/vc/voice_profiler.rs`
- **VoiceProfiler**: メイン プロファイリング構造体
- **測定項目**:
  - データサイズ (バイト数)
  - 送信レート (パケット/秒)
  - キュー長 (現在/最大/平均)
  - 処理時間 (マイクロ秒)
  - 詰まりイベント回数
- **機能**:
  - 5秒間隔での自動レポート生成
  - Tauriイベントによるフロントエンドへのリアルタイム通知
  - 1000エントリまでの履歴保持

### 2. 既存ファイル修正

#### `src-tauri/src/vc.rs`
```rust
pub mod voice_profiler; // 追加
```

#### `src-tauri/src/vc/dis_sub.rs`
- インポート追加: `std::time::Instant`, `tauri::AppHandle`, `voice_profiler::VoiceProfiler`
- `join`メソッドにAppHandleパラメータ追加
- 音声送信ループに詳細プロファイリング実装:
  ```rust
  let start_time = Instant::now();
  let queue_len = rx.len();
  let data_size = d.len();
  
  // 音声処理...
  
  let processing_time = start_time.elapsed();
  profiler.record_packet(data_size, queue_len, processing_time);
  ```

#### `src-tauri/src/vc/vc_client.rs`
- `dis_sub.join()`呼び出し時にAppHandleを渡すよう修正

#### `src-tauri/src/main.rs`
- ログレベルをERRORからINFOに変更 (プロファイリング情報表示のため)

#### `src/App.tsx`
- **VoiceProfileMetrics型定義**: Rustの構造体に対応
- **VoiceProfileDashboard コンポーネント**: リアルタイムメトリクス表示
  - 送信パケット数、送信レート
  - 平均パケットサイズ
  - キュー状況 (現在長/最大長)
  - 平均処理時間、稼働時間
  - キュー詰まり警告
- **イベントリスナー**: `voice-profile-metrics`イベント受信

## プロファイリング内容

### 測定メトリクス
1. **データメトリクス**
   - タイムスタンプ
   - パケットサイズ
   - キュー長
   - 処理時間

2. **キューメトリクス**
   - 現在のキュー長
   - 最大キュー長
   - 平均キュー長
   - 詰まりイベント回数

3. **ネットワークメトリクス**
   - 送信パケット総数
   - 送信バイト総数
   - 送信レート (パケット/秒)
   - 平均パケットサイズ
   - 平均処理時間

### ログ出力
- **INFO**: 5秒間隔で統計サマリ
- **DEBUG**: 個別パケット詳細
- **WARN**: キュー詰まり検知 (20パケット超過時)

## 使用方法

1. アプリ起動
2. Discord VCに接続
3. 音声ストリーミング開始で自動プロファイリング開始
4. UIでリアルタイムメトリクス確認
5. `logfile.log`で詳細統計確認

## 技術仕様

- **測定間隔**: パケット毎
- **レポート間隔**: 5秒
- **履歴保持**: 最大1000エントリ
- **キュー警告閾値**: 10パケット (WARN: 20パケット)
- **フロントエンド更新**: Tauriイベント経由

## 期待される効果

- 音声送信の詰まり検知
- パフォーマンスボトルネック特定
- ネットワーク品質監視
- リアルタイムトラブルシューティング