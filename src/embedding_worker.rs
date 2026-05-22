use crate::config::AppConfig;
use crate::embedding::{EmbeddingProviderKind, create_embedding_provider, run_embedding_batch};
use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

// ---- 时间工具 ----

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ---- ActivityTracker ----

#[derive(Debug)]
pub struct ActivityTracker {
    last_activity_ms: AtomicU64,
    active_imports: AtomicUsize,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self {
            last_activity_ms: AtomicU64::new(now_ms()),
            active_imports: AtomicUsize::new(0),
        }
    }

    pub fn touch(&self) {
        self.last_activity_ms.store(now_ms(), Ordering::SeqCst);
    }

    pub fn import_started(&self) {
        self.active_imports.fetch_add(1, Ordering::SeqCst);
        self.touch();
    }

    pub fn import_finished(&self) {
        let current = self.active_imports.load(Ordering::SeqCst);
        if current == 0 {
            self.touch();
            return;
        }
        self.active_imports.fetch_sub(1, Ordering::SeqCst);
        self.touch();
    }

    pub fn active_imports(&self) -> usize {
        self.active_imports.load(Ordering::SeqCst)
    }

    pub fn millis_since_last_activity(&self) -> u64 {
        let last = self.last_activity_ms.load(Ordering::SeqCst);
        now_ms().saturating_sub(last)
    }

    pub fn is_idle(&self, idle_after_ms: u64) -> bool {
        if self.active_imports() > 0 {
            return false;
        }
        self.millis_since_last_activity() >= idle_after_ms
    }
}

// ---- ImportActivityGuard ----

pub struct ImportActivityGuard {
    activity: Arc<ActivityTracker>,
}

impl ImportActivityGuard {
    pub fn new(activity: Arc<ActivityTracker>) -> Self {
        activity.import_started();
        Self { activity }
    }
}

impl Drop for ImportActivityGuard {
    fn drop(&mut self) {
        self.activity.import_finished();
    }
}

// ---- EmbeddingWorkerConfig ----

#[derive(Debug, Clone)]
pub struct EmbeddingWorkerConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub idle_after_secs: u64,
    pub batch_limit: usize,
    pub dim: usize,
    pub provider_kind: EmbeddingProviderKind,
}

impl Default for EmbeddingWorkerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 60,
            idle_after_secs: 30,
            batch_limit: 4,
            dim: 384,
            provider_kind: EmbeddingProviderKind::Mock,
        }
    }
}

impl EmbeddingWorkerConfig {
    #[allow(dead_code)]
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(value) = std::env::var("OMNIOWN_IDLE_EMBEDDING") {
            config.enabled = parse_enabled(&value);
        }
        if let Ok(value) = std::env::var("OMNIOWN_EMBEDDING_INTERVAL_SECS")
            && let Ok(v) = value.parse::<u64>()
        {
            config.interval_secs = v.max(5);
        }
        if let Ok(value) = std::env::var("OMNIOWN_EMBEDDING_IDLE_AFTER_SECS")
            && let Ok(v) = value.parse::<u64>()
        {
            config.idle_after_secs = v;
        }
        if let Ok(value) = std::env::var("OMNIOWN_EMBEDDING_BATCH_LIMIT")
            && let Ok(v) = value.parse::<usize>()
        {
            config.batch_limit = v.clamp(1, 128);
        }
        if let Ok(value) = std::env::var("OMNIOWN_EMBEDDING_DIM")
            && let Ok(v) = value.parse::<usize>()
        {
            config.dim = v.clamp(8, 4096);
        }
        if let Ok(value) = std::env::var("OMNIOWN_EMBEDDING_PROVIDER") {
            config.provider_kind = EmbeddingProviderKind::parse(&value).unwrap_or_else(|e| {
                eprintln!("⚠️  无效的 OMNIOWN_EMBEDDING_PROVIDER: {e}, 使用默认 'mock'");
                EmbeddingProviderKind::Mock
            });
        }

        config
    }

    pub fn idle_after_ms(&self) -> u64 {
        self.idle_after_secs.saturating_mul(1000)
    }

    pub fn from_app_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.worker.enabled,
            interval_secs: (config.worker.idle_interval_ms / 1000).max(5),
            idle_after_secs: (config.worker.idle_interval_ms / 2000).max(3),
            batch_limit: config.worker.batch_size,
            dim: config.embedding.dim,
            provider_kind: config.embedding.provider,
        }
    }
}

#[allow(dead_code)]
fn parse_enabled(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "false" | "off" | "no"
    )
}

// ---- idle embedding worker ----

