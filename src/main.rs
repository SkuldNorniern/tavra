use std::{env, fs};
use std::process::ExitCode;

use tavra::text;
use tavra::Value;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("fmt") => cmd_fmt(&args[1..]),
        Some("check") => cmd_check(&args[1..]),
        _ => {
            eprintln!("usage: tav <fmt|check> [--write] <path>");
            ExitCode::FAILURE
        }
    }
}

fn cmd_fmt(args: &[String]) -> ExitCode {
    let write = args.iter().any(|a| a == "--write");
    let Some(path) = args.iter().find(|a| *a != "--write") else {
        eprintln!("usage: tav fmt [--write] <path>");
        return ExitCode::FAILURE;
    };

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let root = match text::parse(&source) {
        Ok(Value::Map(root)) => root,
        Ok(_) => unreachable!("document root is always a map"),
        Err(e) => {
            eprintln!("{path}:{e}");
            return ExitCode::FAILURE;
        }
    };

    let formatted = text::format(&root);

    if write {
        if let Err(e) = fs::write(path, &formatted) {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    } else {
        print!("{formatted}");
    }
    ExitCode::SUCCESS
}

fn cmd_check(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: tav check <path>");
        return ExitCode::FAILURE;
    };

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    match text::parse(&source) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{path}:{e}");
            ExitCode::FAILURE
        }
    }
}
