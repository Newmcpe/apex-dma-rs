use std::time::Instant;

use glam::Mat4;
use log::info;
use tokio::sync::watch::Sender;
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::core::reader::GameReader;
use crate::entities::Entity;
use crate::types::Snapshot;

pub struct SamplerConfig {
    pub base_tick_ms: u64,
    pub max_entries: usize,
    pub full_refresh_every_n: u32,
}

struct Cache {
    bases: Vec<u64>,
    entities: Vec<Entity>,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            bases: Vec::new(),
            entities: Vec::new(),
        }
    }
}

pub async fn spawn_sampler<R: GameReader + Send + Sync + 'static>(
    reader: R,
    local_player: u64,
    cfg: SamplerConfig,
    tx: Sender<Snapshot>,
) {
    tokio::spawn(async move {
        let mut tick = interval(Duration::from_millis(cfg.base_tick_ms));
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut cache = Cache::default();
        let mut counter: u32 = 0;

        let mut last_report = Instant::now();
        let mut sum_view_ms: f64 = 0.0;
        let mut sum_full_ms: f64 = 0.0;
        let mut cycles: usize = 0;

        loop {
            tick.tick().await;

            let t0 = Instant::now();
            let view: Mat4 = reader.read_view_matrix().unwrap_or(Mat4::IDENTITY);
            let t1 = Instant::now();

            counter = counter.wrapping_add(1);
            if counter % cfg.full_refresh_every_n == 0 || cache.bases.is_empty() {
                // Full refresh
                let bases = reader
                    .gather_entity_bases(cfg.max_entries, local_player)
                    .unwrap_or_default();
                let entities = reader.read_entities(&bases).unwrap_or_default();
                cache.bases = bases;
                cache.entities = entities;
            } else {
                // Fast path: origins only if we have bases
                if !cache.bases.is_empty() && !cache.entities.is_empty() {
                    if let Ok(origins) = reader.read_entity_origins(&cache.bases) {
                        let len = origins.len().min(cache.entities.len());
                        for i in 0..len {
                            cache.entities[i].origin = origins[i];
                        }
                    }
                }
            }

            let t2 = Instant::now();
            sum_view_ms += (t1 - t0).as_secs_f64() * 1000.0;
            sum_full_ms += (t2 - t1).as_secs_f64() * 1000.0;
            cycles = cycles.saturating_add(1);

            let aabbs = reader.read_entity_aabbs(&cache.bases).unwrap_or_default();

            println!("aabbs: {:?}", aabbs);
            let _ = tx.send_replace(Snapshot {
                view,
                entities: cache.entities.clone(),
                aabbs,
            });

            if last_report.elapsed() >= Duration::from_secs(1) && cycles > 0 {
                let avg_view_ms = sum_view_ms / cycles as f64;
                let avg_full_ms = sum_full_ms / cycles as f64;
                let approx_hz = cycles as f64 / last_report.elapsed().as_secs_f64().max(1.0);
                info!(
                    "Sampler ~1s: cycles={}, avg view={:.2} ms, avg entity pass={:.2} ms (~{:.2} Hz)",
                    cycles, avg_view_ms, avg_full_ms, approx_hz
                );
                sum_view_ms = 0.0;
                sum_full_ms = 0.0;
                cycles = 0;
                last_report = Instant::now();
            }
        }
    });
}
