use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fd_ed25519::{pubkey, Pubkey};
use fd_rpc::{Commitment, JsonRpcResponse, SolanaRpcClient, DEFAULT_RPC_URL};

fn rpc_url() -> String {
    std::env::var("RPC_URL").unwrap_or_else(|_| DEFAULT_RPC_URL.to_string())
}

fn try_connect() -> Option<SolanaRpcClient> {
    let url = rpc_url();
    match SolanaRpcClient::connect(&url) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("SKIP network benchmarks: cannot connect to {url}: {e}");
            None
        }
    }
}

fn serialization_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("serialize");

    g.bench_function("jsonrpc_request/getSlot", |b| {
        let params = b"[{\"commitment\":\"finalized\"}]";
        b.iter(|| {
            let mut buf = [0u8; 4096];
            black_box(fd_rpc::jsonrpc::serialize_request(
                &mut buf,
                "getSlot",
                params,
                black_box(1),
            ))
        });
    });

    g.bench_function("jsonrpc_request/getBalance", |b| {
        let params = br#"["11111111111111111111111111111111",{"commitment":"finalized"}]"#;
        b.iter(|| {
            let mut buf = [0u8; 4096];
            black_box(fd_rpc::jsonrpc::serialize_request(
                &mut buf,
                "getBalance",
                params,
                black_box(42),
            ))
        });
    });

    g.bench_function("jsonrpc_request/sendTransaction", |b| {
        let fake_b64_tx = "A".repeat(600);
        let params = format!(
            r#"["{}",{{"encoding":"base64","preflightCommitment":"finalized"}}]"#,
            fake_b64_tx
        );
        let params_bytes = params.as_bytes();
        b.iter(|| {
            let mut buf = [0u8; 4096];
            black_box(fd_rpc::jsonrpc::serialize_request(
                &mut buf,
                "sendTransaction",
                params_bytes,
                black_box(99),
            ))
        });
    });

    g.finish();
}

const SLOT_RESPONSE: &[u8] = br#"{"jsonrpc":"2.0","result":308518644,"id":1}"#;

const BLOCKHASH_RESPONSE: &[u8] = br#"{"jsonrpc":"2.0","result":{"context":{"slot":308518644},"value":{"blockhash":"GJM1rVLGL5JkXyTMn5Q3CnpP8hPtq6JHN3rr4mVVbRJi","lastValidBlockHeight":290042123}},"id":1}"#;

const VERSION_RESPONSE: &[u8] =
    br#"{"jsonrpc":"2.0","result":{"solana-core":"2.2.6","feature-set":1462826945},"id":1}"#;

const BALANCE_RESPONSE: &[u8] =
    br#"{"jsonrpc":"2.0","result":{"context":{"slot":308518644},"value":99000000000},"id":1}"#;

const HEALTH_RESPONSE: &[u8] = br#"{"jsonrpc":"2.0","result":"ok","id":1}"#;

fn parse_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("parse");

    let responses: &[(&str, &[u8])] = &[
        ("getSlot", SLOT_RESPONSE),
        ("getLatestBlockhash", BLOCKHASH_RESPONSE),
        ("getVersion", VERSION_RESPONSE),
        ("getBalance", BALANCE_RESPONSE),
        ("getHealth", HEALTH_RESPONSE),
    ];

    for &(name, body) in responses {
        g.bench_with_input(BenchmarkId::new("response", name), body, |b, body| {
            b.iter(|| black_box(JsonRpcResponse::parse(black_box(body)).unwrap()));
        });
    }

    g.finish();
}

fn rpc_roundtrip_benches(c: &mut Criterion) {
    let mut client = match try_connect() {
        Some(c) => c,
        None => return,
    };

    let mut g = c.benchmark_group("rpc_roundtrip");
    g.sample_size(20);
    g.measurement_time(std::time::Duration::from_secs(15));

    g.bench_function("getSlot", |b| {
        b.iter(|| {
            black_box(client.get_slot(Commitment::Confirmed).unwrap());
        });
    });

    g.bench_function("getBlockHeight", |b| {
        b.iter(|| {
            black_box(client.get_block_height(Commitment::Confirmed).unwrap());
        });
    });

    g.bench_function("getLatestBlockhash", |b| {
        b.iter(|| {
            black_box(client.get_latest_blockhash(Commitment::Confirmed).unwrap());
        });
    });

    g.bench_function("getTransactionCount", |b| {
        b.iter(|| {
            black_box(client.get_transaction_count(Commitment::Confirmed).unwrap());
        });
    });

    g.bench_function("getBalance", |b| {
        let system_program = [0u8; 32];
        b.iter(|| {
            black_box(
                client
                    .get_balance(&system_program, Commitment::Confirmed)
                    .unwrap(),
            );
        });
    });

    g.bench_function("getHealth", |b| {
        b.iter(|| {
            black_box(client.get_health().unwrap());
        });
    });

    g.bench_function("getVersion", |b| {
        b.iter(|| {
            black_box(client.get_version().unwrap());
        });
    });

    g.bench_function("getAccountInfo", |b| {
        let account = pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");
        b.iter(|| {
            black_box(
                client
                    .get_account_info(account, Commitment::Confirmed)
                    .unwrap(),
            );
        });
    });

    g.bench_function("getMultipleAccounts", |b| {
        let keys: [Pubkey; 2] = [
            pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"),
            pubkey!("BWBHrYqfcjAh5dSiRwzPnY4656cApXVXmkeDmAfwBKQG"),
        ];
        b.iter(|| {
            black_box(
                client
                    .get_multiple_accounts(&keys, Commitment::Confirmed)
                    .unwrap(),
            );
        });
    });

    g.finish();
}

criterion_group!(
    benches,
    serialization_benches,
    parse_benches,
    rpc_roundtrip_benches,
);
criterion_main!(benches);
