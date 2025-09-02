use fd_tpool::task::Task;
use fd_tpool::{partition, task};
use std::sync::{Arc, Mutex};

fn main() {
    let total_tasks = 100;
    let worker_count = 4;

    println!(
        "   partitioning {} tasks across {} workers:",
        total_tasks, worker_count
    );

    for worker_idx in 0..worker_count {
        let (start, end) = partition::partition_range(0, total_tasks, 1, worker_idx, worker_count);

        println!(
            "   worker-{}: tasks {}-{} ({} tasks)",
            worker_idx,
            start,
            end,
            end - start
        );
    }

    let total_tasks = 101;
    let worker_count = 4;

    println!(
        "   partitioning {} tasks across {} workers:",
        total_tasks, worker_count
    );

    let mut total_assigned = 0;
    for worker_idx in 0..worker_count {
        let (start, end) = partition::partition_range(0, total_tasks, 1, worker_idx, worker_count);

        let task_count = end - start;
        total_assigned += task_count;

        println!(
            "   worker-{}: tasks {}-{} ({} tasks)",
            worker_idx, start, end, task_count
        );
    }

    println!(
        "   total-tasks-assigned: {} (should equal {})",
        total_assigned, total_tasks
    );

    let total_tasks = 1_000_000;
    let worker_count = 16;

    println!(
        "   partitioning {} tasks across {} workers:",
        total_tasks, worker_count
    );

    let mut min_tasks = usize::MAX;
    let mut max_tasks = 0;

    for worker_idx in 0..worker_count {
        let (start, end) = partition::partition_range(0, total_tasks, 1, worker_idx, worker_count);

        let task_count = end - start;
        min_tasks = min_tasks.min(task_count);
        max_tasks = max_tasks.max(task_count);

        if worker_idx < 4 || worker_idx >= worker_count - 2 {
            println!(
                "   worker-{}: tasks {}-{} ({} tasks)",
                worker_idx, start, end, task_count
            );
        } else if worker_idx == 4 {
            println!("   ... (workers-4-{}) ...", worker_count - 3);
        }
    }

    println!(
        "   task-distribution: min={}, max={}, difference={}",
        min_tasks,
        max_tasks,
        max_tasks - min_tasks
    );

    let counter = Arc::new(Mutex::new(0u32));
    let counter_clone = counter.clone();

    let increment_task = task::ClosureTask::new(move |worker_idx, worker_count| {
        println!(
            "   task executing: worker={}, worker-count={}",
            worker_idx, worker_count
        );

        if let Ok(mut count) = counter_clone.lock() {
            *count += 1;
            println!("   counter incremented: {}", *count);
        }
    });

    increment_task.execute(0, 1);

    if let Ok(count) = counter.lock() {
        println!("   final counter value: {}", *count);
    }

    println!("   Scenario: 3 tasks, 8 workers");
    for worker_idx in 0..8 {
        let (start, end) = partition::partition_range(0, 3, 1, worker_idx, 8);
        if end > start {
            println!("   worker-{}: tasks {}-{}", worker_idx, start, end);
        } else {
            println!("   worker-{}: no tasks", worker_idx);
        }
    }

    println!("\n   Scenario: 100 tasks, 4 workers, 4 lanes per worker");
    for worker_idx in 0..4 {
        let (start, end) = partition::partition_range(0, 100, 4, worker_idx, 4);
        println!(
            "   worker-{}: tasks {}-{} ({} tasks, {} SIMD blocks)",
            worker_idx,
            start,
            end,
            end - start,
            (end - start) / 4
        );
    }

    let start_time = std::time::Instant::now();
    let iterations = 1_000_000;

    for _ in 0..iterations {
        let _partition = partition::partition_range(0, 1000, 1, 5, 16);
    }

    let duration = start_time.elapsed();
    println!(
        "   partition-calculations: {} in {:?}",
        iterations, duration
    );
    println!("   avg_time_per_partition: {:?}", duration / iterations);

    let (start, end) = partition::partition_range(10, 10, 1, 0, 4);
    println!("   empty range (10,10): worker-0 gets {}-{}", start, end);

    let (start, end) = partition::partition_range(0, 1, 1, 0, 4);
    println!("   single task: worker-0 gets {}-{}", start, end);
    let (start, end) = partition::partition_range(0, 1, 1, 1, 4);
    println!("   single task: worker-1 gets {}-{}", start, end);

    let (start, end) = partition::partition_range(0, 100, 1, 10, 4);
    println!("   invalid worker index (10 >= 4): gets {}-{}", start, end);

    struct WorkItem {
        id: usize,
        data: Vec<f32>,
    }
    let dataset: Vec<WorkItem> = (0..1000)
        .map(|i| WorkItem {
            id: i,
            data: vec![i as f32; 10],
        })
        .collect();

    let worker_count = 8;
    println!(
        "   processing {} items with {} workers:",
        dataset.len(),
        worker_count
    );

    for worker_idx in 0..worker_count {
        let (start, end) =
            partition::partition_range(0, dataset.len(), 1, worker_idx, worker_count);

        if end > start {
            let worker_items = &dataset[start..end];
            let sum: f32 = worker_items.iter().flat_map(|item| &item.data).sum();

            println!(
                "   worker-{}: processes items {}-{}, computed sum = {:.1}",
                worker_idx, start, end, sum
            );
        }
    }
}
