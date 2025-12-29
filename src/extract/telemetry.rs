//! Telemetry module for API usage statistics and error analysis
//!
//! Provides:
//! - API call count tracking
//! - Token usage monitoring
//! - Latency measurement
//! - Error categorization and logging

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// API usage statistics
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    /// Total number of API calls
    pub total_calls: u64,
    /// Successful API calls
    pub successful_calls: u64,
    /// Failed API calls
    pub failed_calls: u64,
    /// Total tokens used (prompt + completion)
    pub total_tokens: u64,
    /// Total prompt tokens
    pub prompt_tokens: u64,
    /// Total completion tokens
    pub completion_tokens: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Maximum latency in milliseconds
    pub max_latency_ms: u64,
    /// Minimum latency in milliseconds
    pub min_latency_ms: u64,
}

impl UsageStats {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            (self.successful_calls as f64 / self.total_calls as f64) * 100.0
        }
    }
}

/// Error category for pattern analysis
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Network connectivity errors
    Network,
    /// Authentication failures
    Authentication,
    /// Rate limiting
    RateLimit,
    /// Model not found
    ModelNotFound,
    /// Invalid request
    InvalidRequest,
    /// Server errors
    ServerError,
    /// Response parsing errors
    ParseError,
    /// Timeout errors
    Timeout,
    /// Unknown errors
    Unknown,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCategory::Network => write!(f, "Network"),
            ErrorCategory::Authentication => write!(f, "Authentication"),
            ErrorCategory::RateLimit => write!(f, "RateLimit"),
            ErrorCategory::ModelNotFound => write!(f, "ModelNotFound"),
            ErrorCategory::InvalidRequest => write!(f, "InvalidRequest"),
            ErrorCategory::ServerError => write!(f, "ServerError"),
            ErrorCategory::ParseError => write!(f, "ParseError"),
            ErrorCategory::Timeout => write!(f, "Timeout"),
            ErrorCategory::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Error record for analysis
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    /// Error category
    pub category: ErrorCategory,
    /// Error message
    pub message: String,
    /// When the error occurred
    pub timestamp: Instant,
    /// Model that caused the error (if applicable)
    pub model: Option<String>,
}

/// Telemetry collector
pub struct TelemetryCollector {
    /// Usage statistics
    stats: Arc<RwLock<UsageStats>>,
    /// Error records (last N errors)
    errors: Arc<RwLock<Vec<ErrorRecord>>>,
    /// Error counts by category
    error_counts: Arc<RwLock<HashMap<ErrorCategory, u64>>>,
    /// Maximum errors to keep in memory
    max_errors: usize,
    /// Latency samples for averaging
    latency_samples: Arc<RwLock<Vec<u64>>>,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new(100)
    }
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new(max_errors: usize) -> Self {
        Self {
            stats: Arc::new(RwLock::new(UsageStats::default())),
            errors: Arc::new(RwLock::new(Vec::new())),
            error_counts: Arc::new(RwLock::new(HashMap::new())),
            max_errors,
            latency_samples: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a successful API call
    pub fn record_success(
        &self,
        prompt_tokens: usize,
        completion_tokens: usize,
        latency: Duration,
    ) {
        let mut stats = self.stats.write().unwrap();
        stats.total_calls += 1;
        stats.successful_calls += 1;
        stats.prompt_tokens += prompt_tokens as u64;
        stats.completion_tokens += completion_tokens as u64;
        stats.total_tokens += (prompt_tokens + completion_tokens) as u64;

        let latency_ms = latency.as_millis() as u64;

        // Update latency stats
        if stats.min_latency_ms == 0 || latency_ms < stats.min_latency_ms {
            stats.min_latency_ms = latency_ms;
        }
        if latency_ms > stats.max_latency_ms {
            stats.max_latency_ms = latency_ms;
        }

        drop(stats);

        // Update average latency
        let mut samples = self.latency_samples.write().unwrap();
        samples.push(latency_ms);
        let avg = samples.iter().sum::<u64>() as f64 / samples.len() as f64;
        drop(samples);

        let mut stats = self.stats.write().unwrap();
        stats.avg_latency_ms = avg;
    }

    /// Record a failed API call
    pub fn record_failure(&self, category: ErrorCategory, message: String, model: Option<String>) {
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.total_calls += 1;
            stats.failed_calls += 1;
        }

        // Update error counts
        {
            let mut counts = self.error_counts.write().unwrap();
            *counts.entry(category.clone()).or_insert(0) += 1;
        }

        // Add error record
        {
            let mut errors = self.errors.write().unwrap();
            errors.push(ErrorRecord {
                category,
                message,
                timestamp: Instant::now(),
                model,
            });

            // Keep only last N errors
            if errors.len() > self.max_errors {
                errors.remove(0);
            }
        }
    }

    /// Get current usage statistics
    pub fn get_stats(&self) -> UsageStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    /// Get error counts by category
    pub fn get_error_counts(&self) -> HashMap<ErrorCategory, u64> {
        let counts = self.error_counts.read().unwrap();
        counts.clone()
    }

    /// Get recent errors
    pub fn get_recent_errors(&self, limit: usize) -> Vec<ErrorRecord> {
        let errors = self.errors.read().unwrap();
        errors.iter().rev().take(limit).cloned().collect()
    }

    /// Reset all statistics
    pub fn reset(&self) {
        *self.stats.write().unwrap() = UsageStats::default();
        self.errors.write().unwrap().clear();
        self.error_counts.write().unwrap().clear();
        self.latency_samples.write().unwrap().clear();
    }

    /// Generate a summary report
    pub fn generate_report(&self) -> String {
        let stats = self.get_stats();
        let error_counts = self.get_error_counts();

        let mut report = String::new();
        report.push_str("=== API Usage Report ===\n\n");

        report.push_str(&format!("Total Calls: {}\n", stats.total_calls));
        report.push_str(&format!("Success Rate: {:.1}%\n", stats.success_rate()));
        report.push_str(&format!(
            "Successful: {} | Failed: {}\n\n",
            stats.successful_calls, stats.failed_calls
        ));

        report.push_str("Token Usage:\n");
        report.push_str(&format!("  Total: {}\n", stats.total_tokens));
        report.push_str(&format!("  Prompt: {}\n", stats.prompt_tokens));
        report.push_str(&format!("  Completion: {}\n\n", stats.completion_tokens));

        report.push_str("Latency:\n");
        report.push_str(&format!("  Average: {:.1}ms\n", stats.avg_latency_ms));
        report.push_str(&format!("  Min: {}ms\n", stats.min_latency_ms));
        report.push_str(&format!("  Max: {}ms\n\n", stats.max_latency_ms));

        if !error_counts.is_empty() {
            report.push_str("Error Breakdown:\n");
            for (category, count) in error_counts.iter() {
                report.push_str(&format!("  {}: {}\n", category, count));
            }
        }

        report
    }
}

