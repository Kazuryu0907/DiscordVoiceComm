use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;
use serde::Serialize;
use log::info;
use tokio::time::{interval, Duration as TokioDuration};

/// パフォーマンスメトリクス: 音声処理アプリケーションの性能統計
#[derive(Debug)]
pub struct PerformanceMetrics {
    // 音声処理関連
    pub audio_processing_time_total: AtomicU64,      // マイクロ秒単位
    pub audio_processing_count: AtomicU64,
    pub audio_processing_max_time: AtomicU64,        // マイクロ秒単位
    
    // バッファプール関連
    pub buffer_pool_gets: AtomicU64,
    pub buffer_pool_hits: AtomicU64,                 // 再利用されたバッファ数
    pub buffer_pool_creates: AtomicU64,              // 新規作成されたバッファ数
    pub buffer_pool_returns: AtomicU64,
    
    // ユーザー名キャッシュ関連
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub cache_http_calls: AtomicU64,
    
    // 音声パケット処理関連
    pub voice_packets_received: AtomicU64,
    pub voice_packets_processed: AtomicU64,
    
    // メモリ関連
    pub memory_allocations: AtomicU64,
    
    // 開始時刻
    start_time: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            audio_processing_time_total: AtomicU64::new(0),
            audio_processing_count: AtomicU64::new(0),
            audio_processing_max_time: AtomicU64::new(0),
            
            buffer_pool_gets: AtomicU64::new(0),
            buffer_pool_hits: AtomicU64::new(0),
            buffer_pool_creates: AtomicU64::new(0),
            buffer_pool_returns: AtomicU64::new(0),
            
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_http_calls: AtomicU64::new(0),
            
            voice_packets_received: AtomicU64::new(0),
            voice_packets_processed: AtomicU64::new(0),
            
            memory_allocations: AtomicU64::new(0),
            
