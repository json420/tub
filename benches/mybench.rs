use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bathtub_db::util::*;
use bathtub_db::protocol::hash;
use bathtub_db::dbase32::encode;
use bathtub_db::base::*;


fn bm_random_id(c: &mut Criterion) {
    c.bench_function("random_id", |b| b.iter(|| random_id()));
}

fn bm_hash(c: &mut Criterion) {
    let buf = vec![0_u8; 4096];
    c.bench_function("hash", |b| b.iter(|| hash(black_box(&buf[..]))));
}

fn bm_encode(c: &mut Criterion) {
    let id: ObjectID = [0_u8; OBJECT_ID_LEN];
    c.bench_function("encode", |b| b.iter(|| encode(black_box(&id))));
}

criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_random_id, bm_hash, bm_encode
}

criterion_main!(benches);
