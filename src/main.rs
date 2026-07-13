use std::{env, fs};
use std::process::ExitCode;

use tavra::envelope::{self, OpenMode, SealMode, SealOptions};
use tavra::{binary, convert, schema, text};
use tavra::{Map, Value};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("fmt") => cmd_fmt(&args[1..]),
        Some("check") => cmd_check(&args[1..]),
        Some("genkey") => cmd_genkey(&args[1..]),
        Some("gensignkey") => cmd_gensignkey(&args[1..]),
        Some("pack") => cmd_pack(&args[1..]),
        Some("unpack") => cmd_unpack(&args[1..]),
        Some("convert") => cmd_convert(&args[1..]),
        _ => {
            eprintln!(
                "usage:\n  \
                 tav fmt [--write] <path>\n  \
                 tav check [--schema <schema.tav>] <path>\n  \
                 tav genkey <keyfile>\n  \
                 tav gensignkey <secretfile> <publicfile>\n  \
                 tav pack <in.tav> <out.tave> [--key <keyfile> | --password <passwordfile>] [--compress] [--sign <secretfile>]\n  \
                 tav unpack <in.tave> <out.tav> [--key <keyfile> | --password <passwordfile>] [--verify <publicfile>]\n  \
                 tav convert <in> <out>  (in: .tav/.tavb/.json/.toml/.yaml/.yml, out: .tav/.tavb)"
            );
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
    let positional = positionals(args, &["--schema"], &[]);
    let Some(path) = positional.first() else {
        eprintln!("usage: tav check [--schema <schema.tav>] <path>");
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

    let Some(schema_path) = find_flag_value(args, "--schema") else {
        return ExitCode::SUCCESS;
    };

    let schema_source = match fs::read_to_string(schema_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{schema_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let schema_root = match text::parse(&schema_source) {
        Ok(Value::Map(root)) => root,
        Ok(_) => unreachable!("document root is always a map"),
        Err(e) => {
            eprintln!("{schema_path}:{e}");
            return ExitCode::FAILURE;
        }
    };

    match schema::validate(&schema_root, &root) {
        Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
        Ok(violations) => {
            for v in violations {
                eprintln!("{path}: {v}");
            }
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("{schema_path}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_genkey(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("usage: tav genkey <keyfile>");
        return ExitCode::FAILURE;
    };
    if let Err(e) = fs::write(path, envelope::generate_key()) {
        eprintln!("{path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn cmd_gensignkey(args: &[String]) -> ExitCode {
    let (Some(secret_path), Some(public_path)) = (args.first(), args.get(1)) else {
        eprintln!("usage: tav gensignkey <secretfile> <publicfile>");
        return ExitCode::FAILURE;
    };
    let (secret, public) = envelope::generate_signing_key();
    if let Err(e) = fs::write(secret_path, secret) {
        eprintln!("{secret_path}: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = fs::write(public_path, public) {
        eprintln!("{public_path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn cmd_pack(args: &[String]) -> ExitCode {
    let positional = positionals(args, &["--key", "--password", "--sign"], &["--compress"]);
    let (Some(in_path), Some(out_path)) = (positional.first(), positional.get(1)) else {
        eprintln!(
            "usage: tav pack <in.tav> <out.tave> [--key <keyfile> | --password <passwordfile>] [--compress] [--sign <secretfile>]"
        );
        return ExitCode::FAILURE;
    };

    let key_path = find_flag_value(args, "--key");
    let password_path = find_flag_value(args, "--password");
    if key_path.is_some() && password_path.is_some() {
        eprintln!("--key and --password are mutually exclusive");
        return ExitCode::FAILURE;
    }

    let source = match fs::read_to_string(in_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{in_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let root = match text::parse(&source) {
        Ok(Value::Map(root)) => root,
        Ok(_) => unreachable!("document root is always a map"),
        Err(e) => {
            eprintln!("{in_path}:{e}");
            return ExitCode::FAILURE;
        }
    };

    let key_bytes;
    let password_bytes;
    let sign_bytes;

    let mode = if let Some(p) = key_path {
        key_bytes = match read_key_file::<32>(p) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
        SealMode::Key(&key_bytes)
    } else if let Some(p) = password_path {
        password_bytes = match fs::read(p) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{p}: {e}");
                return ExitCode::FAILURE;
            }
        };
        SealMode::Password(&password_bytes)
    } else {
        SealMode::None
    };

    let sign_with = if let Some(p) = find_flag_value(args, "--sign") {
        sign_bytes = match read_key_file::<32>(p) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
        Some(&sign_bytes)
    } else {
        None
    };

    let opts = SealOptions { mode, compress: has_flag(args, "--compress"), sign_with };
    let sealed = envelope::seal(&root, &opts);

    if let Err(e) = fs::write(out_path, sealed) {
        eprintln!("{out_path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn cmd_unpack(args: &[String]) -> ExitCode {
    let positional = positionals(args, &["--key", "--password", "--verify"], &[]);
    let (Some(in_path), Some(out_path)) = (positional.first(), positional.get(1)) else {
        eprintln!("usage: tav unpack <in.tave> <out.tav> [--key <keyfile> | --password <passwordfile>] [--verify <publicfile>]");
        return ExitCode::FAILURE;
    };

    let key_path = find_flag_value(args, "--key");
    let password_path = find_flag_value(args, "--password");
    if key_path.is_some() && password_path.is_some() {
        eprintln!("--key and --password are mutually exclusive");
        return ExitCode::FAILURE;
    }

    let bytes = match fs::read(in_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{in_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let key_bytes;
    let password_bytes;
    let verify_bytes;

    let mode = if let Some(p) = key_path {
        key_bytes = match read_key_file::<32>(p) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
        OpenMode::Key(&key_bytes)
    } else if let Some(p) = password_path {
        password_bytes = match fs::read(p) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{p}: {e}");
                return ExitCode::FAILURE;
            }
        };
        OpenMode::Password(&password_bytes)
    } else {
        OpenMode::None
    };

    let verify_with = if let Some(p) = find_flag_value(args, "--verify") {
        verify_bytes = match read_key_file::<32>(p) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        };
        Some(&verify_bytes)
    } else {
        None
    };

    let root = match envelope::open(&bytes, &mode, verify_with) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{in_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let formatted = text::format(&root);
    if let Err(e) = fs::write(out_path, formatted) {
        eprintln!("{out_path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn cmd_convert(args: &[String]) -> ExitCode {
    let (Some(in_path), Some(out_path)) = (args.first(), args.get(1)) else {
        eprintln!("usage: tav convert <in> <out>  (in: .tav/.tavb/.json/.toml/.yaml/.yml, out: .tav/.tavb)");
        return ExitCode::FAILURE;
    };

    let root = match read_as_map(in_path) {
        Ok(root) => root,
        Err(e) => {
            eprintln!("{in_path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let write_result = if out_path.ends_with(".tavb") {
        fs::write(out_path, binary::encode(&root))
    } else if out_path.ends_with(".tav") {
        fs::write(out_path, text::format(&root))
    } else {
        eprintln!("{out_path}: unsupported output extension (use .tav or .tavb)");
        return ExitCode::FAILURE;
    };

    if let Err(e) = write_result {
        eprintln!("{out_path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Reads `path` and parses it into a document root, dispatching on file
/// extension. `.tav`/`.tavb` go through this crate's own parser/decoder;
/// `.json`/`.toml`/`.yaml`/`.yml` go through the corresponding import
/// adapter (`tavra::convert`) — import only, there is no export back to
/// those formats.
fn read_as_map(path: &str) -> Result<Map, String> {
    if path.ends_with(".tav") {
        let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
        return match text::parse(&source) {
            Ok(Value::Map(root)) => Ok(root),
            Ok(_) => unreachable!("document root is always a map"),
            Err(e) => Err(e.to_string()),
        };
    }
    if path.ends_with(".tavb") {
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        return binary::decode(&bytes).map_err(|e| e.to_string());
    }
    if path.ends_with(".json") {
        let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
        return convert::from_json(&source).map_err(|e| e.to_string());
    }
    if path.ends_with(".toml") {
        let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
        return convert::from_toml(&source).map_err(|e| e.to_string());
    }
    if path.ends_with(".yaml") || path.ends_with(".yml") {
        let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
        return convert::from_yaml(&source).map_err(|e| e.to_string());
    }
    Err("unsupported input extension (use .tav/.tavb/.json/.toml/.yaml/.yml)".to_string())
}

/// Reads a fixed-size key/secret from a file — key material comes from
/// files rather than argv so it never lands in shell history or `ps`
/// output.
fn read_key_file<const N: usize>(path: &str) -> Result<[u8; N], String> {
    let bytes = fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    bytes.try_into().map_err(|v: Vec<u8>| format!("{path}: expected {N} bytes, found {}", v.len()))
}

fn find_flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn positionals<'a>(args: &'a [String], value_flags: &[&str], bool_flags: &[&str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if value_flags.contains(&a) {
            i += 2;
        } else if bool_flags.contains(&a) {
            i += 1;
        } else {
            out.push(a);
            i += 1;
        }
    }
    out
}
