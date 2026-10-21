mod cli;
mod file_io;

fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(cli::run())
}
