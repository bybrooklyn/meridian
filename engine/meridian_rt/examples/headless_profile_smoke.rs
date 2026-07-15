//! Exercises the minimal runtime profile without selecting Meridian UI.

use std::process::ExitCode;
use std::time::Duration;

use meridian_rt::EngineRuntime;

fn main() -> ExitCode {
    let mut runtime = EngineRuntime::default();
    let report = runtime.advance(Duration::from_millis(16));
    if runtime.render_snapshot().is_none() || report.render_extraction_error().is_some() {
        eprintln!("Meridian headless runtime profile failed to publish a render snapshot");
        return ExitCode::FAILURE;
    }
    println!(
        "Meridian headless runtime profile passed: frame {} advanced without selecting a UI package or UI task",
        report.frame_id()
    );
    ExitCode::SUCCESS
}
