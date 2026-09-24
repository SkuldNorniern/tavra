#![no_main]

use libfuzzer_sys::fuzz_target;
use tavra::{Value, binary, text};

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    let Ok(Value::Map(root)) = text::parse(src) else { return };

    // whatever parses has to format back to same value
    let formatted = text::format(&root);
    match text::parse(&formatted) {
        Ok(Value::Map(again)) => {
            assert_eq!(again, root, "format/parse roundtrip changed value\n{formatted}");
            assert_eq!(text::format(&again), formatted, "format is not stable");
        }
        other => panic!("formatter output did not parse: {other:?}\n{formatted}"),
    }

    let encoded = binary::encode(&root);
    assert_eq!(binary::decode(&encoded).ok().as_ref(), Some(&root), "binary roundtrip changed value");
});
