#![no_main]

use bongterm_term::WezTermAdapter;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut adapter = WezTermAdapter::new(80, 24);
    adapter.ingest_bytes(data);
    let _ = adapter.current_snapshot();
});
