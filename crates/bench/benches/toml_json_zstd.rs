use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fd_zstd::decompress_all;
use rand::{rngs::StdRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

use fd_json::{json as json_fd, parse, to_string as fd_json_to_string, JsonValue};

use serde_json;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SimpleStruct {
    id: u64,
    name: String,
    active: bool,
    score: f64,
}

impl From<&SimpleStruct> for JsonValue {
    fn from(value: &SimpleStruct) -> Self {
        json_fd!(value)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ComplexStruct {
    id: u64,
    name: String,
    metadata: std::collections::HashMap<String, serde_json::Value>,
    tags: Vec<String>,
    config: ConfigStruct,
    items: Vec<SimpleStruct>,
}

impl From<&ComplexStruct> for JsonValue {
    fn from(value: &ComplexStruct) -> Self {
        json_fd!(value)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ConfigStruct {
    enabled: bool,
    timeout_ms: u32,
    retry_count: u8,
    endpoints: Vec<String>,
}

fn generate_simple_data() -> SimpleStruct {
    SimpleStruct {
        id: 12345,
        name: "test_item".to_string(),
        active: true,
        score: 98.5,
    }
}

fn generate_complex_data() -> ComplexStruct {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "version".to_string(),
        serde_json::Value::String("1.0.0".to_string()),
    );
    metadata.insert(
        "priority".to_string(),
        serde_json::Value::Number(serde_json::Number::from(10)),
    );

    ComplexStruct {
        id: 67890,
        name: "complex_test_item".to_string(),
        metadata,
        tags: vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()],
        config: ConfigStruct {
            enabled: true,
            timeout_ms: 5000,
            retry_count: 3,
            endpoints: vec![
                "https://api1.example.com".to_string(),
                "https://api2.example.com".to_string(),
            ],
        },
        items: (0..10)
            .map(|i| SimpleStruct {
                id: i,
                name: format!("item_{}", i),
                active: i % 2 == 0,
                score: i as f64 * 10.5,
            })
            .collect(),
    }
}

fn generate_large_array_data(size: usize) -> Vec<SimpleStruct> {
    (0..size)
        .map(|i| SimpleStruct {
            id: i as u64,
            name: format!("item_{}", i),
            active: i % 2 == 0,
            score: i as f64 * 1.5,
        })
        .collect()
}

fn bench_json_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialize");

    let simple_data = generate_simple_data();
    let complex_data = generate_complex_data();

    group.bench_function("firedancer_simple", |b| {
        b.iter(|| black_box(fd_json_to_string(&json_fd!(&simple_data)).unwrap()))
    });

    group.bench_function("serde_json_simple", |b| {
        b.iter(|| black_box(serde_json::to_string(&simple_data).unwrap()))
    });

    group.bench_function("firedancer_complex", |b| {
        b.iter(|| black_box(fd_json_to_string(&json_fd!(&complex_data)).unwrap()))
    });

    group.bench_function("serde_json_complex", |b| {
        b.iter(|| black_box(serde_json::to_string(&complex_data).unwrap()))
    });

    // let array_sizes = &[100, 1000, 10000];
    // for &size in array_sizes {
    //     let large_data = generate_large_array_data(size);

    //     group.throughput(Throughput::Elements(size as u64));

    //     group.bench_with_input(
    //         BenchmarkId::new("firedancer_array", size),
    //         &large_data,
    //         |b, data| b.iter(|| black_box(fd_json_to_string(&json_fd!(data)).unwrap())),
    //     );

    //     group.bench_with_input(
    //         BenchmarkId::new("serde_json_array", size),
    //         &large_data,
    //         |b, data| b.iter(|| black_box(serde_json::to_string(data).unwrap())),
    //     );
    // }

    group.finish();
}

fn bench_json_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_deserialize");

    let simple_data = generate_simple_data();
    let complex_data = generate_complex_data();

    let simple_json = serde_json::to_string(&simple_data).unwrap();
    let complex_json = serde_json::to_string(&complex_data).unwrap();

    group.bench_function("firedancer_simple", |b| {
        b.iter(|| black_box(parse(&simple_json).unwrap()))
    });

    group.bench_function("serde_json_simple", |b| {
        b.iter(|| black_box(serde_json::from_str::<SimpleStruct>(&simple_json).unwrap()))
    });

    group.bench_function("firedancer_complex", |b| {
        b.iter(|| black_box(parse(&complex_json).unwrap()))
    });

    group.bench_function("serde_json_complex", |b| {
        b.iter(|| black_box(serde_json::from_str::<ComplexStruct>(&complex_json).unwrap()))
    });

    let array_sizes = &[100, 1000, 10000];
    for &size in array_sizes {
        let large_data = generate_large_array_data(size);
        let large_json = serde_json::to_string(&large_data).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("firedancer_array", size),
            &large_json,
            |b, json| b.iter(|| black_box(parse(json).unwrap())),
        );

        group.bench_with_input(
            BenchmarkId::new("serde_json_array", size),
            &large_json,
            |b, json| {
                b.iter(|| black_box(serde_json::from_str::<Vec<SimpleStruct>>(json).unwrap()))
            },
        );
    }

    group.finish();
}

