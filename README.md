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

## CLI

```
tav fmt [--write] <path>
tav check [--schema <schema.tav>] <path>
tav genkey <keyfile>
tav gensignkey <secretfile> <publicfile>
tav pack <in.tav> <out.tave> [--key <keyfile> | --password <passwordfile>] [--compress] [--sign <secretfile>]
tav unpack <in.tave> <out.tav> [--key <keyfile> | --password <passwordfile>] [--verify <publicfile>]
tav convert <in> <out>   # .tav/.tavb/.json/.toml/.yaml/.yml -> .tav/.tavb
```

Keys and passwords are always read from files, never passed as arguments.
Argv ends up in shell history and `ps` output; secrets don't belong there.

## Building

```
cargo build
cargo test
```

No system dependencies beyond a C compiler (for `blake3` and `zstd`).
Everything else is pure Rust.

## Status

Text parsing and formatting, the binary codec, the envelope pipeline,
schema validation, and JSON/TOML/YAML import are all working and tested
end to end.
And few things are still missing, there's no export back out to
JSON/TOML/YAML, no LSP, and the current adversarial test coverage is
hand-written with AI assisted and not fuzzed yet
