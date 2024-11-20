use blake3;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use getrandom::getrandom;
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
    getrandom(&mut buf[..]).unwrap();
    c.bench_function("blake3 4 KiB", |b| {
        b.iter(|| hash_blake3(black_box(&buf[..])))
    });
}

fn bm_hash2(c: &mut Criterion) {
    let mut buf = vec![0_u8; 65536];
    getrandom(&mut buf[..]).unwrap();
    c.bench_function("blake3 64 KiB", |b| {
        b.iter(|| hash_blake3(black_box(&buf[..])))
    });
}

fn bm_dalek_s(c: &mut Criterion) {
    let buf = [7_u8; 30];
    use ed25519_dalek::{Signature, Signer, SigningKey};
    use rand::rngs::OsRng;
    let mut csprng = OsRng;
    let sk = SigningKey::generate(&mut csprng);
    c.bench_function("ed25519-dalek sign", |b| {
        b.iter(|| sk.sign(black_box(&buf)))
    });
}

fn bm_dalek_v(c: &mut Criterion) {
    let buf = [7_u8; 30];
    use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
    use rand::rngs::OsRng;
    let mut csprng = OsRng;
    let sk = SigningKey::generate(&mut csprng);
    let sig = sk.sign(&buf);
    let pk = sk.verifying_key();
    c.bench_function("ed25519_dalek verify", |b| {
        b.iter(|| pk.verify(black_box(&buf), black_box(&sig)))
    });
}

fn bm_dalek_v_strict(c: &mut Criterion) {
    let buf = [7_u8; 30];
    use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
    use rand::rngs::OsRng;
    let mut csprng = OsRng;
    let sk = SigningKey::generate(&mut csprng);
    let sig = sk.sign(&buf);
    let pk = sk.verifying_key();
    c.bench_function("ed25519_dalek verify_strict", |b| {
        b.iter(|| pk.verify_strict(black_box(&buf), black_box(&sig)))
    });
}

fn bm_db32enc(c: &mut Criterion) {
    let mut src = DefaultName::new();
    c.bench_function("db32enc: Name.to_string()", |b| {
        b.iter(|| black_box(src.to_string()))
    });
}

fn bm_db32dec(c: &mut Criterion) {
    let mut hash = DefaultName::new();
    hash.randomize();
    let src = &hash.to_string();
    c.bench_function("db32dec: Name::from_str()", |b| {
        b.iter(|| DefaultName::from_dbase32(black_box(src)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bm_hash, bm_hash2, bm_dalek_s, bm_dalek_v, bm_dalek_v_strict, bm_db32enc, bm_db32dec
}

criterion_main!(benches);
