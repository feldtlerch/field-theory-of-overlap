use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let start = Instant::now();
    let result = ftoo_codegen::paper::run(&std::env::args_os().skip(1).collect::<Vec<_>>());
    eprintln!("paper: wall={:.6}s", start.elapsed().as_secs_f64());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
