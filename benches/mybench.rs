use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bathtub_db::util::*;
use bathtub_db::protocol::{hash, hash2};


fn bm_random_id(c: &mut Criterion) {
    c.bench_function("random_id", |b| b.iter(|| random_id()));
}

fn bm_hash(c: &mut Criterion) {
    let buf = vec![0_u8; 4096];
    c.bench_function("hash", |b| b.iter(|| hash(black_box(&buf[..]))));
}

fn bm_hash2(c: &mut Criterion) {
    let buf = vec![0_u8; 4096];
    c.bench_function("hash2", |b| b.iter(|| hash2(black_box(&buf[..]))));
}

criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_random_id, bm_hash, bm_hash2
}

criterion_main!(benches);
