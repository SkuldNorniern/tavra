#![no_main]

use libfuzzer_sys::fuzz_target;
use tavra::{Value, binary, text};

fuzz_target!(|data: &[u8]| {
    let Ok(root) = binary::decode(data) else { return };

    // decoder only takes canonical input, so encoding it again gives same bytes
    assert_eq!(binary::encode(&root), data, "decoded non-canonical bytes");

    let formatted = text::format(&root);
    match text::parse(&formatted) {
        Ok(Value::Map(again)) => assert_eq!(again, root, "text roundtrip changed value\n{formatted}"),
        other => panic!("formatter output did not parse: {other:?}\n{formatted}"),
    }
});
