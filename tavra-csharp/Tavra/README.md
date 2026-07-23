# Tavra

C# bindings for [Tavra](https://github.com/SkuldNorniern/tavra) — a
readable data/config format with a canonical binary form and an
encrypted, signed envelope.

This is a P/Invoke wrapper over `tavra-c`'s stable C API: parse/format
`.tav` text, encode/decode `.tavb` binary, seal/open `.tave` envelopes.
Core round-trip only — no schema or JSON/TOML/YAML import from C# yet.

```csharp
using Tavra;

using var doc = TavraDocument.Parse("name = \"demo\"\nport = 8080\n");
string text = doc.Format();
byte[] tavb = doc.Encode();

byte[] key = TavraDocument.GenerateKey();
byte[] sealedDoc = doc.SealKey(key, compress: true);
using var opened = TavraDocument.OpenKey(sealedDoc, key);
```

Built primarily for Unity (`netstandard2.1`, plain P/Invoke, no
reflection-heavy patterns — IL2CPP-AOT friendly). If you're targeting
Unity specifically, prefer the `Assets/Plugins/<platform>/` drop-in
archive from a GitHub Release over this NuGet package — Unity doesn't
consume NuGet packages the normal way.

Ships native `libtavra_c` builds for linux-x64, osx-arm64, and win-x64
under `runtimes/<rid>/native/`; .NET's native resolver picks the right
one up automatically.
