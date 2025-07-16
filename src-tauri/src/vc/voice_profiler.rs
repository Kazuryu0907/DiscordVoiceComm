use std::time::{Duration, Instant};
use std::collections::VecDeque;
use log::{debug, info, warn};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct DataMetrics {
    pub timestamp: u64,
    pub data_size: usize,
    pub queue_length: usize,
    pub processing_time_micros: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueMetrics {
    pub current_length: usize,
    pub max_length: usize,
    pub avg_length: f64,
    pub queue_full_events: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMetrics {
    pub packets_sent: u64,
    pub total_bytes_sent: u64,
    pub send_rate_per_sec: f64,
    pub avg_packet_size: f64,
    pub processing_time_avg_micros: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceProfileMetrics {
    pub data: DataMetrics,
    pub queue: QueueMetrics,
    pub network: NetworkMetrics,
    pub uptime_seconds: u64,
}

pub struct VoiceProfiler {
    start_time: Instant,
    last_report_time: Instant,
    packets_sent: u64,
    total_bytes_sent: u64,
    processing_times: VecDeque<u64>,
    queue_lengths: VecDeque<usize>,
    max_queue_length: usize,
    queue_full_events: u64,
    app_handle: Option<AppHandle>,
    report_interval: Duration,
}

impl VoiceProfiler {
    pub fn new(app_handle: Option<AppHandle>) -> Self {
        Self {
            start_time: Instant::now(),
            last_report_time: Instant::now(),
            packets_sent: 0,
            total_bytes_sent: 0,
            processing_times: VecDeque::with_capacity(1000),
            queue_lengths: VecDeque::with_capacity(1000),
            max_queue_length: 0,
            queue_full_events: 0,
            app_handle,
            report_interval: Duration::from_secs(5),
        }
    }

    pub fn record_packet(&mut self, data_size: usize, queue_length: usize, processing_time: Duration) {
        let now = Instant::now();
        let processing_micros = processing_time.as_micros() as u64;
        
        // Update counters
        self.packets_sent += 1;
        self.total_bytes_sent += data_size as u64;
        
        // Store processing time (keep last 1000 entries)
        self.processing_times.push_back(processing_micros);
        if self.processing_times.len() > 1000 {
            self.processing_times.pop_front();
        }
        
        // Store queue length (keep last 1000 entries)
        self.queue_lengths.push_back(queue_length);
        if self.queue_lengths.len() > 1000 {
            self.queue_lengths.pop_front();
        }
        
        // Update max queue length
        if queue_length > self.max_queue_length {
            self.max_queue_length = queue_length;
        }
        
        // Check for queue congestion
        if queue_length > 10 {
            self.queue_full_events += 1;
            if queue_length > 20 {
                warn!("Voice queue congestion detected: {} packets queued", queue_length);
            }
        }
        
        // Create current data metrics
        let data_metrics = DataMetrics {
            timestamp: now.duration_since(self.start_time).as_millis() as u64,
            data_size,
            queue_length,
            processing_time_micros: processing_micros,
        };
        
        // Log detailed packet info (DEBUG level)
        debug!("Voice packet: size={} bytes, queue={}, processing={}μs", 
               data_size, queue_length, processing_micros);
        
        // Periodic reporting
        if now.duration_since(self.last_report_time) >= self.report_interval {
            self.generate_report();
            self.last_report_time = now;
        }
    }
    
    fn generate_report(&self) {
        let uptime = self.start_time.elapsed();
        let uptime_secs = uptime.as_secs();
        
        // Calculate averages
        let avg_processing_time = if !self.processing_times.is_empty() {
            self.processing_times.iter().sum::<u64>() as f64 / self.processing_times.len() as f64
        } else {
            0.0
        };
        
        let avg_queue_length = if !self.queue_lengths.is_empty() {
            self.queue_lengths.iter().sum::<usize>() as f64 / self.queue_lengths.len() as f64
        } else {
            0.0
        };
        
        let send_rate = if uptime_secs > 0 {
            self.packets_sent as f64 / uptime_secs as f64
        } else {
            0.0
        };
        
        let avg_packet_size = if self.packets_sent > 0 {
            self.total_bytes_sent as f64 / self.packets_sent as f64
        } else {
            0.0
        };
        
        // Create comprehensive metrics
        let metrics = VoiceProfileMetrics {
            data: DataMetrics {
                timestamp: uptime.as_millis() as u64,
                data_size: avg_packet_size as usize,
                queue_length: avg_queue_length as usize,
                processing_time_micros: avg_processing_time as u64,
            },
            queue: QueueMetrics {
                current_length: self.queue_lengths.back().copied().unwrap_or(0),
                max_length: self.max_queue_length,
                avg_length: avg_queue_length,
                queue_full_events: self.queue_full_events,
            },
            network: NetworkMetrics {
                packets_sent: self.packets_sent,
                total_bytes_sent: self.total_bytes_sent,
                send_rate_per_sec: send_rate,
                avg_packet_size,
                processing_time_avg_micros: avg_processing_time,
            },
            uptime_seconds: uptime_secs,
        };
        
        // Log summary info
        info!("Voice Profile Report: {} packets sent, {:.1} pkt/s, avg size: {:.0} bytes, avg processing: {:.1}μs, queue: {:.1} avg/{} max", 
              metrics.network.packets_sent, 
              metrics.network.send_rate_per_sec,
              metrics.network.avg_packet_size,
              metrics.network.processing_time_avg_micros,
              metrics.queue.avg_length,
              metrics.queue.max_length);
        
        // Emit to frontend if app handle is available
        if let Some(ref app) = self.app_handle {
            if let Err(e) = app.emit("voice-profile-metrics", &metrics) {
                debug!("Failed to emit voice profile metrics: {}", e);
            }
        }
    }
    
    pub fn get_current_metrics(&self) -> VoiceProfileMetrics {
        let uptime = self.start_time.elapsed();
        let uptime_secs = uptime.as_secs();
        
        let avg_processing_time = if !self.processing_times.is_empty() {
            self.processing_times.iter().sum::<u64>() as f64 / self.processing_times.len() as f64
        } else {
            0.0
        };
        
        let avg_queue_length = if !self.queue_lengths.is_empty() {
            self.queue_lengths.iter().sum::<usize>() as f64 / self.queue_lengths.len() as f64
        } else {
            0.0
        };
        
        let send_rate = if uptime_secs > 0 {
            self.packets_sent as f64 / uptime_secs as f64
        } else {
            0.0
        };
        
        let avg_packet_size = if self.packets_sent > 0 {
            self.total_bytes_sent as f64 / self.packets_sent as f64
        } else {
            0.0
        };
        
        VoiceProfileMetrics {
            data: DataMetrics {
                timestamp: uptime.as_millis() as u64,
                data_size: avg_packet_size as usize,
                queue_length: avg_queue_length as usize,
                processing_time_micros: avg_processing_time as u64,
            },
            queue: QueueMetrics {
                current_length: self.queue_lengths.back().copied().unwrap_or(0),
                max_length: self.max_queue_length,
                avg_length: avg_queue_length,
                queue_full_events: self.queue_full_events,
            },
            network: NetworkMetrics {
                packets_sent: self.packets_sent,
                total_bytes_sent: self.total_bytes_sent,
                send_rate_per_sec: send_rate,
                avg_packet_size,
                processing_time_avg_micros: avg_processing_time,
            },
            uptime_seconds: uptime_secs,
        }
    }
}