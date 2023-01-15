use criterion::{black_box, criterion_group, criterion_main, Criterion};
use blake3;
use tub::util::getrandom;
use tub::chaos::DefaultName;


pub fn hash_blake3(data: &[u8]) -> DefaultName {
    let mut h = blake3::Hasher::new();
    h.update(data);
    let mut hash = DefaultName::new();
    h.finalize_xof().fill(hash.as_mut_buf());
    hash
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


fn bm_db32enc(c: &mut Criterion) {
    let mut src = DefaultName::new();
    c.bench_function("db32enc: Name.to_string()",
        |b| b.iter(|| black_box(src.to_string()))
    );
}

fn bm_db32dec(c: &mut Criterion) {
    let mut hash = DefaultName::new();
    hash.randomize();
    let src = &hash.to_string();
    c.bench_function("db32dec: Name::from_str()",
        |b| b.iter(|| DefaultName::from_str(black_box(src)))
    );
}


criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bm_hash, bm_hash2, bm_db32enc, bm_db32dec
}

criterion_main!(benches);