// fn bench_toml_serialize(c: &mut Criterion) {
//     let mut group = c.benchmark_group("toml_serialize");

//     let simple_data = generate_simple_data();
//     let complex_data = generate_complex_data();

//     group.bench_function("firedancer_simple", |b| {
//         b.iter(|| black_box(fd_toml::to_string(&simple_data).unwrap()))
//     });

//     group.bench_function("toml_crate_simple", |b| {
//         b.iter(|| black_box(toml::to_string(&simple_data).unwrap()))
//     });

//     group.bench_function("firedancer_complex", |b| {
//         b.iter(|| black_box(fd_toml_to_string(&complex_data).unwrap()))
//     });

//     group.bench_function("toml_crate_complex", |b| {
//         b.iter(|| black_box(toml::to_string(&complex_data).unwrap()))
//     });

//     group.finish();
// }

// fn bench_toml_deserialize(c: &mut Criterion) {
//     let mut group = c.benchmark_group("toml_deserialize");

//     let simple_data = generate_simple_data();
//     let complex_data = generate_complex_data();

//     let simple_toml = toml::to_string(&simple_data).unwrap();
//     let complex_toml = toml::to_string(&complex_data).unwrap();

//     group.bench_function("firedancer_simple", |b| {
//         b.iter(|| black_box(fd_toml_from_str::<SimpleStruct>(&simple_toml).unwrap()))
//     });

//     group.bench_function("toml_crate_simple", |b| {
//         b.iter(|| black_box(toml::from_str::<SimpleStruct>(&simple_toml).unwrap()))
//     });

//     group.bench_function("firedancer_complex", |b| {
//         b.iter(|| black_box(fd_toml_from_str::<ComplexStruct>(&complex_toml).unwrap()))
//     });

//     group.bench_function("toml_crate_complex", |b| {
//         b.iter(|| black_box(toml::from_str::<ComplexStruct>(&complex_toml).unwrap()))
//     });

//     group.finish();
// }

fn bench_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");

    let data_sizes = &[1024, 4096, 16384, 65536, 262144];

    for &size in data_sizes {
        let mut rng = StdRng::seed_from_u64(42);
        let mut data = vec![0u8; size];
        rng.fill_bytes(&mut data);

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("firedancer_compress", size),
            &data,
            |b, data| b.iter(|| black_box(decompress_all(data, 1).unwrap())),
        );

        group.bench_with_input(
            BenchmarkId::new("zstd_crate_compress", size),
            &data,
            |b, data| b.iter(|| black_box(zstd::bulk::compress(data, 1).unwrap())),
        );
    }

    group.finish();
}

criterion_group!(
    serialization_benches,
    bench_json_serialize,
    bench_json_deserialize,
    bench_compression,
);

criterion_main!(serialization_benches);
