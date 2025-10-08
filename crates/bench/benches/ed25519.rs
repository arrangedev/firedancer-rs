use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{rngs::StdRng, RngCore, SeedableRng};

use fd_ed25519::{Keypair, Pubkey};
use solana_pubkey::Pubkey as SolanaPubkey;

const MESSAGE_SIZES: &[usize] = &[32, 64, 128, 256, 512, 1024, 2048, 4096];

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut data = vec![0u8; size];
    rng.fill_bytes(&mut data);
    data
}

fn bench_ed25519_keygen(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519_keygen");

    group.bench_function("firedancer", |b| {
        let mut rng = StdRng::seed_from_u64(42);
        b.iter(|| {
            let mut secret_key = [0u8; 32];
            rng.fill_bytes(&mut secret_key);
            black_box(Keypair::from_secret_key(&secret_key).unwrap())
        })
    });

    group.finish();
}

fn bench_ed25519_sign(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519_sign");
    let fd_keypair = Keypair::from_secret_key(&[1u8; 32]).unwrap();

    for &size in MESSAGE_SIZES {
        let message = generate_test_data(size);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("firedancer", size), &message, |b, msg| {
            b.iter(|| black_box(fd_keypair.sign(msg).unwrap()))
        });
    }

    group.finish();
}

fn bench_ed25519_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519_verify");

    let fd_keypair = Keypair::from_secret_key(&[1u8; 32]).unwrap();

    for &size in MESSAGE_SIZES {
        let message = generate_test_data(size);
        let fd_signature = fd_keypair.sign(&message).unwrap();

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("firedancer", size), &message, |b, msg| {
            b.iter(|| black_box(fd_keypair.pubkey().verify(msg, &fd_signature).unwrap()))
        });
    }

    group.finish();
}

fn bench_ed25519_batch_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519_batch_verify");

    let batch_sizes = &[1, 4, 8, 16, 32, 64];
    let message = generate_test_data(256);

    for &batch_size in batch_sizes {
        let fd_keypairs: Vec<_> = (0..batch_size)
            .map(|i| Keypair::from_secret_key(&[(i + 1) as u8; 32]).unwrap())
            .collect();
        let fd_pubkeys: Vec<_> = fd_keypairs.iter().map(|kp| *kp.pubkey()).collect();
        let fd_signatures: Vec<_> = fd_keypairs
            .iter()
            .map(|kp| kp.sign(&message).unwrap())
            .collect();

        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("firedancer", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    black_box(
                        fd_ed25519::batch_verify_single_message(
                            &message,
                            &fd_pubkeys,
                            &fd_signatures,
                        )
                        .unwrap(),
                    )
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("firedancer_individual", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    let result: Result<Vec<_>, _> = fd_keypairs
                        .iter()
                        .zip(&fd_signatures)
                        .map(|(kp, sig)| kp.pubkey().verify(&message, sig))
                        .collect();
                    black_box(result.is_ok())
                })
            },
        );
    }

    group.finish();
}

fn bench_pubkey_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("pubkey_serialization");
    let fd_keypair = Keypair::from_secret_key(&[1u8; 32]).unwrap();
    let fd_pubkey = fd_keypair.pubkey();

    group.bench_function("firedancer_to_base58", |b| {
        b.iter(|| black_box(fd_pubkey.to_base58()))
    });

    group.bench_function("solana_to_base58", |b| {
        let solana_pubkey = SolanaPubkey::new_from_array(*fd_pubkey.as_bytes());
        b.iter(|| black_box(solana_pubkey.to_string()))
    });

    let fd_base58 = fd_pubkey.to_base58();
    let solana_pubkey = SolanaPubkey::new_from_array(*fd_pubkey.as_bytes());
    let solana_base58 = solana_pubkey.to_string();

    group.bench_function("firedancer_from_base58", |b| {
        b.iter(|| black_box(Pubkey::from_base58(&fd_base58).unwrap()))
    });

    group.bench_function("solana_from_base58", |b| {
        b.iter(|| black_box(solana_base58.parse::<SolanaPubkey>().unwrap()))
    });

    group.bench_function("firedancer_to_hex", |b| {
        b.iter(|| black_box(fd_pubkey.to_hex()))
    });

    group.bench_function("hex_to_hex", |b| {
        b.iter(|| black_box(hex::encode(fd_pubkey.as_bytes())))
    });

    group.finish();
}

criterion_group!(
    ed25519_benches,
    bench_ed25519_keygen,
    bench_ed25519_sign,
    bench_ed25519_verify,
    bench_ed25519_batch_verify,
    bench_pubkey_serialization
);
criterion_main!(ed25519_benches);
