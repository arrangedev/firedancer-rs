use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use fd_tmpl::{CStrDeque, FdHeap, FdMap, FdPool, FdQueue, FdSet, FdStack, FdVec};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

// ── Map: FdMap (256 slots) vs HashMap<u64,u64> ─────────────────────────────

fn map_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("map");
    const N: u64 = 100;

    g.bench_function("fd/insert", |b| {
        b.iter_batched(
            || {
                let mut m = FdMap::new().unwrap();
                for i in 0..N {
                    m.insert(i, i * 10).unwrap();
                }
                m
            },
            |mut m| {
                m.insert(black_box(N), black_box(N * 10)).unwrap();
                m
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/insert", |b| {
        b.iter_batched(
            || {
                let mut m = HashMap::<u64, u64>::with_capacity(256);
                for i in 0..N {
                    m.insert(i, i * 10);
                }
                m
            },
            |mut m| {
                m.insert(black_box(N), black_box(N * 10));
                m
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/lookup_hit", |b| {
        let mut m = FdMap::new().unwrap();
        for i in 0..N {
            m.insert(i, i * 10).unwrap();
        }
        b.iter(|| black_box(m.get(&black_box(50))));
    });

    g.bench_function("std/lookup_hit", |b| {
        let mut m = HashMap::<u64, u64>::with_capacity(256);
        for i in 0..N {
            m.insert(i, i * 10);
        }
        b.iter(|| black_box(m.get(&black_box(50u64))));
    });

    g.bench_function("fd/lookup_miss", |b| {
        let mut m = FdMap::new().unwrap();
        for i in 0..N {
            m.insert(i, i * 10).unwrap();
        }
        b.iter(|| black_box(m.get(&black_box(N + 100))));
    });

    g.bench_function("std/lookup_miss", |b| {
        let mut m = HashMap::<u64, u64>::with_capacity(256);
        for i in 0..N {
            m.insert(i, i * 10);
        }
        b.iter(|| black_box(m.get(&black_box(N + 100))));
    });

    g.bench_function("fd/remove", |b| {
        b.iter_batched(
            || {
                let mut m = FdMap::new().unwrap();
                for i in 0..N {
                    m.insert(i, i * 10).unwrap();
                }
                m
            },
            |mut m| {
                m.remove(&black_box(50));
                m
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/remove", |b| {
        b.iter_batched(
            || {
                let mut m = HashMap::<u64, u64>::with_capacity(256);
                for i in 0..N {
                    m.insert(i, i * 10);
                }
                m
            },
            |mut m| {
                m.remove(&black_box(50u64));
                m
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/clear", |b| {
        b.iter_batched(
            || {
                let mut m = FdMap::new().unwrap();
                for i in 0..N {
                    m.insert(i, i * 10).unwrap();
                }
                m
            },
            |mut m| {
                m.clear();
                m
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/clear", |b| {
        b.iter_batched(
            || {
                let mut m = HashMap::<u64, u64>::with_capacity(256);
                for i in 0..N {
                    m.insert(i, i * 10);
                }
                m
            },
            |mut m| {
                m.clear();
                m
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Set: FdSet (256 slots) vs HashSet<u64> ──────────────────────────────────

fn set_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("set");
    const N: u64 = 100;

    g.bench_function("fd/insert", |b| {
        b.iter_batched(
            || {
                let mut s = FdSet::new().unwrap();
                for i in 0..N {
                    s.insert(i).unwrap();
                }
                s
            },
            |mut s| {
                s.insert(black_box(N)).unwrap();
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/insert", |b| {
        b.iter_batched(
            || {
                let mut s = HashSet::<u64>::with_capacity(256);
                for i in 0..N {
                    s.insert(i);
                }
                s
            },
            |mut s| {
                s.insert(black_box(N));
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/contains_hit", |b| {
        let mut s = FdSet::new().unwrap();
        for i in 0..N {
            s.insert(i).unwrap();
        }
        b.iter(|| black_box(s.contains(&black_box(50))));
    });

    g.bench_function("std/contains_hit", |b| {
        let mut s = HashSet::<u64>::with_capacity(256);
        for i in 0..N {
            s.insert(i);
        }
        b.iter(|| black_box(s.contains(&black_box(50u64))));
    });

    g.bench_function("fd/contains_miss", |b| {
        let mut s = FdSet::new().unwrap();
        for i in 0..N {
            s.insert(i).unwrap();
        }
        b.iter(|| black_box(s.contains(&black_box(N + 100))));
    });

    g.bench_function("std/contains_miss", |b| {
        let mut s = HashSet::<u64>::with_capacity(256);
        for i in 0..N {
            s.insert(i);
        }
        b.iter(|| black_box(s.contains(&black_box(N + 100))));
    });

    g.bench_function("fd/remove", |b| {
        b.iter_batched(
            || {
                let mut s = FdSet::new().unwrap();
                for i in 0..N {
                    s.insert(i).unwrap();
                }
                s
            },
            |mut s| {
                s.remove(&black_box(50));
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/remove", |b| {
        b.iter_batched(
            || {
                let mut s = HashSet::<u64>::with_capacity(256);
                for i in 0..N {
                    s.insert(i);
                }
                s
            },
            |mut s| {
                s.remove(&black_box(50u64));
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Stack: FdStack (cap 64) vs Vec<u64> (LIFO) ─────────────────────────────

fn stack_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("stack");
    const PREFILL: u64 = 30;

    g.bench_function("fd/push", |b| {
        b.iter_batched(
            || {
                let mut s = FdStack::new().unwrap();
                for i in 0..PREFILL {
                    s.push(i).unwrap();
                }
                s
            },
            |mut s| {
                s.push(black_box(999)).unwrap();
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/push", |b| {
        b.iter_batched(
            || {
                let mut v = Vec::<u64>::with_capacity(64);
                for i in 0..PREFILL {
                    v.push(i);
                }
                v
            },
            |mut v| {
                v.push(black_box(999));
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/pop", |b| {
        b.iter_batched(
            || {
                let mut s = FdStack::new().unwrap();
                for i in 0..PREFILL {
                    s.push(i).unwrap();
                }
                s
            },
            |mut s| {
                black_box(s.pop());
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/pop", |b| {
        b.iter_batched(
            || {
                let mut v = Vec::<u64>::with_capacity(64);
                for i in 0..PREFILL {
                    v.push(i);
                }
                v
            },
            |mut v| {
                black_box(v.pop());
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/peek", |b| {
        let mut s = FdStack::new().unwrap();
        for i in 0..PREFILL {
            s.push(i).unwrap();
        }
        b.iter(|| black_box(s.peek()));
    });

    g.bench_function("std/peek", |b| {
        let mut v = Vec::<u64>::with_capacity(64);
        for i in 0..PREFILL {
            v.push(i);
        }
        b.iter(|| black_box(v.last()));
    });

    g.bench_function("fd/fill_drain", |b| {
        b.iter_batched(
            || FdStack::new().unwrap(),
            |mut s| {
                for i in 0..60u64 {
                    s.push(black_box(i)).unwrap();
                }
                for _ in 0..60 {
                    black_box(s.pop());
                }
                s
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/fill_drain", |b| {
        b.iter_batched(
            || Vec::<u64>::with_capacity(64),
            |mut v| {
                for i in 0..60u64 {
                    v.push(black_box(i));
                }
                for _ in 0..60 {
                    black_box(v.pop());
                }
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Queue: FdQueue (cap 64) vs VecDeque<u64> ────────────────────────────────

fn queue_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("queue");
    const PREFILL: u64 = 30;

    g.bench_function("fd/push", |b| {
        b.iter_batched(
            || {
                let mut q = FdQueue::new().unwrap();
                for i in 0..PREFILL {
                    q.push(i).unwrap();
                }
                q
            },
            |mut q| {
                q.push(black_box(999)).unwrap();
                q
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/push", |b| {
        b.iter_batched(
            || {
                let mut q = VecDeque::<u64>::with_capacity(64);
                for i in 0..PREFILL {
                    q.push_back(i);
                }
                q
            },
            |mut q| {
                q.push_back(black_box(999));
                q
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/pop", |b| {
        b.iter_batched(
            || {
                let mut q = FdQueue::new().unwrap();
                for i in 0..PREFILL {
                    q.push(i).unwrap();
                }
                q
            },
            |mut q| {
                black_box(q.pop());
                q
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/pop", |b| {
        b.iter_batched(
            || {
                let mut q = VecDeque::<u64>::with_capacity(64);
                for i in 0..PREFILL {
                    q.push_back(i);
                }
                q
            },
            |mut q| {
                black_box(q.pop_front());
                q
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/fill_drain", |b| {
        b.iter_batched(
            || FdQueue::new().unwrap(),
            |mut q| {
                for i in 0..60u64 {
                    q.push(black_box(i)).unwrap();
                }
                for _ in 0..60 {
                    black_box(q.pop());
                }
                q
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/fill_drain", |b| {
        b.iter_batched(
            || VecDeque::<u64>::with_capacity(64),
            |mut q| {
                for i in 0..60u64 {
                    q.push_back(black_box(i));
                }
                for _ in 0..60 {
                    black_box(q.pop_front());
                }
                q
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Deque: CStrDeque (cap 64) vs VecDeque<String> ──────────────────────────

fn deque_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("deque");

    g.bench_function("fd/push_tail", |b| {
        b.iter_batched(
            || {
                let mut d = CStrDeque::new().unwrap();
                for i in 0..15 {
                    d.push_tail(&format!("s{i}")).unwrap();
                }
                d
            },
            |mut d| {
                d.push_tail(black_box("bench_value")).unwrap();
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/push_tail", |b| {
        b.iter_batched(
            || {
                let mut d = VecDeque::<String>::with_capacity(64);
                for i in 0..15 {
                    d.push_back(format!("s{i}"));
                }
                d
            },
            |mut d| {
                d.push_back(black_box(String::from("bench_value")));
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/push_head", |b| {
        b.iter_batched(
            || {
                let mut d = CStrDeque::new().unwrap();
                for i in 0..15 {
                    d.push_tail(&format!("s{i}")).unwrap();
                }
                d
            },
            |mut d| {
                d.push_head(black_box("bench_value")).unwrap();
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/push_head", |b| {
        b.iter_batched(
            || {
                let mut d = VecDeque::<String>::with_capacity(64);
                for i in 0..15 {
                    d.push_back(format!("s{i}"));
                }
                d
            },
            |mut d| {
                d.push_front(black_box(String::from("bench_value")));
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/pop_head", |b| {
        b.iter_batched(
            || {
                let mut d = CStrDeque::new().unwrap();
                for i in 0..30 {
                    d.push_tail(&format!("s{i}")).unwrap();
                }
                d
            },
            |mut d| {
                black_box(d.pop_head());
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/pop_head", |b| {
        b.iter_batched(
            || {
                let mut d = VecDeque::<String>::with_capacity(64);
                for i in 0..30 {
                    d.push_back(format!("s{i}"));
                }
                d
            },
            |mut d| {
                black_box(d.pop_front());
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/pop_tail", |b| {
        b.iter_batched(
            || {
                let mut d = CStrDeque::new().unwrap();
                for i in 0..30 {
                    d.push_tail(&format!("s{i}")).unwrap();
                }
                d
            },
            |mut d| {
                black_box(d.pop_tail());
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/pop_tail", |b| {
        b.iter_batched(
            || {
                let mut d = VecDeque::<String>::with_capacity(64);
                for i in 0..30 {
                    d.push_back(format!("s{i}"));
                }
                d
            },
            |mut d| {
                black_box(d.pop_back());
                d
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Heap: FdHeap vs BinaryHeap<u64> ────────────────────────────────────────

fn heap_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("heap");
    const CAP: u64 = 1024;

    g.bench_function("fd/insert", |b| {
        b.iter_batched(
            || FdHeap::new(CAP).unwrap(),
            |mut h| {
                h.insert(black_box(42)).unwrap();
                h
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/push", |b| {
        b.iter_batched(
            || BinaryHeap::<u64>::with_capacity(CAP as usize),
            |mut h| {
                h.push(black_box(42));
                h
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/batch_insert", |b| {
        b.iter_batched(
            || FdHeap::new(CAP).unwrap(),
            |mut h| {
                for i in 0..100u64 {
                    let _ = h.insert(black_box(i));
                }
                h
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/batch_push", |b| {
        b.iter_batched(
            || BinaryHeap::<u64>::with_capacity(CAP as usize),
            |mut h| {
                for i in 0..100u64 {
                    h.push(black_box(i));
                }
                h
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/pop", |b| {
        b.iter_batched(
            || {
                let mut h = BinaryHeap::<u64>::with_capacity(CAP as usize);
                for i in 0..500 {
                    h.push(i);
                }
                h
            },
            |mut h| {
                black_box(h.pop());
                h
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Pool: FdPool (no std equivalent) ────────────────────────────────────────

fn pool_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("pool");
    const CAP: u64 = 1024;

    g.bench_function("fd/acquire", |b| {
        b.iter_batched(
            || FdPool::new(CAP).unwrap(),
            |mut p| {
                black_box(p.acquire());
                p
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/release", |b| {
        b.iter_batched(
            || {
                let mut p = FdPool::new(CAP).unwrap();
                let idx = p.acquire().unwrap();
                (p, idx)
            },
            |(mut p, idx)| {
                p.release(black_box(idx));
                p
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/acquire_release", |b| {
        b.iter_batched(
            || FdPool::new(CAP).unwrap(),
            |mut p| {
                let idx = p.acquire().unwrap();
                p.release(idx);
                p
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/batch_acquire_release", |b| {
        b.iter_batched(
            || FdPool::new(CAP).unwrap(),
            |mut p| {
                let mut indices = [0u64; 100];
                for slot in &mut indices {
                    *slot = p.acquire().unwrap();
                }
                for &idx in &indices {
                    p.release(idx);
                }
                p
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

// ── Vec: FdVec vs Vec<u64> ──────────────────────────────────────────────────

fn vec_benches(c: &mut Criterion) {
    let mut g = c.benchmark_group("vec");
    const CAP: u64 = 1024;
    const PREFILL: u64 = 500;

    g.bench_function("fd/push", |b| {
        b.iter_batched(
            || {
                let mut v = FdVec::new(CAP).unwrap();
                for i in 0..PREFILL {
                    v.push(i).unwrap();
                }
                v
            },
            |mut v| {
                v.push(black_box(PREFILL)).unwrap();
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/push", |b| {
        b.iter_batched(
            || {
                let mut v = Vec::<u64>::with_capacity(CAP as usize);
                for i in 0..PREFILL {
                    v.push(i);
                }
                v
            },
            |mut v| {
                v.push(black_box(PREFILL));
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/pop", |b| {
        b.iter_batched(
            || {
                let mut v = FdVec::new(CAP).unwrap();
                for i in 0..PREFILL {
                    v.push(i).unwrap();
                }
                v
            },
            |mut v| {
                black_box(v.pop());
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/pop", |b| {
        b.iter_batched(
            || {
                let mut v = Vec::<u64>::with_capacity(CAP as usize);
                for i in 0..PREFILL {
                    v.push(i);
                }
                v
            },
            |mut v| {
                black_box(v.pop());
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("fd/get", |b| {
        let mut v = FdVec::new(CAP).unwrap();
        for i in 0..PREFILL {
            v.push(i).unwrap();
        }
        b.iter(|| black_box(v.get(black_box(250))));
    });

    g.bench_function("std/get", |b| {
        let mut v = Vec::<u64>::with_capacity(CAP as usize);
        for i in 0..PREFILL {
            v.push(i);
        }
        b.iter(|| black_box(v.get(black_box(250))));
    });

    g.bench_function("fd/set", |b| {
        let mut v = FdVec::new(CAP).unwrap();
        for i in 0..PREFILL {
            v.push(i).unwrap();
        }
        b.iter(|| v.set(black_box(250), black_box(42)).unwrap());
    });

    g.bench_function("std/set", |b| {
        let mut v = Vec::<u64>::with_capacity(CAP as usize);
        for i in 0..PREFILL {
            v.push(i);
        }
        b.iter(|| v[black_box(250)] = black_box(42));
    });

    g.bench_function("fd/batch_push", |b| {
        b.iter_batched(
            || FdVec::new(CAP).unwrap(),
            |mut v| {
                for i in 0..500u64 {
                    v.push(black_box(i)).unwrap();
                }
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.bench_function("std/batch_push", |b| {
        b.iter_batched(
            || Vec::<u64>::with_capacity(CAP as usize),
            |mut v| {
                for i in 0..500u64 {
                    v.push(black_box(i));
                }
                v
            },
            BatchSize::SmallInput,
        );
    });

    g.finish();
}

criterion_group!(
    benches,
    map_benches,
    set_benches,
    stack_benches,
    queue_benches,
    deque_benches,
    heap_benches,
    pool_benches,
    vec_benches,
);
criterion_main!(benches);
