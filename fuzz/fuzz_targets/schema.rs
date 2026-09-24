#![no_main]

use libfuzzer_sys::fuzz_target;
use tavra::{Value, schema, text};

// input is "<schema>\0<document>", both as .tav text
fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    let Some((schema_src, doc_src)) = src.split_once('\0') else { return };
    let (Ok(Value::Map(schema_root)), Ok(Value::Map(doc))) = (text::parse(schema_src), text::parse(doc_src)) else {
        return;
    };
    let _ = schema::validate(&schema_root, &doc);
});
