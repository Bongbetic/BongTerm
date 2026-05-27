//! Criterion bench: parser MB/s on synthetic mixed ANSI/UTF-8 output.
//!
//! Phase 0 baseline: `WezTermAdapter::ingest_bytes` is a scaffold stub that
//! does nothing. The numbers here are the floor; real parser work lands in
//! Phase 1 task 1.B.3. CI gate uses these as lower-bound regression guards.

use bongterm_term::WezTermAdapter;
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

fn payload(mb: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(mb * 1024 * 1024);
    let line = b"Lorem ipsum \x1b[31mdolor\x1b[0m sit amet, \x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\\n";
    while v.len() + line.len() < v.capacity() {
        v.extend_from_slice(line);
    }
    v
}

fn bench_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_throughput");
    for &mb in &[1usize, 10, 100] {
        let data = payload(mb);
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function(format!("parse_{mb}MB"), |b| {
            b.iter(|| {
                let mut adapter = WezTermAdapter::new(80, 24);
                adapter.ingest_bytes(black_box(&data));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_parser_throughput);
criterion_main!(benches);
