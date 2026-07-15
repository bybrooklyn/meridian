use std::process::ExitCode;

use meridian_editor::{run, usage, MeridianArgumentError, MeridianOptions};

fn main() -> ExitCode {
    let options = match MeridianOptions::parse(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(MeridianArgumentError::HelpRequested) => {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("Meridian argument error: {error}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };
    match run(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Meridian failed: {error}");
            ExitCode::FAILURE
        }
    }
}
