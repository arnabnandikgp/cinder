use std::{env, path::PathBuf, process::ExitCode};

use cinder_operator::{Journal, OperatorRuntime, RuntimeConfig};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let command = args.next();
    let execute = command.as_deref().and_then(|s| s.to_str()) == Some("execute");
    if execute || command.as_deref().and_then(|s| s.to_str()) == Some("recover") {
        let Some(config) = args.next().map(PathBuf::from) else {
            eprintln!("Usage: cinder-operator <recover|execute> <config.json> [journal-path]");
            return ExitCode::from(2);
        };
        let path = args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".cinder-operator/journal.sqlite"));
        if args.next().is_some() {
            return ExitCode::from(2);
        }
        let result = RuntimeConfig::read(&config)
            .and_then(|config| {
                if execute {
                    OperatorRuntime::open_execution(config, &path)
                } else {
                    OperatorRuntime::open(config, &path)
                }
            })
            .inspect_err(|_| {
                eprintln!("Operator setup failed");
            })
            .and_then(|mut runtime| {
                let recovery = runtime.recover().inspect_err(|_| {
                    eprintln!("Operator pass failed");
                });
                // Neither one-pass command is an autonomous service. Leave
                // ownership gates closed before releasing the process lease.
                let halted = runtime.halt();
                match (recovery, halted) {
                    (Ok(report), Ok(())) => Ok(report),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            });
        return match result {
            Ok(report) => {
                println!(
                    "Recovery: {:?}; unresolved operations: {}",
                    report.reason, report.unresolved_operations
                );
                println!("Process stopped: OPERATOR_DOWN; execution pass: {execute}");
                if report.unresolved_operations == 0 && report.reason.is_none() {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                }
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        };
    }
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
        eprintln!("Usage: cinder-operator <init|status> [journal-path]\n       cinder-operator <recover|execute> <config.json> [journal-path]\nExecute requires explicit solvency and execution policies.");
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
            println!("Trading: opt-in execute command; status does not connect to a venue");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
