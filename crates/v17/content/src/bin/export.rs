//! Writes the 1.7.10 content export.
//!
//! `cargo run -p sunset-1-7-content --bin export -- [DIR]`
//! writes into `DIR`, `.local/sunrise-client/1.7.10` by default.

use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(sunset_1_7_content::DEFAULT_DIR));
    if args.next().is_some() {
        eprintln!("usage: export [DIR]");
        return ExitCode::from(2);
    }
    match sunset_1_7_content::export(&dir) {
        Ok(summary) => {
            println!(
                "wrote {} states across {} blocks and {} items to {}",
                summary.states,
                summary.blocks,
                summary.items,
                dir.display()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}: {error}", dir.display());
            ExitCode::FAILURE
        }
    }
}
