use std::num::NonZeroUsize;
use std::time::Duration;

use meridian_core::FrameRate;
use meridian_diagnostics::{
    BenchmarkMetadata, BenchmarkMetrics, BenchmarkResult, FrameBudget, FrameHistory, FrameSample,
    PipelineDiagnostics,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rate = FrameRate::new(60).expect("60 FPS is non-zero");
    let mut history = FrameHistory::new(
        NonZeroUsize::new(120).expect("history capacity is non-zero"),
        FrameBudget::for_rate(rate),
    );
    for milliseconds in [15, 16, 17, 16, 18, 16, 15, 16] {
        history.push(
            FrameSample::new(
                Duration::from_millis(milliseconds),
                Duration::from_millis(4),
            )
            .with_pipeline_diagnostics(PipelineDiagnostics::new(1, 1, 1, 1, 0, true)),
        );
    }

    let summary = history.summary().expect("synthetic history has samples");
    let metadata =
        BenchmarkMetadata::new("clear-frame", "workspace", "none", "synthetic", "default")
            .with_context("none", "none", "none");
    let metrics =
        BenchmarkMetrics::from_summary(summary, Duration::from_millis(4), Duration::from_millis(3));

    println!("{}", BenchmarkResult::new(metadata, metrics).to_json()?);
    Ok(())
}
