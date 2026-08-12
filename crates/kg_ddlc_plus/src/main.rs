use std::process::ExitCode;

fn main() -> ExitCode {
    match kg_ddlc_plus::run_cli(std::env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("kg-ddlc-plus: {error}");
            ExitCode::FAILURE
        }
    }
}
