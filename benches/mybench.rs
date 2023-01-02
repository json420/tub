use criterion::{black_box, criterion_group, criterion_main, Criterion};
use blake3;
use tub::util::*;
use tub::dbase32::{db32enc, isdb32, db32enc_into, db32dec_into};
use tub::base::*;
use seahash;

pub fn hash_blake3(data: &[u8]) -> TubHash {
    let mut h = blake3::Hasher::new();
    h.update(data);
    let mut hash: TubHash = [0_u8; TUB_HASH_LEN];
    h.finalize_xof().fill(&mut hash);
    hash
}

fn bm_random_id(c: &mut Criterion) {
    c.bench_function("random_id", |b| b.iter(|| random_id()));
}

fn bm_hash(c: &mut Criterion) {
    let mut buf = vec![0_u8; 4096];
    getrandom(&mut buf[..]);
    c.bench_function("blake3 4 KiB", |b| b.iter(|| hash_blake3(black_box(&buf[..]))));
}

fn bm_hash2(c: &mut Criterion) {
    let mut buf = vec![0_u8; 65536];
    getrandom(&mut buf[..]);
    c.bench_function("blake3 64 KiB", |b| b.iter(|| hash_blake3(black_box(&buf[..]))));
}

fn bm_seahash(c: &mut Criterion) {
    let mut buf = vec![0_u8; 4096];
    getrandom(&mut buf[..]);
    c.bench_function("seahash 4 KiB", |b| b.iter(|| seahash::hash(black_box(&buf[..]))));
}

fn bm_seahash_64k(c: &mut Criterion) {
    let mut buf = vec![0_u8; 65536];
    getrandom(&mut buf[..]);
    c.bench_function("seahash 64 KiB", |b| b.iter(|| seahash::hash(black_box(&buf[..]))));
}


fn bm_isdb32(c: &mut Criterion) {
    let txt = db32enc(&random_hash());
    c.bench_function("isdb32",
        |b| b.iter(|| isdb32(black_box(txt.as_bytes())))
    );
}

fn bm_db32enc_into(c: &mut Criterion) {
    let src: TubHash = [0_u8; TUB_HASH_LEN];
    let mut dst = [0_u8; TUB_HASH_LEN * 8 / 5];
    c.bench_function("db32enc_into",
        |b| b.iter(|| db32enc_into(black_box(&src), black_box(&mut dst)))
    );
}

fn bm_db32dec_into(c: &mut Criterion) {
    let txt = db32enc(&random_hash());
    let mut bin = [0_u8; TUB_HASH_LEN];
    c.bench_function("db32dec_into",
        |b| b.iter(|| db32dec_into(black_box(txt.as_bytes()), black_box(&mut bin)))
    );
}


criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_random_id, bm_hash, bm_hash2, bm_seahash, bm_seahash_64k, bm_isdb32, bm_db32enc_into, bm_db32dec_into
}

criterion_main!(benches);