/// Global telemetry instance
static TELEMETRY: once_cell::sync::Lazy<TelemetryCollector> =
    once_cell::sync::Lazy::new(TelemetryCollector::default);

/// Get the global telemetry collector
pub fn telemetry() -> &'static TelemetryCollector {
    &TELEMETRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_stats_default() {
        let stats = UsageStats::default();
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_usage_stats_success_rate() {
        let stats = UsageStats {
            total_calls: 100,
            successful_calls: 75,
            failed_calls: 25,
            ..Default::default()
        };
        assert!((stats.success_rate() - 75.0).abs() < 0.1);
    }

    #[test]
    fn test_telemetry_collector_new() {
        let collector = TelemetryCollector::new(50);
        let stats = collector.get_stats();
        assert_eq!(stats.total_calls, 0);
    }

    #[test]
    fn test_record_success() {
        let collector = TelemetryCollector::new(100);
        collector.record_success(100, 50, Duration::from_millis(500));

        let stats = collector.get_stats();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.successful_calls, 1);
        assert_eq!(stats.prompt_tokens, 100);
        assert_eq!(stats.completion_tokens, 50);
        assert_eq!(stats.total_tokens, 150);
    }

    #[test]
    fn test_record_failure() {
        let collector = TelemetryCollector::new(100);
        collector.record_failure(
            ErrorCategory::RateLimit,
            "Rate limit exceeded".to_string(),
            Some("test-model".to_string()),
        );

        let stats = collector.get_stats();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.failed_calls, 1);

        let counts = collector.get_error_counts();
        assert_eq!(counts.get(&ErrorCategory::RateLimit), Some(&1));
    }

    #[test]
    fn test_error_category_display() {
        assert_eq!(ErrorCategory::Network.to_string(), "Network");
        assert_eq!(ErrorCategory::RateLimit.to_string(), "RateLimit");
    }

    #[test]
    fn test_latency_tracking() {
        let collector = TelemetryCollector::new(100);

        collector.record_success(10, 10, Duration::from_millis(100));
        collector.record_success(10, 10, Duration::from_millis(200));
        collector.record_success(10, 10, Duration::from_millis(300));

        let stats = collector.get_stats();
        assert_eq!(stats.min_latency_ms, 100);
        assert_eq!(stats.max_latency_ms, 300);
        assert!((stats.avg_latency_ms - 200.0).abs() < 0.1);
    }

    #[test]
    fn test_reset() {
        let collector = TelemetryCollector::new(100);
        collector.record_success(100, 50, Duration::from_millis(500));
        collector.record_failure(ErrorCategory::Network, "Test".to_string(), None);

        collector.reset();

        let stats = collector.get_stats();
        assert_eq!(stats.total_calls, 0);
        assert!(collector.get_error_counts().is_empty());
    }

    #[test]
    fn test_generate_report() {
        let collector = TelemetryCollector::new(100);
        collector.record_success(100, 50, Duration::from_millis(500));
        collector.record_failure(ErrorCategory::RateLimit, "Rate limit".to_string(), None);

        let report = collector.generate_report();
        assert!(report.contains("Total Calls: 2"));
        assert!(report.contains("Success Rate: 50.0%"));
        assert!(report.contains("RateLimit: 1"));
    }

    #[test]
    fn test_recent_errors_limit() {
        let collector = TelemetryCollector::new(5);

        for i in 0..10 {
            collector.record_failure(ErrorCategory::Network, format!("Error {}", i), None);
        }

        let recent = collector.get_recent_errors(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_global_telemetry() {
        let t = telemetry();
        // Just verify it's accessible
        let _ = t.get_stats();
    }
}
