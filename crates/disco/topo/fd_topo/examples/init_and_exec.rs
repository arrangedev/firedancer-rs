use std::{ffi::CStr, thread::sleep, time::Duration};

use fd_topo::{
    fd_info, fd_notice,
    types::{ActiveObject, ActiveTile, ActiveTopology},
    CpuTopology, ObjectCallbacks, PageSize, Result, SandboxConfig, TileExecutionMode, TileRunner,
    TileRunnerRegistry, Topo, TopoBuilder, TopologyCallbacks,
};

const PROGNAME: &'static CStr = c"tachyon_fd";
const VERIFY_NAMES: [&'static CStr; 6] = [
    c"verify_0",
    c"verify_1",
    c"verify_2",
    c"verify_3",
    c"verify_4",
    c"verify_5",
];

const BANK_NAMES: [(&'static CStr, &'static CStr); 2] = [
    (c"bank_acc_0", c"bank_prog_0"),
    (c"bank_acc_1", c"bank_prog_1"),
];

const METRIC_TILE_LINKS: [&'static CStr; 5] = [c"net", c"quic", c"verify", c"pack", c"bank"];

// created by fd_topob_tile for each tile, fd_topob_link for each link
const AUTO_OBJECTS: [&'static CStr; 6] = [
    c"tile",
    c"metrics",
    c"keyswitch",
    c"mcache",
    c"dcache",
    c"fseq",
];

const ALL_OBJECTS: [&'static CStr; 13] = [
    c"net_rx_buf",
    c"net_tx_buf",
    c"quic_conn",
    c"quic_stream",
    c"verify_0",
    c"verify_1",
    c"pack_pending",
    c"pack_micro",
    c"bank_acc_0",
    c"bank_acc_1",
    c"bank_prog_0",
    c"bank_prog_1",
    c"metrd",
];

fn main() -> Result<()> {
    let cpu_topo = match std::env::var("FD_CPU_METHOD").as_deref() {
        Ok("thin") => CpuTopology::new_simple(PROGNAME)?,
        Ok("full") => {
            let cpu_count = std::env::var("FD_CPU_COUNT")
                .unwrap_or_else(|_| "6".to_string())
                .parse::<usize>()
                .unwrap_or(6);
            let numa_count = std::env::var("FD_NUMA_COUNT")
                .unwrap_or_else(|_| "1".to_string())
                .parse::<usize>()
                .unwrap_or(1);

            println!("> [cpu-cfg] FD_CPU_METHOD=manual, cpus={cpu_count}, numa={numa_count}",);
            CpuTopology::new_custom(PROGNAME, cpu_count, numa_count)?
        }
        _ => match CpuTopology::new_simple(PROGNAME) {
            Ok(topo) => topo,
            Err(_) => CpuTopology::new_custom(PROGNAME, 6, 1)?,
        },
    };

    println!(
        "> [cpu-cfg] cpus={}, numa-nodes={}",
        cpu_topo.cpu_count(),
        cpu_topo.numa_node_count()
    );

    for numa_node in 0..cpu_topo.numa_node_count() {
        let cpus_on_node = cpu_topo.cpus_on_numa_node(numa_node);
        println!(
            "   >> [numa-node-{}] num-cpus={}, cpus=[{}]",
            numa_node,
            cpus_on_node.len(),
            cpus_on_node
                .iter()
                .map(|cpu| cpu.idx.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    #[cfg(not(target_os = "linux"))]
    println!("os={}", std::env::consts::OS);

    let mut builder = TopoBuilder::new(c"initandexec")?;

    create_workspaces(&mut builder)?;
    create_links(&mut builder)?;
    create_tiles(&mut builder)?;
    wire_topology(&mut builder)?;

    // auto layout won't work on some machines, and we've already manually laid out topology
    // builder.auto_layout(false)?;

    let mut callbacks = create_callbacks()?;
    let callback_ptr = callbacks.finalize()?;

    let mut tile_registry = create_tile_runners()?;
    tile_registry.finalize()?;

    let use_anonymous = std::env::var("FD_USE_ANON_WKSP")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(true);

    let mut topo = if use_anonymous {
        println!("> [build] using anonymous wksps (mem-backed)");
        check_meminfo();

        let page_size =
            std::env::var("FD_PAGE_SIZE")
                .ok()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "normal" | "4k" => Some(PageSize::Normal),
                    "huge" | "2m" => Some(PageSize::Huge),
                    "gigantic" | "1g" => Some(PageSize::Gigantic),
                    _ => None,
                });

        if let Some(ref ps) = page_size {
            let page_name = match ps {
                PageSize::Normal => "normal (4KB)",
                PageSize::Huge => "huge (2MB)",
                PageSize::Gigantic => "gigantic (1GB)",
            };
            println!("   >> using {} pages (via FD_PAGE_SIZE)", page_name);
        } else {
            println!("   >> using default page_sz (set FD_PAGE_SIZE=normal)");
            println!("   >> run [vendor_path]/util/shmem/fd_shmem_cfg alloc [page_cnt] [page_sz] [numa_node]");
        }

        builder.build_anonymous(callback_ptr, Some(PageSize::Normal))?
    } else {
        println!("> [build] using wksps (disk-backed)");
        builder.build(callback_ptr, false)?
    };

    analyze_topology(&topo)?;

    // Configure sandboxing - disabled by default for compatibility
    let sandbox_config = match std::env::var("FD_SANDBOX") {
        Ok(val) if val == "1" || val.to_lowercase() == "true" => {
            println!("   >> sandboxing enabled (via FD_SANDBOX)");
            SandboxConfig::enabled().with_stdio() // Allow stdio for debugging
        }
        _ => {
            println!("   >> sandboxing disabled (set FD_SANDBOX=1 to enable)");
            SandboxConfig::disabled()
        }
    };

    simulate_execution(&mut topo, &tile_registry, &sandbox_config)?;

    Ok(())
}

fn create_workspaces(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [wksp] creating workspaces");

    builder.add_wksp(c"net")?;
    println!("   >> ✓ net_wksp");

    builder.add_wksp(c"pack")?;
    println!("   >> ✓ pack_wksp");

    builder.add_wksp(c"bank")?;
    println!("   >> ✓ bank_wksp");

    builder.add_wksp(c"metric")?;
    println!("   >> ✓ metric_wksp");

    println!("     >>> ✓ wksps created");

    Ok(())
}

fn create_links(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [link] creating links");

    builder.add_link(c"net_quic", c"net", 1024, 2048, 16)?;
    println!("   >> ✓ link: net_quic (depth: 1024, mtu: 2048)");

    builder.add_link(c"quic_verify", c"pack", 2048, 1500, 32)?;
    println!("   >> ✓ link: quic_verify (depth: 2048, mtu: 1500)");

    builder.add_link(c"verify_pack", c"pack", 1024, 1024, 16)?;
    println!("   >> ✓ link: verify_pack (depth: 1024, mtu: 1024)");

    builder.add_link(c"verify_pack", c"pack", 1024, 1024, 16)?;
    println!("   >> ✓ link: verify_pack (depth: 1024, mtu: 1024)");

    builder.add_link(c"pack_bank", c"bank", 512, 4096, 8)?;
    println!("   >> ✓ link: pack_bank (depth: 512, mtu: 4096)");

    for i in 0..5 {
        builder.add_link(c"metric", c"metric", 256, 512, 4)?;
    }
    println!("   >> ✓ link: metric_collect (depth: 256, mtu: 512)");

    println!("     >>> ✓ links created");

    Ok(())
}

fn create_tiles(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [tile] creating tiles");

    builder.add_tile(c"net", c"net", c"metric", Some(0), false, false)?;
    builder.add_object(c"net_rx_buf", c"net")?;
    builder.add_object(c"net_tx_buf", c"net")?;
    println!("   >> ✓ net (cpuid=0)");

    builder.add_tile(c"quic", c"net", c"metric", Some(1), false, false)?;
    builder.add_object(c"quic_conn", c"net")?;
    builder.add_object(c"quic_stream", c"net")?;
    println!("   >> ✓ quic (cpuid=1)");

    for i in 0..2 {
        builder.add_tile(c"verify", c"pack", c"metric", Some(2 + i), false, false)?;
        builder.add_object(VERIFY_NAMES[i], c"pack")?;
        println!("   >> ✓ verify {} (cpuid={})", i, 2 + i);
    }

    builder.add_tile(c"pack", c"pack", c"metric", Some(4), false, false)?;
    builder.add_object(c"pack_pending", c"pack")?;
    builder.add_object(c"pack_micro", c"pack")?;
    println!("   >> ✓ pack (cpuid=4)");

    for i in 0..2 {
        builder.add_tile(c"bank", c"bank", c"metric", Some(5 + i), false, false)?;
        builder.add_object(BANK_NAMES[i].0, c"bank")?;
        builder.add_object(BANK_NAMES[i].1, c"bank")?;
        println!("   >> ✓ bank {} (cpuid={})", i, 5 + i);
    }

    builder.add_tile(c"metric", c"metric", c"metric", Some(7), false, false)?;
    builder.add_object(c"metrd", c"metric")?;
    println!("   >> ✓ metric (cpuid=7)");

    println!("     >>> ✓ tiles created");

    Ok(())
}

fn wire_topology(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [tile] wiring tiles");

    builder.add_tile_output(c"net", 0, c"net_quic", 0)?;
    builder.add_tile_input(c"quic", 0, c"net", c"net_quic", 0, true, true)?;
    builder.add_tile_output(c"quic", 0, c"quic_verify", 0)?;
    println!("   >> ✓ 'net' -> 'quic' (cpuid=0)");
    println!("   >> ✓ 'quic' -> 'net' (cpuid=0)");
    println!("   >> ✓ 'quic' -> 'verify' (cpuid=0)");

    for i in 0..2 {
        builder.add_tile_input(c"verify", i, c"pack", c"quic_verify", 0, true, true)?;
        builder.add_tile_output(c"verify", i, c"verify_pack", i)?; // Use different link_kind_id for each verify tile
    }

    println!("   >> ✓ 'quic' -> 'verify' (cpuid=0)");
    println!("   >> ✓ 'verify' -> 'pack' (cpuid=1)");

    for i in 0..2 {
        builder.add_tile_input(c"pack", 0, c"pack", c"verify_pack", i, true, true)?;
    }
    builder.add_tile_output(c"pack", 0, c"pack_bank", 0)?;

    println!("   >> ✓ 'verify' -> 'pack' (cpuid=1)");
    println!("   >> ✓ 'pack' -> 'bank' (cpuid=1)");

    for i in 0..2 {
        builder.add_tile_input(c"bank", i, c"bank", c"pack_bank", 0, true, true)?;
    }

    for (i, tile_name) in METRIC_TILE_LINKS.iter().enumerate() {
        builder.add_tile_output(tile_name, 0, c"metric", i)?;
    }

    for i in 0..5 {
        builder.add_tile_input(c"metric", 0, c"metric", c"metric", i, false, true)?;
    }

    println!("   >> ✓ 'pack' -> 'metric' (cpuid=1)");
    println!("   >> ✓ 'bank' -> 'metric' (cpuid=1)");
    println!("   >> ✓ 'quic' -> 'metric' (cpuid=1)");
    println!("   >> ✓ 'verify' -> 'metric' (cpuid=1)");
    println!("   >> ✓ 'net' -> 'metric' (cpuid=1)");

    println!("     >>> ✓ topology wired");
    Ok(())
}

fn analyze_topology(topo: &fd_topo::Topo) -> Result<()> {
    println!("> [topo] analyzing structure");

    println!(
        "   >> wksps={} links={} tiles={} objs={}",
        topo.workspace_cnt(),
        topo.link_cnt(),
        topo.tile_cnt(),
        topo.object_cnt()
    );

    if let Some(net_wksp_id) = topo.find_wksp("net") {
        println!("   >> ✓ wksp=net id={}", net_wksp_id);
    }

    if let Some(pack_tile_id) = topo.find_tile("pack", 0) {
        println!("   >> ✓ tile=pack id={}", pack_tile_id);
    }

    if let Some(verify_link_id) = topo.find_link("quic_verify", 0) {
        println!("   >> ✓ link=quic_verify id={}", verify_link_id);
    }

    let max_tile_mlock = topo.max_tile_mlock();
    let total_mlock = topo.total_mlock();

    println!(
        "   >> mem: max_tile_mlock={} total_mlock={}",
        max_tile_mlock / (1024 * 1024),
        total_mlock / (1024 * 1024)
    );

    let verify_tile_count = topo.tile_name_cnt("verify");
    let bank_tile_count = topo.tile_name_cnt("bank");
    println!("   >> parallelism: verify={verify_tile_count} bank={bank_tile_count}",);

    println!("     >>> ✓ topology verified");

    Ok(())
}

fn simulate_execution(
    topo: &mut Topo,
    tile_registry: &TileRunnerRegistry,
    sandbox_config: &SandboxConfig,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        println!("> [wksp] joining workspaces: {}", topo.workspace_cnt());

        match topo.join_wksps(false) {
            Ok(()) => println!("   >> ✓ workspaces joined"),
            Err(e) => eprintln!("   >> ✗ err={e:?}"),
        }

        println!("   >> initializing objects");
        match topo.init_objects() {
            Ok(()) => println!("   >> ✓ objects initialized"),
            Err(e) => eprintln!("   >> ✗ err={e:?}"),
        }

        println!("   >> filling objects");
        topo.fill();

        println!("   >> initializing tile contexts");
        for tile_id in 0..topo.tile_cnt() {
            match topo.join_tile_wksps(tile_id) {
                Ok(()) => match topo.fill_tile(tile_id) {
                    Ok(()) => println!("   >> ✓ {tile_id} initialized"),
                    Err(e) => eprintln!("   >> ✗ err={e:?} tile-{tile_id}"),
                },
                Err(e) => eprintln!("   >> ✗ err={e:?} tile-{tile_id}"),
            }
        }

        topo.print_to_log();

        println!("   >> starting tile exec");
        let uid = unsafe { libc::getuid() };
        let gid = unsafe { libc::getgid() };

        let execution_mode = match std::env::var("FD_EXECUTION_MODE").as_deref() {
            Ok("single") => {
                println!("   >> using single tile execution mode");
                TileExecutionMode::Single
            }
            Ok("isolated") | _ => {
                println!("   >> using isolated tile execution mode (default)");
                TileExecutionMode::Isolated
            }
            _ => {
                println!("   >> using isolated tile execution mode (default)");
                TileExecutionMode::Isolated
            }
        };

        match topo.run_tiles(uid, gid, tile_registry, sandbox_config, execution_mode) {
            Ok(()) => println!("   >> ✓ tiles started"),
            Err(e) => eprintln!("   >> ✗ err={e:?}"),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!(
            "   >> workspaces={}, tiles={}, links={}",
            topo.workspace_cnt(),
            topo.tile_cnt(),
            topo.link_cnt()
        );
    }

    Ok(())
}

unsafe extern "C" fn populate_stdio_fds(
    _topo: *const ActiveTopology,
    _tile: *const ActiveTile,
    out_fds_sz: u64,
    out_fds: *mut i32,
) -> u64 {
    if out_fds_sz >= 3 {
        *out_fds.offset(0) = 0; // stdin
        *out_fds.offset(1) = 1; // stdout
        *out_fds.offset(2) = 2; // stderr
        3
    } else {
        0
    }
}

fn create_tile_runners() -> Result<TileRunnerRegistry> {
    let mut registry = TileRunnerRegistry::new();

    registry
        .add_runner(TileRunner::new(c"net", net_tile_run).with_allowed_fds(populate_stdio_fds))?;
    registry
        .add_runner(TileRunner::new(c"quic", quic_tile_run).with_allowed_fds(populate_stdio_fds))?;
    registry.add_runner(
        TileRunner::new(c"verify", verify_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;
    registry
        .add_runner(TileRunner::new(c"pack", pack_tile_run).with_allowed_fds(populate_stdio_fds))?;
    registry
        .add_runner(TileRunner::new(c"bank", bank_tile_run).with_allowed_fds(populate_stdio_fds))?;
    registry.add_runner(
        TileRunner::new(c"metric", metric_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;

    Ok(registry)
}

unsafe extern "C" fn net_tile_run(_topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [net] {} starting", (*tile).id);

    let mut iteration = 0u64;
    loop {
        sleep(Duration::from_millis(100));
        if iteration % 30 == 0 {
            fd_info!(
                "    >> {}: processing packets (iteration {})",
                (*tile).id,
                iteration
            );
        }
        iteration = iteration.wrapping_add(1);
    }
}

unsafe extern "C" fn quic_tile_run(_topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [quic] {} starting", (*tile).id);

    let mut iteration = 0u64;
    loop {
        sleep(Duration::from_millis(150));
        if iteration % 20 == 0 {
            fd_info!(
                "    >> {}: processing connections (iteration {})",
                (*tile).id,
                iteration
            );
        }
        iteration = iteration.wrapping_add(1);
    }
}

unsafe extern "C" fn verify_tile_run(_topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!(
        "> [verify] {} (kind {}) starting",
        (*tile).id,
        (*tile).kind_id
    );

    let mut iteration = 0u64;
    loop {
        sleep(Duration::from_millis(200));
        if iteration % 15 == 0 {
            fd_info!(
                "    >> {}: verifying signatures (iteration {})",
                (*tile).id,
                iteration
            );
        }
        iteration = iteration.wrapping_add(1);
    }
}

unsafe extern "C" fn pack_tile_run(_topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [pack] {} starting", (*tile).id);

    let mut iteration = 0u64;
    loop {
        sleep(Duration::from_millis(300));
        if iteration % 10 == 0 {
            fd_info!(
                "    >> {}: packing transactions (iteration {})",
                (*tile).id,
                iteration
            );
        }
        iteration = iteration.wrapping_add(1);
    }
}

unsafe extern "C" fn bank_tile_run(_topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!(
        "> [bank] {} (kind {}) starting",
        (*tile).id,
        (*tile).kind_id
    );

    let mut iteration = 0u64;
    loop {
        sleep(Duration::from_millis(400));
        if iteration % 7 == 0 {
            fd_info!(
                "    >> {} (kind {}): processing operations (iteration {})",
                (*tile).id,
                (*tile).kind_id,
                iteration
            );
        }
        iteration = iteration.wrapping_add(1);
    }
}

unsafe extern "C" fn metric_tile_run(_topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [metric] {} starting", (*tile).id);

    let mut iteration = 0u64;
    loop {
        sleep(Duration::from_millis(250));
        if iteration % 12 == 0 {
            fd_info!(
                "    >> {}: collecting metrics (iteration {})",
                (*tile).id,
                iteration
            );
        }
        iteration = iteration.wrapping_add(1);
    }
}

fn create_callbacks() -> Result<TopologyCallbacks> {
    let mut registry = TopologyCallbacks::new();

    for obj_name in ALL_OBJECTS.iter().chain(AUTO_OBJECTS.iter()) {
        registry.add_callback(ObjectCallbacks::new(
            *obj_name,
            basic_footprint,
            basic_align,
        ))?;
    }

    Ok(registry)
}

unsafe extern "C" fn basic_footprint(
    _topo: *const ActiveTopology,
    _obj: *const ActiveObject,
) -> u64 {
    4096
}

unsafe extern "C" fn basic_align(_topo: *const ActiveTopology, _obj: *const ActiveObject) -> u64 {
    64
}

fn check_meminfo() {
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut available_kb = 0;
            let mut hugepages_total = 0;
            let mut hugepages_free = 0;

            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        available_kb = value.parse::<u64>().unwrap_or(0);
                    }
                } else if line.starts_with("HugePages_Total:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        hugepages_total = value.parse::<u64>().unwrap_or(0);
                    }
                } else if line.starts_with("HugePages_Free:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        hugepages_free = value.parse::<u64>().unwrap_or(0);
                    }
                }
            }

            println!("   >> system memory: {}MB available", available_kb / 1024);
            if hugepages_total > 0 {
                println!(
                    "   >> huge pages: {}/{} free (2MB each)",
                    hugepages_free, hugepages_total
                );
            } else {
                println!("   >> huge pages: not configured (will use normal pages)");
            }

            if available_kb < 100 * 1024 {
                // Less than 100MB
                println!("   >> WARNING: Low available memory ({}MB). Consider freeing memory or reducing topology size.", available_kb / 1024);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("   >> memory check not available on this platform");
    }
}
