//! Optional CI observations, separate from kernel acceptance receipts.
use serde_json::json;
use std::{env, fs, io::Write, time::Instant};

pub struct Observation {
    start: Instant,
    kind: &'static str,
    configuration: String,
}

impl Observation {
    pub fn start(kind: &'static str, configuration: String) -> Self {
        Self {
            start: Instant::now(),
            kind,
            configuration,
        }
    }

    pub fn finish(self, outcome: &str) {
        let Some(directory) = env::var_os("CI_TIMING_DIR") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        let record = json!({"schema_version":1,"kind":self.kind,
            "configuration":self.configuration,"outcome":outcome,
            "wall_seconds":self.start.elapsed().as_secs_f64(),
            "process_id":std::process::id(),"observation":"executed"});
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(&directory)?;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join(format!("process-{}.jsonl", std::process::id())))?;
            writeln!(file, "{record}")
        };
        if let Err(error) = write() {
            // Reporting failure is visible, but never changes kernel pass/fail semantics.
            eprintln!("CI timing observation unavailable: {error}");
        }
    }
}
