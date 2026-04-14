use std::str::FromStr;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fd_ed25519::{pubkey, Pubkey};
use fd_rpc::{BatchEntry, Commitment, JsonRpcResponse, SolanaRpcClient, DEFAULT_RPC_URL};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_pubkey::Pubkey as SolanaPubkey;

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

fn batch_roundtrip_benches(c: &mut Criterion) {
    let mut client = match try_connect() {
        Some(c) => c,
        None => return,
    };

    let mut g = c.benchmark_group("batch_roundtrip");
    g.sample_size(20);
    g.measurement_time(std::time::Duration::from_secs(15));

    let system_program = [0u8; 32];
    let multiple_keys: [Pubkey; 2] = [
        pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"),
        pubkey!("BWBHrYqfcjAh5dSiRwzPnY4656cApXVXmkeDmAfwBKQG"),
    ];

    let entries = [
        BatchEntry::get_slot(Commitment::Confirmed),
        BatchEntry::get_latest_blockhash(Commitment::Confirmed),
        BatchEntry::get_transaction_count(Commitment::Confirmed),
        BatchEntry::get_balance(&system_program, Commitment::Confirmed),
        BatchEntry::get_multiple_accounts(&multiple_keys, Commitment::Confirmed),
    ];

    g.bench_function("5_calls", |b| {
        b.iter(|| {
            black_box(client.batch(black_box(&entries)).unwrap());
        });
    });

    g.finish();
}

fn solana_client_roundtrip_benches(c: &mut Criterion) {
    let url = rpc_url();
    let client = RpcClient::new_with_commitment(url, CommitmentConfig::confirmed());

    if client.get_slot().is_err() {
        eprintln!("SKIP solana_client benchmarks: cannot connect to RPC");
        return;
    }

    let mut g = c.benchmark_group("solana_client_roundtrip");
    g.sample_size(20);
    g.measurement_time(std::time::Duration::from_secs(15));

    g.bench_function("getSlot", |b| {
        b.iter(|| {
            black_box(client.get_slot().unwrap());
        });
    });

    g.bench_function("getBlockHeight", |b| {
        b.iter(|| {
            black_box(client.get_block_height().unwrap());
        });
    });

    g.bench_function("getLatestBlockhash", |b| {
        b.iter(|| {
            black_box(client.get_latest_blockhash().unwrap());
        });
    });

    g.bench_function("getTransactionCount", |b| {
        b.iter(|| {
            black_box(client.get_transaction_count().unwrap());
        });
    });

    g.bench_function("getBalance", |b| {
        let system_program = SolanaPubkey::default();
        b.iter(|| {
            black_box(client.get_balance(&system_program).unwrap());
        });
    });

    g.bench_function("getVersion", |b| {
        b.iter(|| {
            black_box(client.get_version().unwrap());
        });
    });

    g.bench_function("getAccountInfo", |b| {
        let account =
            SolanaPubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE").unwrap();
        b.iter(|| {
            black_box(client.get_account(&account).unwrap());
        });
    });

    g.bench_function("getMultipleAccounts", |b| {
        let keys = [
            SolanaPubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE").unwrap(),
            SolanaPubkey::from_str("BWBHrYqfcjAh5dSiRwzPnY4656cApXVXmkeDmAfwBKQG").unwrap(),
        ];
        b.iter(|| {
            black_box(client.get_multiple_accounts(&keys).unwrap());
        });
    });

    g.finish();
}

criterion_group!(
    benches,
    serialization_benches,
    parse_benches,
    rpc_roundtrip_benches,
    batch_roundtrip_benches,
    solana_client_roundtrip_benches,
);
criterion_main!(benches);
