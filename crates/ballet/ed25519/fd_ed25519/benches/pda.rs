use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fd_ed25519::Pubkey;
use solana_address::Address;

fn bench_find_program_address(c: &mut Criterion) {
    let mut g = c.benchmark_group("find_program_address");

    let program_id = Pubkey::from([42u8; 32]);
    let solana_pid = Address::from([42u8; 32]);

    let cases: &[(&str, &[&[u8]])] = &[
        ("1_seed_short", &[b"hello"]),
        ("1_seed_32b", &[&[0xABu8; 32]]),
        ("2_seeds_mixed", &[b"token_metadata", &[0xFFu8; 32]]),
        ("3_seeds", &[b"prefix", &[0xCCu8; 32], b"suffix"]),
    ];

    for (label, seeds) in cases {
        g.bench_with_input(BenchmarkId::new("solana_sdk", label), seeds, |b, seeds| {
            b.iter(|| Address::find_program_address(black_box(seeds), black_box(&solana_pid)))
        });
        g.bench_with_input(BenchmarkId::new("fd_ed25519", label), seeds, |b, seeds| {
            b.iter(|| Pubkey::find_program_address(black_box(seeds), black_box(&program_id)))
        });
    }

    g.finish();
}

fn bench_worst_case_bump(c: &mut Criterion) {
    let mut g = c.benchmark_group("worst_case_bump");

    let mut program_id_bytes = [0u8; 32];
    let seeds: &[&[u8]] = &[b"worst_case_seed"];

    let mut worst_program_id = None;
    let mut worst_bump = u8::MAX;
    for i in 0u8..=255 {
        program_id_bytes[0] = i;
        let pid = Pubkey::from(program_id_bytes);
        if let Ok((_, bump)) = Pubkey::find_program_address(seeds, &pid) {
            if bump < worst_bump {
                worst_bump = bump;
                worst_program_id = Some(pid);
                if bump < 200 {
                    break;
                }
            }
        }
    }

    let program_id = worst_program_id.expect("could not find a program_id with low bump");
    let solana_pid = Address::from(*program_id.as_bytes());

    g.bench_function(&format!("solana_sdk_bump_{worst_bump}"), |b| {
        b.iter(|| Address::find_program_address(black_box(seeds), black_box(&solana_pid)))
    });
    g.bench_function(&format!("fd_ed25519_bump_{worst_bump}"), |b| {
        b.iter(|| Pubkey::find_program_address(black_box(seeds), black_box(&program_id)))
    });

    g.finish();
}

criterion_group!(benches, bench_find_program_address, bench_worst_case_bump);
criterion_main!(benches);
