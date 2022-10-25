use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bathtub_db::util::*;
use bathtub_db::protocol::hash;
use bathtub_db::dbase32::{db32enc, isdb32, db32enc_into, db32dec_into};
use bathtub_db::base::*;


fn bm_random_id(c: &mut Criterion) {
    c.bench_function("random_id", |b| b.iter(|| random_id()));
}

fn bm_hash(c: &mut Criterion) {
    let buf = vec![0_u8; 4096];
    c.bench_function("hash", |b| b.iter(|| hash(black_box(&buf[..]))));
}

fn bm_isdb32(c: &mut Criterion) {
    let txt = db32enc(&random_object_id());
    c.bench_function("isdb32",
        |b| b.iter(|| isdb32(black_box(&txt[..])))
    );
}

fn bm_db32enc_into(c: &mut Criterion) {
    let src: ObjectID = [0_u8; OBJECT_ID_LEN];
    let mut dst = [0_u8; OBJECT_ID_LEN * 8 / 5];
    c.bench_function("db32enc_into",
        |b| b.iter(|| db32enc_into(black_box(&src), black_box(&mut dst)))
    );
}

fn bm_db32dec_into(c: &mut Criterion) {
    let txt = db32enc(&random_object_id());
    let mut bin = [0_u8; OBJECT_ID_LEN];
    c.bench_function("db32dec_into",
        |b| b.iter(|| db32dec_into(black_box(&txt[..]), black_box(&mut bin)))
    );
}


criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_random_id, bm_hash, bm_isdb32, bm_db32enc_into, bm_db32dec_into
}

criterion_main!(benches);
