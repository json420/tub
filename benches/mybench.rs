use criterion::{black_box, criterion_group, criterion_main, Criterion};
use btdb_layer0::util::*;
use btdb_layer0::protocol::hash;


fn bm_random_id(c: &mut Criterion) {
    c.bench_function("random_id", |b| b.iter(|| random_id()));
}

fn bm_hash(c: &mut Criterion) {
    let buf = vec![0_u8; 4096];
    c.bench_function("hash", |b| b.iter(|| hash(black_box(&buf[..]))));
}

criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_random_id, bm_hash
}

criterion_main!(benches);
