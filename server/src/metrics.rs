//! Optional per-tick profiling, enabled by NETWORK_ALPHA_METRICS (JSONL path).
use serde::Serialize;
use std::{
    fs::File,
    io::{BufWriter, Write},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub struct TickMetrics {
    output: BufWriter<File>,
    start: Instant,
    pub sequence: u64,
    observer_ms: f64,
}

#[derive(Default, Serialize)]
pub struct TickSample {
    pub flow_fields: usize,
    pub buffers: utils::buffer::BufferStats,
    pub allocations: crate::allocations::TickAllocations,
    pub unix_time_ms: u128,
    pub observer_ms_previous_tick: f64,
    pub sequence: u64,
    pub elapsed_seconds: f64,
    pub tick_interval_ms: f64,
    pub network_receive_ms: f64,
    pub commands_ms: f64,
    pub simulation_ms: f64,
    pub events_ms: f64,
    pub snapshots_ms: f64,
    pub network_send_ms: f64,
    pub cleanup_ms: f64,
    pub processing_ms: f64,
    pub over_budget: bool,
    pub allocated_entities: usize,
    pub active_entities: usize,
    pub players: usize,
    pub queued_events: usize,
    pub network: crate::net::server::NetworkCounters,
}

impl TickMetrics {
    pub fn from_env() -> std::io::Result<Option<Self>> {
        match std::env::var_os("NETWORK_ALPHA_METRICS") {
            Some(path) => Ok(Some(Self {
                output: BufWriter::new(File::create(path)?),
                start: Instant::now(),
                sequence: 0,
                observer_ms: 0.0,
            })),
            None => Ok(None),
        }
    }

    pub fn record(&mut self, mut sample: TickSample) -> std::io::Result<()> {
        let observer_start = Instant::now();
        sample.unix_time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        sample.observer_ms_previous_tick = self.observer_ms;
        sample.sequence = self.sequence;
        sample.elapsed_seconds = self.start.elapsed().as_secs_f64();
        self.sequence += 1;
        serde_json::to_writer(&mut self.output, &sample)?;
        self.output.write_all(b"\n")?;
        // Preserve the last complete sample even if the workload terminates the server.
        self.output.flush()?;
        self.observer_ms = observer_start.elapsed().as_secs_f64() * 1000.0;
        Ok(())
    }
}
