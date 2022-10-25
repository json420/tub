use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bathtub_db::util::*;
use bathtub_db::protocol::hash;
use bathtub_db::dbase32::{db32enc, db32enc_into, decode};
use bathtub_db::base::*;


fn bm_random_id(c: &mut Criterion) {
    c.bench_function("random_id", |b| b.iter(|| random_id()));
}

fn bm_hash(c: &mut Criterion) {
    let buf = vec![0_u8; 4096];
    c.bench_function("hash", |b| b.iter(|| hash(black_box(&buf[..]))));
}

fn bm_db32enc_into(c: &mut Criterion) {
    let src: ObjectID = [0_u8; OBJECT_ID_LEN];
    let mut dst = [0_u8; OBJECT_ID_LEN / 5 * 8];
    c.bench_function("db32enc_into",
        |b| b.iter(|| db32enc_into(black_box(&src), black_box(&mut dst)))
    );
}

fn bm_decode(c: &mut Criterion) {
    let txt = db32enc(&random_object_id());
    c.bench_function("decode",
        |b| b.iter(|| decode(black_box(&txt)))
    );
}

criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_random_id, bm_hash, bm_db32enc_into, bm_decode
}

criterion_main!(benches);
