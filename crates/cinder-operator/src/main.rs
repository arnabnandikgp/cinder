use std::{env, path::PathBuf, process::ExitCode};

use cinder_operator::Journal;

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let command = args.next();
    let path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".cinder-operator/journal.sqlite"));
    if args.next().is_some()
        || !matches!(
            command.as_deref().and_then(|s| s.to_str()),
            Some("init" | "status")
        )
    {
        eprintln!("Usage: cinder-operator <init|status> [journal-path]\nJournal management only; live trading is not implemented.");
        return ExitCode::from(2);
    }
    if command.as_deref().and_then(|s| s.to_str()) == Some("status") && !path.exists() {
        eprintln!("Journal does not exist; initialize it first.");
        return ExitCode::FAILURE;
    }
    let result = Journal::open(&path).and_then(|journal| journal.status());
    match result {
        Ok(status) => {
            println!(
                "Journal schema: {}; SQLite: {}",
                status.schema_version, status.sqlite_version
            );
            for (state, count) in status.operation_counts {
                println!("{state:?}: {count}");
            }
            println!("Live trading: not implemented");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
