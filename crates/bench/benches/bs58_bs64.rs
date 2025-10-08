use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::{rngs::StdRng, RngCore, SeedableRng};

use fd_base64::{decode, encode};

use base64::{engine::general_purpose, Engine as _};

const DATA_SIZES: &[usize] = &[64, 256, 1024, 4096, 16384, 65536, 262144];

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut data = vec![0u8; size];
    rng.fill_bytes(&mut data);
    data
}

fn bench_base64_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("base64_encode");

    for &size in DATA_SIZES {
        let data = generate_test_data(size);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("firedancer", size), &data, |b, data| {
            b.iter(|| black_box(encode(data)))
        });

        group.bench_with_input(BenchmarkId::new("base64_crate", size), &data, |b, data| {
            b.iter(|| black_box(general_purpose::STANDARD.encode(data)))
        });

        group.bench_with_input(
            BenchmarkId::new("base64_crate_url_safe", size),
            &data,
            |b, data| b.iter(|| black_box(general_purpose::URL_SAFE.encode(data))),
        );
    }

    group.finish();
}

fn bench_base64_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("base64_decode");

    for &size in DATA_SIZES {
        let data = generate_test_data(size);
        let fd_encoded = encode(&data);
        let base64_encoded = general_purpose::STANDARD.encode(&data);
        let url_safe_encoded = general_purpose::URL_SAFE.encode(&data);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("firedancer", size),
            &fd_encoded,
            |b, encoded| b.iter(|| black_box(decode(encoded).unwrap())),
        );

        group.bench_with_input(
            BenchmarkId::new("base64_crate", size),
            &base64_encoded,
            |b, encoded| b.iter(|| black_box(general_purpose::STANDARD.decode(encoded).unwrap())),
        );

        group.bench_with_input(
            BenchmarkId::new("base64_crate_url_safe", size),
            &url_safe_encoded,
            |b, encoded| b.iter(|| black_box(general_purpose::URL_SAFE.decode(encoded).unwrap())),
        );
    }

    group.finish();
}

fn bench_base64_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("base64_roundtrip");

    for &size in DATA_SIZES {
        let data = generate_test_data(size);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("firedancer", size), &data, |b, data| {
            b.iter(|| {
                let encoded = encode(data);
                black_box(decode(&encoded).unwrap())
            })
        });

        group.bench_with_input(BenchmarkId::new("base64_crate", size), &data, |b, data| {
            b.iter(|| {
                let encoded = general_purpose::STANDARD.encode(data);
                black_box(general_purpose::STANDARD.decode(&encoded).unwrap())
            })
        });
    }

    group.finish();
}

fn bench_hex_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("hex_encoding");

    for &size in DATA_SIZES {
        let data = generate_test_data(size);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("hex_crate_encode", size),
            &data,
            |b, data| b.iter(|| black_box(hex::encode(data))),
        );

        let hex_encoded = hex::encode(&data);

        group.bench_with_input(
            BenchmarkId::new("hex_crate_decode", size),
            &hex_encoded,
            |b, encoded| b.iter(|| black_box(hex::decode(encoded).unwrap())),
        );
    }

    group.finish();
}

fn bench_encoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding_comparison");

    let size = 4096;
    let data = generate_test_data(size);

    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("base64_firedancer_encode", |b| {
        b.iter(|| black_box(encode(&data)))
    });

    group.bench_function("base64_standard_encode", |b| {
        b.iter(|| black_box(general_purpose::STANDARD.encode(&data)))
    });

    group.bench_function("hex_encode", |b| b.iter(|| black_box(hex::encode(&data))));

    let base64_encoded = encode(&data);
    let hex_encoded = hex::encode(&data);

    group.bench_function("base64_firedancer_decode", |b| {
        b.iter(|| black_box(decode(&base64_encoded).unwrap()))
    });

    group.bench_function("base64_standard_decode", |b| {
        let std_encoded = general_purpose::STANDARD.encode(&data);
        b.iter(|| black_box(general_purpose::STANDARD.decode(&std_encoded).unwrap()))
    });

    group.bench_function("hex_decode", |b| {
        b.iter(|| black_box(hex::decode(&hex_encoded).unwrap()))
    });

    group.finish();
}

criterion_group!(
    encoding_benches,
    bench_base64_encode,
    bench_base64_decode,
    bench_base64_roundtrip,
    bench_hex_encoding,
    bench_encoding_comparison
);
criterion_main!(encoding_benches);