pub async fn run_idle_embedding_worker(
    db_path: PathBuf,
    activity: Arc<ActivityTracker>,
    config: EmbeddingWorkerConfig,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<()> {
    if !config.enabled {
        println!("🧠 idle embedding worker disabled");
        return Ok(());
    }

    println!(
        "🧠 idle embedding worker enabled: interval={}s idle_after={}s batch_limit={} provider={} dim={}",
        config.interval_secs,
        config.idle_after_secs,
        config.batch_limit,
        config.provider_kind.as_str(),
        config.dim
    );

    let running = Arc::new(AtomicBool::new(false));
    let mut interval = tokio::time::interval(Duration::from_secs(config.interval_secs));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if !activity.is_idle(config.idle_after_ms()) {
                    continue;
                }

                if running.swap(true, Ordering::SeqCst) {
                    continue;
                }

                let db_path = db_path.clone();
                let running = running.clone();
                let batch_limit = config.batch_limit;
                let provider_kind = config.provider_kind;
                let dim = config.dim;

                tokio::spawn(async move {
                    let result = tokio::task::spawn_blocking(move || -> Result<_> {
                        let conn = Connection::open(db_path)?;
                        let provider = create_embedding_provider(provider_kind, dim)?;
                        let stats = run_embedding_batch(&conn, &*provider, batch_limit)?;
                        Ok(stats)
                    })
                    .await;

                    running.store(false, Ordering::SeqCst);

                    match result {
                        Ok(Ok(stats)) => {
                            if stats.done > 0 || stats.skipped > 0 || stats.failed > 0 {
                                println!(
                                    "🧠 idle embedding: done={} skipped={} failed={}",
                                    stats.done, stats.skipped, stats.failed
                                );
                            }
                        }
                        Ok(Err(err)) => {
                            eprintln!("⚠️ idle embedding failed: {err:#}");
                        }
                        Err(err) => {
                            eprintln!("⚠️ idle embedding task join failed: {err:#}");
                        }
                    }
                });
            }

            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    println!("🧠 idle embedding worker stopped");
                    break;
                }
                if changed.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

// ---- 非重入工具 ----

#[allow(dead_code)]
pub(crate) fn try_start_run(running: &AtomicBool) -> bool {
    !running.swap(true, Ordering::SeqCst)
}

#[allow(dead_code)]
pub(crate) fn finish_run(running: &AtomicBool) {
    running.store(false, Ordering::SeqCst);
}

// ---- 测试 ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_tracker_new_is_not_idle_immediately() {
        let tracker = ActivityTracker::new();
        assert!(!tracker.is_idle(1_000));
    }

    #[test]
    fn activity_tracker_is_idle_when_threshold_is_zero() {
        let tracker = ActivityTracker::new();
        assert!(tracker.is_idle(0));
    }

    #[test]
    fn activity_tracker_active_imports_prevent_idle() {
        let tracker = ActivityTracker::new();
        tracker.import_started();
        assert!(!tracker.is_idle(0));
        tracker.import_finished();
        assert!(tracker.is_idle(0));
    }

    #[test]
    fn import_finished_does_not_underflow() {
        let tracker = ActivityTracker::new();
        tracker.import_finished();
        assert_eq!(tracker.active_imports(), 0);
    }

    #[test]
    fn parse_enabled_false_values() {
        assert!(!parse_enabled("0"));
        assert!(!parse_enabled("false"));
        assert!(!parse_enabled("off"));
        assert!(!parse_enabled("no"));
    }

    #[test]
    fn parse_enabled_true_values() {
        assert!(parse_enabled("1"));
        assert!(parse_enabled("true"));
        assert!(parse_enabled("yes"));
        assert!(parse_enabled("anything"));
    }

    #[test]
    fn embedding_worker_config_default_is_low_resource() {
        let config = EmbeddingWorkerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_secs, 60);
        assert_eq!(config.idle_after_secs, 30);
        assert_eq!(config.batch_limit, 4);
        assert_eq!(config.dim, 384);
        assert_eq!(config.provider_kind, EmbeddingProviderKind::Mock);
    }

    #[test]
    fn running_flag_prevents_overlap() {
        let running = AtomicBool::new(false);
        assert!(try_start_run(&running));
        assert!(!try_start_run(&running));
        finish_run(&running);
        assert!(try_start_run(&running));
    }

    #[test]
    fn from_app_config_maps_correctly() {
        let config = AppConfig::default();
        let wc = EmbeddingWorkerConfig::from_app_config(&config);
        assert!(wc.enabled);
        assert_eq!(wc.interval_secs, 60);
        assert_eq!(wc.idle_after_secs, 30);
        assert_eq!(wc.batch_limit, 4);
        assert_eq!(wc.dim, 384);
        assert_eq!(wc.provider_kind, EmbeddingProviderKind::Mock);
    }

    #[test]
    fn from_app_config_with_overrides() {
        use crate::config::{EmbeddingConfig, WorkerConfig};
        let mut config = AppConfig::default();
        config.worker = WorkerConfig {
            enabled: false,
            idle_interval_ms: 10_000,
            batch_size: 16,
            max_docs_per_cycle: 50,
        };
        config.embedding = EmbeddingConfig {
            provider: EmbeddingProviderKind::Local,
            dim: 768,
            max_chars_per_doc: 50_000,
        };
        let wc = EmbeddingWorkerConfig::from_app_config(&config);
        assert!(!wc.enabled);
        assert_eq!(wc.interval_secs, 10);
        assert_eq!(wc.idle_after_secs, 5);
        assert_eq!(wc.batch_limit, 16);
        assert_eq!(wc.dim, 768);
        assert_eq!(wc.provider_kind, EmbeddingProviderKind::Local);
    }
}
