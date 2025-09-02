//! example may not work in all environments since the tile system
//! requires proper boot and target env

use fd_tile::{StackInfo, Tile, TileInfo};

fn main() {
    let tile_info = Tile::current_info();
    print_tile(&tile_info);

    let stack_info = Tile::stack_info();
    print_stack(&stack_info);
    print_cpu_set(&tile_info);

    println!("current-tile-id: {}", fd_tile::current_tile_id());
    println!("current-tile-idx: {}", fd_tile::current_tile_idx());
    println!("total-tile-cnt: {}", fd_tile::tile_count());
    let (id0, id1) = fd_tile::thread_group_range();
    println!("thread-group-range: [{}, {})", id0, id1);
}

fn print_tile(info: &TileInfo) {
    println!("thread-group-id-range: [{}, {})", info.id0, info.id1);
    println!("current-tile-id: {}", info.id);
    println!("current-tile-idx: {}", info.idx);
    println!("total-tiles-group: {}", info.count);

    if info.count == 0 {
        println!("× warning: tile system appears uninitialized (count = 0)");
    } else if info.count == 1 {
        println!("ℹ running in single-tile mode");
    } else {
        println!("√ multi-tile environment detected");
    }
    println!();
}

fn print_stack(info: &StackInfo) {
    if info.stack0.is_null() || info.stack1.is_null() {
        println!("× warning: stack information unavailable (not initialized)");
        println!("stack-start: {:?}", info.stack0);
        println!("stack-end: {:?}", info.stack1);
        println!("stack-size: {} bytes", info.size);
        println!();
        return;
    }

    println!("stack-start: {:p}", info.stack0);
    println!("stack-end: {:p}", info.stack1);
    println!(
        "stack-size: {} bytes ({:.2} MiB)",
        info.size,
        info.size as f64 / (1024.0 * 1024.0)
    );
    println!(
        "est-used: {} bytes ({:.2} KiB)",
        info.used,
        info.used as f64 / 1024.0
    );
    println!(
        "est-free: {} bytes ({:.2} KiB)",
        info.free,
        info.free as f64 / 1024.0
    );

    if info.size > 0 {
        let usage_percent = (info.used as f64 / info.size as f64) * 100.0;
        println!("stack-usage: {:.1}%", usage_percent);

        if usage_percent > 80.0 {
            println!("× high stack usage!");
        } else if usage_percent > 50.0 {
            println!("× moderate stack usage");
        } else {
            println!("√ stack usage healthy");
        }
    }
    println!();
}

fn print_cpu_set(info: &TileInfo) {
    if info.count == 0 {
        eprintln!("no tiles to query");
        return;
    }

    for tile_idx in 0..info.count {
        match Tile::cpu_id(tile_idx) {
            Some(cpu_id) => {
                let marker = if tile_idx == info.idx {
                    " <- current"
                } else {
                    ""
                };
                println!("tile {tile_idx}: cpu {cpu_id}{marker}");
            }
            None => {
                let marker = if tile_idx == info.idx {
                    " <- current"
                } else {
                    ""
                };
                println!("tile {tile_idx}: floating/unassigned{marker}");
            }
        }
    }
    println!();
}
