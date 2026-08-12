use std::process::ExitCode;

fn main() -> ExitCode {
    match keygen_player::run_cli(std::env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("keygen: {error}");
            ExitCode::FAILURE
        }
    }
}