            start_time: Instant::now(),
        }
    }
    
    /// 音声処理時間を記録
    pub fn record_audio_processing_time(&self, duration: Duration) {
        #[cfg(feature = "metrics")]
        {
            let micros = duration.as_micros() as u64;
            self.audio_processing_time_total.fetch_add(micros, Ordering::Relaxed);
            self.audio_processing_count.fetch_add(1, Ordering::Relaxed);
            
            // 最大時間を更新
            let mut max_time = self.audio_processing_max_time.load(Ordering::Relaxed);
            while micros > max_time {
                match self.audio_processing_max_time.compare_exchange_weak(
                    max_time, micros, Ordering::Relaxed, Ordering::Relaxed
                ) {
                    Ok(_) => break,
                    Err(current) => max_time = current,
                }
            }
        }
    }
    
    /// バッファプール取得を記録
    pub fn record_buffer_pool_get(&self, was_reused: bool) {
        #[cfg(feature = "metrics")]
        {
            self.buffer_pool_gets.fetch_add(1, Ordering::Relaxed);
            if was_reused {
                self.buffer_pool_hits.fetch_add(1, Ordering::Relaxed);
            } else {
                self.buffer_pool_creates.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    /// バッファプール返却を記録
    pub fn record_buffer_pool_return(&self) {
        #[cfg(feature = "metrics")]
        {
            self.buffer_pool_returns.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// キャッシュヒットを記録
    pub fn record_cache_hit(&self) {
        #[cfg(feature = "metrics")]
        {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// キャッシュミスを記録
    pub fn record_cache_miss(&self) {
        #[cfg(feature = "metrics")]
        {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// HTTP API呼び出しを記録
    pub fn record_http_call(&self) {
        #[cfg(feature = "metrics")]
        {
            self.cache_http_calls.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// 音声パケット受信を記録
    pub fn record_voice_packet_received(&self) {
        #[cfg(feature = "metrics")]
        {
            self.voice_packets_received.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// 音声パケット処理を記録
    pub fn record_voice_packet_processed(&self) {
        #[cfg(feature = "metrics")]
        {
            self.voice_packets_processed.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// メモリアロケーションを記録
    pub fn record_memory_allocation(&self) {
        #[cfg(feature = "metrics")]
        {
            self.memory_allocations.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// 統計情報のスナップショットを取得
    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let uptime = self.start_time.elapsed();
        
        let audio_count = self.audio_processing_count.load(Ordering::Relaxed);
        let audio_total_micros = self.audio_processing_time_total.load(Ordering::Relaxed);
        let audio_avg_micros = if audio_count > 0 { audio_total_micros / audio_count } else { 0 };
        
        let cache_total = self.cache_hits.load(Ordering::Relaxed) + self.cache_misses.load(Ordering::Relaxed);
        let cache_hit_rate = if cache_total > 0 { 
            (self.cache_hits.load(Ordering::Relaxed) as f64 / cache_total as f64) * 100.0 
        } else { 0.0 };
        
        let buffer_gets = self.buffer_pool_gets.load(Ordering::Relaxed);
        let buffer_hit_rate = if buffer_gets > 0 {
            (self.buffer_pool_hits.load(Ordering::Relaxed) as f64 / buffer_gets as f64) * 100.0
        } else { 0.0 };
        
        MetricsSnapshot {
            uptime_secs: uptime.as_secs(),
            
            audio_processing_avg_micros: audio_avg_micros,
            audio_processing_max_micros: self.audio_processing_max_time.load(Ordering::Relaxed),
            audio_processing_count: audio_count,
            
            buffer_pool_hit_rate_percent: buffer_hit_rate,
            buffer_pool_gets: buffer_gets,
            buffer_pool_creates: self.buffer_pool_creates.load(Ordering::Relaxed),
            
            cache_hit_rate_percent: cache_hit_rate,
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            cache_http_calls: self.cache_http_calls.load(Ordering::Relaxed),
            
            voice_packets_received: self.voice_packets_received.load(Ordering::Relaxed),
            voice_packets_processed: self.voice_packets_processed.load(Ordering::Relaxed),
            
            memory_allocations: self.memory_allocations.load(Ordering::Relaxed),
        }
    }
}

/// 統計情報のスナップショット（JSON出力用）
#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    
    // 音声処理統計
    pub audio_processing_avg_micros: u64,
    pub audio_processing_max_micros: u64,
    pub audio_processing_count: u64,
    
    // バッファプール統計
    pub buffer_pool_hit_rate_percent: f64,
    pub buffer_pool_gets: u64,
    pub buffer_pool_creates: u64,
    
    // キャッシュ統計
    pub cache_hit_rate_percent: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_http_calls: u64,
    
    // 音声パケット統計
    pub voice_packets_received: u64,
    pub voice_packets_processed: u64,
    
    // メモリ統計
    pub memory_allocations: u64,
}

/// グローバルメトリクスインスタンス
pub static METRICS: Lazy<PerformanceMetrics> = Lazy::new(|| PerformanceMetrics::new());

/// 音声処理時間の計測用ガード
pub struct AudioProcessingTimer {
    #[cfg(feature = "metrics")]
    start: Instant,
}

impl AudioProcessingTimer {
    pub fn start() -> Self {
        Self {
            #[cfg(feature = "metrics")]
            start: Instant::now(),
        }
    }
}

impl Drop for AudioProcessingTimer {
    fn drop(&mut self) {
        #[cfg(feature = "metrics")]
        {
            let duration = self.start.elapsed();
            METRICS.record_audio_processing_time(duration);
        }
    }
}

/// 定期的な統計出力を開始
pub fn start_metrics_reporting() {
    #[cfg(feature = "metrics")]
    {
        tokio::spawn(async {
            let mut interval = interval(TokioDuration::from_secs(60)); // 1分間隔
            
            loop {
                interval.tick().await;
                let snapshot = METRICS.get_snapshot();
                
                match serde_json::to_string(&snapshot) {
                    Ok(json) => {
                        info!("Performance Metrics: {}", json);
                    }
                    Err(e) => {
                        log::error!("Failed to serialize metrics: {}", e);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_audio_processing_timer() {
        let metrics = PerformanceMetrics::new();
        
        {
            let _timer = AudioProcessingTimer::start();
            thread::sleep(Duration::from_millis(10));
        }
        
        let snapshot = metrics.get_snapshot();
        assert!(snapshot.audio_processing_count > 0);
        assert!(snapshot.audio_processing_avg_micros > 0);
    }
    
    #[test]
    fn test_buffer_pool_metrics() {
        let metrics = PerformanceMetrics::new();
        
        metrics.record_buffer_pool_get(true);  // hit
        metrics.record_buffer_pool_get(false); // miss
        metrics.record_buffer_pool_return();
        
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.buffer_pool_gets, 2);
        assert_eq!(snapshot.buffer_pool_creates, 1);
        assert_eq!(snapshot.buffer_pool_hit_rate_percent, 50.0);
    }
    
    #[test]
    fn test_cache_metrics() {
        let metrics = PerformanceMetrics::new();
        
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.record_http_call();
        
        let snapshot = metrics.get_snapshot();
        assert_eq!(snapshot.cache_hits, 2);
        assert_eq!(snapshot.cache_misses, 1);
        assert_eq!(snapshot.cache_http_calls, 1);
        assert!((snapshot.cache_hit_rate_percent - 66.67).abs() < 0.1);
    }
}