# Tavra

Tavra is a data and config format built to replace JSON, TOML, and YAML.
The text syntax is easy to read and write by hand, but every document also
has a canonical binary form and an optional encrypted envelope. Same value
model, all the way from a config file you edit by hand to something you
hash, sign, and ship.

```tavra
name = "demo-service"
port = 8080

server = {
    host = "0.0.0.0"
    max_connections = 1024

    # DER-encoded, not base64-in-a-string like you'd do in JSON
    tls = {
        cert = b"3082010a0282010100c1"
        enabled = true
    }
}

logging = {
    level = "info"
    outputs = ["stdout", "file"]
}
```

Nesting only ever looks like `key = { ... }`. No table headers, no
`[[array-of-tables]]` special case to memorize.

## Why another format

JSON and YAML both read badly once a file grows past a screen or two, just
for different reasons. JSON is quotes and braces and commas everywhere,
no comments, no way to write binary data or a real date without smuggling
it through a string. Miss a comma and the whole file is invalid, with an
error message that rarely points at the actual mistake. YAML goes the
other way: whitespace is meaningful, typing is implicit ("no" can quietly
become a boolean), and it's picked up a long history of parsing bugs and
outright vulnerabilities from just how much the format lets a parser
infer. TOML is the closest fit, but its table headers repeat the full
nesting path on every section, and the syntax falls apart once you need
an array of tables. Tavra borrows the readability and the comments and
skips the parts that keep causing trouble.

`.tave` documents can be compressed, encrypted, and signed. There's one
fixed cipher suite for now, not a menu of algorithms to choose from,
mostly to keep the initial implementation simple. That can be extended
later if a real use case asks for it.

## The three layers

- **`.tav`**: the text format shown above. Parses into a strict value
  model: null, bool, int, float, string, bytes, datetime, array, map.
- **`.tavb`**: canonical binary encoding of that same value model. Equal
  values always produce identical bytes, which is what makes hashing and
  signing meaningful in the first place.
- **`.tave`**: an envelope around `.tavb`, with optional zstd compression,
  optional XChaCha20-Poly1305 encryption (raw key or password via
  Argon2id), and an optional Ed25519 signature.

On top of those there's an optional schema system. A schema is just
another `.tav` document describing the shape you expect: field types,
which ones are required, enums, whether a map is closed to unlisted keys.
There's also one-way import from JSON, TOML, and YAML for migrating
existing files.

## Bindings

The core is Rust, but you don't have to be. `tavra-c`, `tavra-python`, and
`tavra-csharp` are sibling crates/projects in this same repo, each wrapping
the core round-trip (parse/format, binary encode/decode, envelope
seal/open):

- **tavra-c**: a plain C API (`tavra-c/include/tavra.h`), cdylib +
  staticlib. Everything else wraps this or the Rust crate directly.
- **tavra-python**: same API as Rust via PyO3 — the value model maps onto
  native `dict`/`list`/`str`/`bytes`/`datetime.*`. Built with `maturin`.
  Python `datetime` can't hold everything Tavra can: nanoseconds are
  truncated to microseconds, and a leap second (`:60`) raises
  `ValueError`.
- **tavra-csharp**: a `netstandard2.1` class library over `tavra-c`'s C
  ABI via P/Invoke — the round-trip surface only, aimed at Unity.

## CLI

```
tav fmt [--write] <path>
tav check [--schema <schema.tav>] <path>
tav genkey [--force] <keyfile>
tav gensignkey [--force] <secretfile> <publicfile>
tav pack <in.tav> <out.tave> [--key <keyfile> | --password <passwordfile>] [--compress] [--sign <secretfile>]
tav unpack <in.tave> <out.tav> [--key <keyfile> | --password <passwordfile>] [--verify <publicfile>]
tav convert <in> <out>   # .tav/.tavb/.json/.toml/.yaml/.yml -> .tav/.tavb
```

Keys and passwords are always read from files, never passed as arguments.
Argv ends up in shell history and `ps` output; secrets don't belong there.
Generated secret keys are written with mode 0600 on Unix, and `genkey`/
`gensignkey` refuse to overwrite an existing file without `--force`.

## Building

```
cargo build --workspace
cargo test --workspace
```

builds and tests the core crate plus `tavra-c` and `tavra-python`. No
system dependencies beyond a C compiler (for `blake3` and `zstd`).

`tavra-csharp` is a separate `.slnx` solution (dotnet SDK, not Cargo):

```
cargo build -p tavra-c   # build the native lib it P/Invokes into first
dotnet test tavra-csharp/Tavra.Tests/Tavra.Tests.csproj
```

## Status

Text parsing and formatting, the binary codec, the envelope pipeline,
schema validation, JSON/TOML/YAML import, and the C/Python/C# bindings are
all working and tested end to end.
A few things are still missing: no export back out to JSON/TOML/YAML and
no LSP.

Fuzz targets for `.tav`, `.tavb`, `.tave` and schema are in `fuzz/`
(needs nightly and cargo-fuzz):

```
cd fuzz
cargo +nightly fuzz run text     # also binary, envelope, schema
```
