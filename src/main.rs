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
    let Some(parsed) = parse_or_usage(args, &[], &["--write"], 1, "tav fmt [--write] <path>") else {
        return ExitCode::FAILURE;
    };
    let write = parsed.has("--write");
    let path = parsed.positional[0];

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
    let Some(parsed) = parse_or_usage(args, &["--schema"], &[], 1, "tav check [--schema <schema.tav>] <path>") else {
        return ExitCode::FAILURE;
    };
    let path = parsed.positional[0];

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

    let Some(schema_path) = parsed.value("--schema") else {
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
    let Some(parsed) = parse_or_usage(args, &[], &[], 1, "tav genkey <keyfile>") else {
        return ExitCode::FAILURE;
    };
    let path = parsed.positional[0];
    if let Err(e) = fs::write(path, envelope::generate_key()) {
        eprintln!("{path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn cmd_gensignkey(args: &[String]) -> ExitCode {
    let Some(parsed) = parse_or_usage(args, &[], &[], 2, "tav gensignkey <secretfile> <publicfile>") else {
        return ExitCode::FAILURE;
    };
    let (secret_path, public_path) = (parsed.positional[0], parsed.positional[1]);
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
    let Some(parsed) = parse_or_usage(
        args,
        &["--key", "--password", "--sign"],
        &["--compress"],
        2,
        "tav pack <in.tav> <out.tave> [--key <keyfile> | --password <passwordfile>] [--compress] [--sign <secretfile>]",
    ) else {
        return ExitCode::FAILURE;
    };
    let (in_path, out_path) = (parsed.positional[0], parsed.positional[1]);

    let key_path = parsed.value("--key");
    let password_path = parsed.value("--password");
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

    let sign_with = if let Some(p) = parsed.value("--sign") {
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

    let opts = SealOptions { mode, compress: parsed.has("--compress"), sign_with };
    let sealed = envelope::seal(&root, &opts);

    if let Err(e) = fs::write(out_path, sealed) {
        eprintln!("{out_path}: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn cmd_unpack(args: &[String]) -> ExitCode {
    let Some(parsed) = parse_or_usage(
        args,
        &["--key", "--password", "--verify"],
        &[],
        2,
        "tav unpack <in.tave> <out.tav> [--key <keyfile> | --password <passwordfile>] [--verify <publicfile>]",
    ) else {
        return ExitCode::FAILURE;
    };
    let (in_path, out_path) = (parsed.positional[0], parsed.positional[1]);

    let key_path = parsed.value("--key");
    let password_path = parsed.value("--password");
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

    let verify_with = if let Some(p) = parsed.value("--verify") {
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
    let Some(parsed) = parse_or_usage(
        args,
        &[],
        &[],
        2,
        "tav convert <in> <out>  (in: .tav/.tavb/.json/.toml/.yaml/.yml, out: .tav/.tavb)",
    ) else {
        return ExitCode::FAILURE;
    };
    let (in_path, out_path) = (parsed.positional[0], parsed.positional[1]);

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

struct ParsedArgs<'a> {
    positional: Vec<&'a str>,
    values: Vec<(&'a str, &'a str)>,
    bools: Vec<&'a str>,
}

impl<'a> ParsedArgs<'a> {
    fn value(&self, name: &str) -> Option<&'a str> {
        self.values.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }

    fn has(&self, name: &str) -> bool {
        self.bools.contains(&name)
    }
}

/// Any malformed arg is an error. Skipping a bad `--key` would write plaintext.
fn parse_args<'a>(
    args: &'a [String],
    value_flags: &[&str],
    bool_flags: &[&str],
    positional_count: usize,
) -> Result<ParsedArgs<'a>, String> {
    let mut parsed = ParsedArgs { positional: Vec::new(), values: Vec::new(), bools: Vec::new() };
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if value_flags.contains(&a) {
            let Some(v) = args.get(i + 1).map(String::as_str).filter(|v| !v.starts_with("--")) else {
                return Err(format!("{a} requires a value"));
            };
            if parsed.value(a).is_some() {
                return Err(format!("{a} given more than once"));
            }
            parsed.values.push((a, v));
            i += 2;
        } else if bool_flags.contains(&a) {
            if parsed.has(a) {
                return Err(format!("{a} given more than once"));
            }
            parsed.bools.push(a);
            i += 1;
        } else if a.starts_with("--") {
            return Err(format!("unknown flag {a}"));
        } else {
            parsed.positional.push(a);
            i += 1;
        }
    }
    if parsed.positional.len() != positional_count {
        return Err(format!("expected {positional_count} path argument(s), found {}", parsed.positional.len()));
    }
    Ok(parsed)
}

fn parse_or_usage<'a>(
    args: &'a [String],
    value_flags: &[&str],
    bool_flags: &[&str],
    positional_count: usize,
    usage: &str,
) -> Option<ParsedArgs<'a>> {
    match parse_args(args, value_flags, bool_flags, positional_count) {
        Ok(parsed) => Some(parsed),
        Err(e) => {
            eprintln!("{e}\nusage: {usage}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|a| (*a).to_string()).collect()
    }

    const VALUE: &[&str] = &["--key", "--password", "--sign"];
    const BOOL: &[&str] = &["--compress"];

    #[test]
    fn trailing_value_flag_without_value_is_an_error() {
        for flag in VALUE {
            let a = args(&["in.tav", "out.tave", flag]);
            assert!(parse_args(&a, VALUE, BOOL, 2).is_err(), "{flag}");
        }
    }

    #[test]
    fn value_flag_followed_by_flag_is_an_error() {
        let a = args(&["in.tav", "out.tave", "--key", "--compress"]);
        assert!(parse_args(&a, VALUE, BOOL, 2).is_err());
    }

    #[test]
    fn unknown_flag_is_an_error() {
        let a = args(&["in.tav", "out.tave", "--kye", "k"]);
        assert!(parse_args(&a, VALUE, BOOL, 2).is_err());
    }

    #[test]
    fn duplicate_flag_is_an_error() {
        let a = args(&["in.tav", "out.tave", "--key", "a", "--key", "b"]);
        assert!(parse_args(&a, VALUE, BOOL, 2).is_err());
        let a = args(&["in.tav", "out.tave", "--compress", "--compress"]);
        assert!(parse_args(&a, VALUE, BOOL, 2).is_err());
    }

    #[test]
    fn wrong_positional_count_is_an_error() {
        assert!(parse_args(&args(&["in.tav"]), VALUE, BOOL, 2).is_err());
        assert!(parse_args(&args(&["in.tav", "out.tave", "extra"]), VALUE, BOOL, 2).is_err());
    }

    #[test]
    fn well_formed_arguments_parse() {
        let a = args(&["--compress", "in.tav", "--key", "k.bin", "out.tave", "--sign", "s.bin"]);
        let p = parse_args(&a, VALUE, BOOL, 2).unwrap();
        assert_eq!(p.positional, ["in.tav", "out.tave"]);
        assert_eq!(p.value("--key"), Some("k.bin"));
        assert_eq!(p.value("--sign"), Some("s.bin"));
        assert_eq!(p.value("--password"), None);
        assert!(p.has("--compress"));
    }
}
