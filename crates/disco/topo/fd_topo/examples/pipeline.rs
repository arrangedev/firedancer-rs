use std::{
    ffi::CStr,
    fs::OpenOptions,
    io::Write,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use fd_topo::{
    fd_debug, fd_info, fd_notice, fd_warn,
    types::{ActiveObject, ActiveTile, ActiveTopology},
    CpuTopology, ObjectCallbacks, PageSize, Result, SandboxConfig, TileExecutionMode, TileRunner,
    TileRunnerRegistry, Topo, TopoBuilder, TopologyCallbacks,
};

const PROGNAME: &'static CStr = c"pipeline";

const COLLECTOR_WKSP: &'static CStr = c"collect";
const METRICS_WKSP: &'static CStr = c"metric";
const PROCESSING_WKSP: &'static CStr = c"proc";
const OUTPUT_WKSP: &'static CStr = c"output";

const CPUMEM_TILE: &'static CStr = c"cpumem"; // 6 chars - OK
const DISK_TILE: &'static CStr = c"disk"; // 4 chars - OK
const NETWORK_TILE: &'static CStr = c"net"; // 3 chars - OK
const PROCESSOR_TILE: &'static CStr = c"proc"; // 4 chars - OK
const WRITER_TILE: &'static CStr = c"writer"; // 6 chars - OK

const CPUMEM_LINK: &'static CStr = c"cm_to_pr";
const DISK_LINK: &'static CStr = c"dk_to_pr";
const NETWORK_LINK: &'static CStr = c"nt_to_pr";
const PROCESSOR_LINK: &'static CStr = c"pr_to_wr";
const METRICS_LINK: &'static CStr = c"metr_coll";

const CPUMEM_OBJECT: &'static CStr = c"cpumem_data"; // 11 chars - OK
const DISK_OBJECT: &'static CStr = c"disk_data"; // 9 chars - OK
const NETWORK_OBJECT: &'static CStr = c"net_data"; // 8 chars - OK
const PROCESSOR_OBJECT: &'static CStr = c"proc_data"; // 9 chars - OK

// Mutable statics are generally a bad idea, but fuck it
static mut PREV_CPU_METRICS: Option<CpuMemMetrics> = None;
static mut PREV_DISK_METRICS: Option<DiskMetrics> = None;
static mut PREV_NETWORK_METRICS: Option<NetworkMetrics> = None;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CpuMemMetrics {
    timestamp: u64,
    cpu_user: u64,
    cpu_nice: u64,
    cpu_system: u64,
    cpu_idle: u64,
    cpu_iowait: u64,
    mem_total: u64,
    mem_available: u64,
    mem_free: u64,
    mem_cached: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DiskMetrics {
    timestamp: u64,
    reads_completed: u64,
    reads_merged: u64,
    sectors_read: u64,
    time_reading: u64,
    writes_completed: u64,
    writes_merged: u64,
    sectors_written: u64,
    time_writing: u64,
    io_in_progress: u64,
    time_io: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct NetworkMetrics {
    timestamp: u64,
    rx_bytes: u64,
    rx_packets: u64,
    rx_errors: u64,
    rx_dropped: u64,
    tx_bytes: u64,
    tx_packets: u64,
    tx_errors: u64,
    tx_dropped: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AggregatedMetrics {
    timestamp: u64,
    cpu_usage_percent: f32,
    memory_usage_percent: f32,
    disk_read_rate: f32,
    disk_write_rate: f32,
    network_rx_rate: f32,
    network_tx_rate: f32,
    io_wait_percent: f32,
}

const METRIC_OBJECTS: [&'static CStr; 4] =
    [CPUMEM_OBJECT, DISK_OBJECT, NETWORK_OBJECT, PROCESSOR_OBJECT];

const AUTO_OBJECTS: [&'static CStr; 6] = [
    c"tile",
    c"metrics",
    c"keyswitch",
    c"mcache",
    c"dcache",
    c"fseq",
];

fn main() -> Result<()> {
    let cpu_topo = match std::env::var("FD_CPU_METHOD").as_deref() {
        Ok("thin") => CpuTopology::new_simple(PROGNAME)?,
        Ok("full") => {
            let cpu_count = std::env::var("FD_CPU_COUNT")
                .unwrap_or_else(|_| "8".to_string())
                .parse::<usize>()
                .unwrap_or(8);
            let numa_count = std::env::var("FD_NUMA_COUNT")
                .unwrap_or_else(|_| "1".to_string())
                .parse::<usize>()
                .unwrap_or(1);

            println!("> [cpu-cfg] FD_CPU_METHOD=full, cpus={cpu_count}, numa={numa_count}");
            CpuTopology::new_custom(PROGNAME, cpu_count, numa_count)?
        }
        _ => match CpuTopology::new_simple(PROGNAME) {
            Ok(topo) => topo,
            Err(_) => CpuTopology::new_custom(PROGNAME, 8, 1)?,
        },
    };

    println!(
        "> [cpu-cfg] cpus={}, numa-nodes={}",
        cpu_topo.cpu_count(),
        cpu_topo.numa_node_count()
    );

    let mut builder = TopoBuilder::new(c"pipeline")?;

    create_wksps(&mut builder)?;
    create_links(&mut builder)?;
    create_tiles(&mut builder)?;
    wire_topology(&mut builder)?;

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

    verify_layout(&topo)?;

    let sandbox_config = match std::env::var("FD_SANDBOX") {
        Ok(val) if val == "1" || val.to_lowercase() == "true" => {
            println!("   >> sandboxing enabled (via FD_SANDBOX)");
            SandboxConfig::enabled().with_stdio()
        }
        _ => {
            println!("   >> sandboxing disabled (set FD_SANDBOX=1 to enable)");
            SandboxConfig::disabled()
        }
    };

    exec(&mut topo, &tile_registry, &sandbox_config)?;

    Ok(())
}

fn create_wksps(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [wksp] creating workspaces");

    builder.add_wksp(COLLECTOR_WKSP)?;
    println!("   >> ✓ collect_wksp");

    builder.add_wksp(PROCESSING_WKSP)?;
    println!("   >> ✓ proc_wksp");

    builder.add_wksp(OUTPUT_WKSP)?;
    println!("   >> ✓ output_wksp");

    builder.add_wksp(METRICS_WKSP)?;
    println!("   >> ✓ metric_wksp");

    println!("     >>> ✓ wksps created");
    Ok(())
}

fn create_links(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [link] creating links");

    builder.add_link(CPUMEM_LINK, COLLECTOR_WKSP, 512, 1024, 8)?;
    println!("   >> ✓ link: cm_to_pr (depth: 512, mtu: 1024)");

    builder.add_link(DISK_LINK, COLLECTOR_WKSP, 512, 1024, 8)?;
    println!("   >> ✓ link: dk_to_pr (depth: 512, mtu: 1024)");

    builder.add_link(NETWORK_LINK, COLLECTOR_WKSP, 512, 1024, 8)?;
    println!("   >> ✓ link: nt_to_pr (depth: 512, mtu: 1024)");

    builder.add_link(PROCESSOR_LINK, PROCESSING_WKSP, 256, 2048, 4)?;
    println!("   >> ✓ link: pr_to_wr (depth: 256, mtu: 2048)");

    for i in 0..5 {
        builder.add_link(METRICS_LINK, METRICS_WKSP, 256, 512, 4)?;
    }
    println!("   >> ✓ link: metric_collect (depth: 256, mtu: 512)");

    println!("     >>> ✓ links created");
    Ok(())
}

fn create_tiles(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [tile] creating tiles");

    builder.add_tile(
        CPUMEM_TILE,
        COLLECTOR_WKSP,
        METRICS_WKSP,
        Some(0),
        false,
        false,
    )?;
    builder.add_object(CPUMEM_OBJECT, COLLECTOR_WKSP)?;
    println!("   >> ✓ cpumem (cpuid=0)");

    builder.add_tile(
        DISK_TILE,
        COLLECTOR_WKSP,
        METRICS_WKSP,
        Some(1),
        false,
        false,
    )?;
    builder.add_object(DISK_OBJECT, COLLECTOR_WKSP)?;
    println!("   >> ✓ disk (cpuid=1)");

    builder.add_tile(
        NETWORK_TILE,
        COLLECTOR_WKSP,
        METRICS_WKSP,
        Some(2),
        false,
        false,
    )?;
    builder.add_object(NETWORK_OBJECT, COLLECTOR_WKSP)?;
    println!("   >> ✓ network (cpuid=2)");

    builder.add_tile(
        PROCESSOR_TILE,
        PROCESSING_WKSP,
        METRICS_WKSP,
        Some(3),
        false,
        false,
    )?;
    builder.add_object(PROCESSOR_OBJECT, PROCESSING_WKSP)?;
    println!("   >> ✓ processor (cpuid=3)");

    builder.add_tile(
        WRITER_TILE,
        OUTPUT_WKSP,
        METRICS_WKSP,
        Some(4),
        false,
        false,
    )?;
    println!("   >> ✓ writer (cpuid=4)");

    println!("     >>> ✓ tiles created");
    Ok(())
}

fn wire_topology(builder: &mut TopoBuilder) -> Result<()> {
    println!("> [tile] wiring tiles");

    builder.add_tile_output(CPUMEM_TILE, 0, CPUMEM_LINK, 0)?;
    builder.add_tile_input(
        PROCESSOR_TILE,
        0,
        PROCESSING_WKSP,
        CPUMEM_LINK,
        0,
        true,
        true,
    )?;

    builder.add_tile_output(DISK_TILE, 0, DISK_LINK, 0)?;
    builder.add_tile_input(PROCESSOR_TILE, 0, PROCESSING_WKSP, DISK_LINK, 0, true, true)?;

    builder.add_tile_output(NETWORK_TILE, 0, NETWORK_LINK, 0)?;
    builder.add_tile_input(
        PROCESSOR_TILE,
        0,
        PROCESSING_WKSP,
        NETWORK_LINK,
        0,
        true,
        true,
    )?;

    builder.add_tile_output(PROCESSOR_TILE, 0, PROCESSOR_LINK, 0)?;
    builder.add_tile_input(WRITER_TILE, 0, OUTPUT_WKSP, PROCESSOR_LINK, 0, true, true)?;

    let metric_tiles = [
        CPUMEM_TILE,
        DISK_TILE,
        NETWORK_TILE,
        PROCESSOR_TILE,
        WRITER_TILE,
    ];
    for (i, tile_name) in metric_tiles.iter().enumerate() {
        builder.add_tile_output(tile_name, 0, METRICS_LINK, i)?;
    }

    builder.add_tile(c"metric", METRICS_WKSP, METRICS_WKSP, Some(5), false, false)?;
    println!("   >> ✓ metric (cpuid=5)");

    for i in 0..5 {
        builder.add_tile_input(c"metric", 0, METRICS_WKSP, METRICS_LINK, i, false, true)?;
    }

    println!("   >> ✓ cpumem -> processor");
    println!("   >> ✓ disk -> processor");
    println!("   >> ✓ network -> processor");
    println!("   >> ✓ processor -> writer");
    println!("   >> ✓ all tiles -> metric");

    println!("     >>> ✓ topology wired");
    Ok(())
}

fn verify_layout(topo: &Topo) -> Result<()> {
    println!("> [topo] analyzing structure");

    println!(
        "   >> wksps={} links={} tiles={} objs={}",
        topo.workspace_cnt(),
        topo.link_cnt(),
        topo.tile_cnt(),
        topo.object_cnt()
    );

    if let Some(collector_wksp_id) = topo.find_wksp(COLLECTOR_WKSP.to_str().unwrap()) {
        println!("   >> ✓ wksp=collector id={}", collector_wksp_id);
    }

    if let Some(cpumem_tile_id) = topo.find_tile(CPUMEM_TILE.to_str().unwrap(), 0) {
        println!("   >> ✓ tile=cpumem id={}", cpumem_tile_id);
    }

    if let Some(cpumem_link_id) = topo.find_link(CPUMEM_LINK.to_str().unwrap(), 0) {
        println!("   >> ✓ link=cpumem id={}", cpumem_link_id);
    }

    let max_tile_mlock = topo.max_tile_mlock();
    let total_mlock = topo.total_mlock();

    println!(
        "   >> mem: max_tile_mlock={}KB total_mlock={}KB",
        max_tile_mlock / 1024,
        total_mlock / 1024
    );

    println!("     >>> ✓ topology verified");
    Ok(())
}

fn exec(
    topo: &mut Topo,
    _tile_registry: &TileRunnerRegistry,
    _sandbox_config: &SandboxConfig,
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

        match topo.run_tiles(uid, gid, _tile_registry, _sandbox_config, execution_mode) {
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

    registry.add_runner(
        TileRunner::new(CPUMEM_TILE, cpumem_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;
    registry.add_runner(
        TileRunner::new(DISK_TILE, disk_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;
    registry.add_runner(
        TileRunner::new(NETWORK_TILE, net_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;
    registry.add_runner(
        TileRunner::new(PROCESSOR_TILE, proc_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;
    registry.add_runner(
        TileRunner::new(WRITER_TILE, writer_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;
    registry.add_runner(
        TileRunner::new(METRICS_WKSP, metric_tile_run).with_allowed_fds(populate_stdio_fds),
    )?;

    Ok(registry)
}

unsafe extern "C" fn cpumem_tile_run(topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [cpumem-tile] {} starting", (*tile).id);

    let mut prev_cpu_total = 0u64;
    let mut prev_cpu_idle = 0u64;

    loop {
        sleep(Duration::from_millis(1000));

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut metrics = CpuMemMetrics {
            timestamp,
            cpu_user: 0,
            cpu_nice: 0,
            cpu_system: 0,
            cpu_idle: 0,
            cpu_iowait: 0,
            mem_total: 0,
            mem_available: 0,
            mem_free: 0,
            mem_cached: 0,
        };

        // /proc/stat
        if let Ok(stat_content) = std::fs::read_to_string("/proc/stat") {
            if let Some(cpu_line) = stat_content.lines().next() {
                if cpu_line.starts_with("cpu ") {
                    let parts: Vec<&str> = cpu_line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        metrics.cpu_user = parts[1].parse().unwrap_or(0);
                        metrics.cpu_nice = parts[2].parse().unwrap_or(0);
                        metrics.cpu_system = parts[3].parse().unwrap_or(0);
                        metrics.cpu_idle = parts[4].parse().unwrap_or(0);
                        metrics.cpu_iowait = parts[5].parse().unwrap_or(0);
                    }
                }
            }
        }

        // /proc/meminfo
        if let Ok(meminfo_content) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo_content.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        metrics.mem_total = value.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if line.starts_with("MemAvailable:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        metrics.mem_available = value.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if line.starts_with("MemFree:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        metrics.mem_free = value.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if line.starts_with("Cached:") {
                    if let Some(value) = line.split_whitespace().nth(1) {
                        metrics.mem_cached = value.parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
            }
        }

        let cpu_total = metrics.cpu_user
            + metrics.cpu_nice
            + metrics.cpu_system
            + metrics.cpu_idle
            + metrics.cpu_iowait;
        let _cpu_usage = if prev_cpu_total > 0 {
            let total_diff = cpu_total - prev_cpu_total;
            let idle_diff = metrics.cpu_idle - prev_cpu_idle;
            if total_diff > 0 {
                ((total_diff - idle_diff) as f32 / total_diff as f32) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        prev_cpu_total = cpu_total;
        prev_cpu_idle = metrics.cpu_idle;

        fd_info!(
            "    >> [cpumem-tile]: id={} mem_total={:.0}MB, mem_available={:.0}MB",
            (*tile).id,
            metrics.mem_total / (1024 * 1024),
            metrics.mem_available / (1024 * 1024)
        );

        cpumem_metrics(topo, tile, &metrics);
    }
}

#[allow(unused_variables)]
unsafe extern "C" fn disk_tile_run(topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [disk-tile] {} starting", (*tile).id);

    loop {
        sleep(Duration::from_millis(1000));

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut metrics = DiskMetrics {
            timestamp,
            reads_completed: 0,
            reads_merged: 0,
            sectors_read: 0,
            time_reading: 0,
            writes_completed: 0,
            writes_merged: 0,
            sectors_written: 0,
            time_writing: 0,
            io_in_progress: 0,
            time_io: 0,
        };

        // /proc/diskstats
        if let Ok(diskstats_content) = std::fs::read_to_string("/proc/diskstats") {
            for line in diskstats_content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 14 {
                    let device_name = parts[2];
                    if device_name.starts_with("sd") || device_name.starts_with("nvme") {
                        if !device_name.chars().last().unwrap_or('0').is_ascii_digit()
                            || device_name.starts_with("nvme")
                                && device_name.contains("n")
                                && !device_name.contains("p")
                        {
                            metrics.reads_completed += parts[3].parse::<u64>().unwrap_or(0);
                            metrics.reads_merged += parts[4].parse::<u64>().unwrap_or(0);
                            metrics.sectors_read += parts[5].parse::<u64>().unwrap_or(0);
                            metrics.time_reading += parts[6].parse::<u64>().unwrap_or(0);
                            metrics.writes_completed += parts[7].parse::<u64>().unwrap_or(0);
                            metrics.writes_merged += parts[8].parse::<u64>().unwrap_or(0);
                            metrics.sectors_written += parts[9].parse::<u64>().unwrap_or(0);
                            metrics.time_writing += parts[10].parse::<u64>().unwrap_or(0);
                            metrics.io_in_progress += parts[11].parse::<u64>().unwrap_or(0);
                            metrics.time_io += parts[12].parse::<u64>().unwrap_or(0);
                        }
                    }
                }
            }
        }

        fd_info!(
            "    >> [disk-tile]: id={} reads={}, writes={}, sectors_read={}, sectors_written={}",
            (*tile).id,
            metrics.reads_completed,
            metrics.writes_completed,
            metrics.sectors_read,
            metrics.sectors_written
        );

        disk_metrics(topo, tile, &metrics);
    }
}

#[allow(unused_variables)]
unsafe extern "C" fn net_tile_run(topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [net-tile] {} starting", (*tile).id);

    loop {
        sleep(Duration::from_millis(1000));

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut metrics = NetworkMetrics {
            timestamp,
            rx_bytes: 0,
            rx_packets: 0,
            rx_errors: 0,
            rx_dropped: 0,
            tx_bytes: 0,
            tx_packets: 0,
            tx_errors: 0,
            tx_dropped: 0,
        };

        // /proc/net/dev
        if let Ok(netdev_content) = std::fs::read_to_string("/proc/net/dev") {
            for line in netdev_content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 17 {
                    let interface = parts[0].trim_end_matches(':');
                    if !interface.starts_with("lo")
                        && !interface.starts_with("docker")
                        && !interface.starts_with("br-")
                    {
                        metrics.rx_bytes += parts[1].parse::<u64>().unwrap_or(0);
                        metrics.rx_packets += parts[2].parse::<u64>().unwrap_or(0);
                        metrics.rx_errors += parts[3].parse::<u64>().unwrap_or(0);
                        metrics.rx_dropped += parts[4].parse::<u64>().unwrap_or(0);
                        metrics.tx_bytes += parts[9].parse::<u64>().unwrap_or(0);
                        metrics.tx_packets += parts[10].parse::<u64>().unwrap_or(0);
                        metrics.tx_errors += parts[11].parse::<u64>().unwrap_or(0);
                        metrics.tx_dropped += parts[12].parse::<u64>().unwrap_or(0);
                    }
                }
            }
        }

        fd_debug!(
            "    >> [net-tile]: id={} rx={:.0}/s, tx={:.0}/s, rx_pkts={}, tx_pkts={}",
            (*tile).id,
            metrics.rx_bytes / (1024 * 1024),
            metrics.tx_bytes / (1024 * 1024),
            metrics.rx_packets,
            metrics.tx_packets
        );

        net_metrics(topo, tile, &metrics);
    }
}

#[allow(unused_variables)]
unsafe extern "C" fn proc_tile_run(topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [proc-tile] {} starting", (*tile).id);

    let _prev_cpu_metrics: Option<CpuMemMetrics> = None;
    let _prev_disk_metrics: Option<DiskMetrics> = None;
    let _prev_network_metrics: Option<NetworkMetrics> = None;

    loop {
        sleep(Duration::from_millis(2000));

        let aggregated = process_metrics(topo, tile);

        fd_info!("    >> [proc-tile]: id={} cpu={:.1}%, memory={:.1}%, disk_read={:.0}/s, disk_write={:.0}/s, network_rx={:.0}/s, network_tx={:.0}/s, io_wait={:.1}%", (*tile).id, aggregated.cpu_usage_percent, aggregated.memory_usage_percent, aggregated.disk_read_rate, aggregated.disk_write_rate, aggregated.network_rx_rate, aggregated.network_tx_rate, aggregated.io_wait_percent);

        compiled_metrics(topo, tile, &aggregated);
    }
}

#[allow(unused_variables)]
unsafe extern "C" fn writer_tile_run(topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [writer-tile] {} starting", (*tile).id);

    let output_file = "/tmp/sysmon_output.txt";
    let mut file = match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_file)
    {
        Ok(f) => f,
        Err(_) => {
            fd_warn!("    >> [writer-tile]: failed to open output file");
            return;
        }
    };

    fd_debug!("    >> [writer-tile]: dst={}", output_file);

    loop {
        sleep(Duration::from_millis(3000));

        if let Some(aggregated) = recv_metrics(topo, tile) {
            let output_line = format!(
                "{{\"timestamp\":{},\"cpu_usage\":{:.1},\"memory_usage\":{:.1},\"disk_read_rate\":{:.0},\"disk_write_rate\":{:.0},\"network_rx_rate\":{:.0},\"network_tx_rate\":{:.0},\"io_wait\":{:.1}}}\n",
                aggregated.timestamp,
                aggregated.cpu_usage_percent,
                aggregated.memory_usage_percent,
                aggregated.disk_read_rate,
                aggregated.disk_write_rate,
                aggregated.network_rx_rate,
                aggregated.network_tx_rate,
                aggregated.io_wait_percent
            );

            if let Err(_) = file.write_all(output_line.as_bytes()) {
                fd_warn!("    >> [writer-tile]: write error");
            } else if let Err(_) = file.flush() {
                fd_warn!("    >> [writer-tile]: flush error");
            } else {
                fd_info!("    >> [writer-tile]: wrote metrics to file");
            }
        } else {
            fd_debug!("    >> [writer-tile]: no metrics received");
        }
    }
}

#[allow(unused_variables)]
unsafe extern "C" fn metric_tile_run(topo: *mut ActiveTopology, tile: *mut ActiveTile) {
    fd_notice!("> [metric-tile] {} starting", (*tile).id);

    loop {
        sleep(Duration::from_millis(2500));
        fd_info!("    >> [metric-tile]: id={} collecting metrics", (*tile).id);
        // TODO: Receive metrics from all tiles and aggregate/log them
    }
}

fn create_callbacks() -> Result<TopologyCallbacks> {
    let mut registry = TopologyCallbacks::new();

    for obj_name in METRIC_OBJECTS.iter().chain(AUTO_OBJECTS.iter()) {
        registry.add_callback(ObjectCallbacks::new(
            *obj_name,
            calculate_footprint,
            calculate_align,
        ))?;
    }

    Ok(registry)
}

unsafe extern "C" fn calculate_footprint(
    _topo: *const ActiveTopology,
    obj: *const ActiveObject,
) -> u64 {
    let obj_name = std::ffi::CStr::from_ptr((*obj).name.as_ptr());

    match obj_name.to_bytes() {
        b"cpumem_data" => std::mem::size_of::<CpuMemMetrics>() as u64,
        b"disk_data" => std::mem::size_of::<DiskMetrics>() as u64,
        b"net_data" => std::mem::size_of::<NetworkMetrics>() as u64,
        b"proc_data" => std::mem::size_of::<AggregatedMetrics>() as u64,
        b"tile" => 4096,
        b"metrics" => 8192,
        b"keyswitch" => 64,
        b"mcache" => 64 * 1024,
        b"dcache" => 1024 * 1024, // metric payloads
        b"fseq" => 64,
        _ => 4096,
    }
}

unsafe extern "C" fn calculate_align(
    _topo: *const ActiveTopology,
    obj: *const ActiveObject,
) -> u64 {
    let obj_name = std::ffi::CStr::from_ptr((*obj).name.as_ptr());

    match obj_name.to_bytes() {
        b"cpumem_data" => std::mem::align_of::<CpuMemMetrics>() as u64,
        b"disk_data" => std::mem::align_of::<DiskMetrics>() as u64,
        b"net_data" => std::mem::align_of::<NetworkMetrics>() as u64,
        b"proc_data" => std::mem::align_of::<AggregatedMetrics>() as u64,
        b"mcache" | b"dcache" => 64, // cache line
        _ => 64,
    }
}

unsafe fn cpumem_metrics(
    topo: *mut ActiveTopology,
    tile: *mut ActiveTile,
    metrics: &CpuMemMetrics,
) {
    metrics_to_processor(
        topo,
        tile,
        0,
        metrics as *const _ as *const u8,
        std::mem::size_of::<CpuMemMetrics>(),
    );
}

unsafe fn disk_metrics(topo: *mut ActiveTopology, tile: *mut ActiveTile, metrics: &DiskMetrics) {
    metrics_to_processor(
        topo,
        tile,
        0,
        metrics as *const _ as *const u8,
        std::mem::size_of::<DiskMetrics>(),
    );
}

unsafe fn net_metrics(topo: *mut ActiveTopology, tile: *mut ActiveTile, metrics: &NetworkMetrics) {
    metrics_to_processor(
        topo,
        tile,
        0,
        metrics as *const _ as *const u8,
        std::mem::size_of::<NetworkMetrics>(),
    );
}

unsafe fn compiled_metrics(
    topo: *mut ActiveTopology,
    tile: *mut ActiveTile,
    metrics: &AggregatedMetrics,
) {
    metrics_to_processor(
        topo,
        tile,
        0,
        metrics as *const _ as *const u8,
        std::mem::size_of::<AggregatedMetrics>(),
    );
}

unsafe fn metrics_to_processor(
    topo: *mut ActiveTopology,
    tile: *mut ActiveTile,
    link_index: usize,
    data: *const u8,
    size: usize,
) {
    let topo_ref = &*(topo as *const fd_topo_sys::fd_topo_t);
    let tile_wrapper = fd_topo::Tile::from_raw(tile as *mut _);
    let topo_wrapper = fd_topo::Topo::from_raw(topo_ref as *const _ as *mut _, false);
    let data_slice = std::slice::from_raw_parts(data, size);

    match tile_wrapper.send_unchecked(&topo_wrapper, link_index, data_slice) {
        Ok(()) => {
            fd_info!("    >> [proc-tile]: sent={}", size);
        }
        Err(_) => {
            fd_debug!("    >> [proc-tile]: failed to send (dcache full?)");
        }
    }
}

unsafe fn process_metrics(topo: *mut ActiveTopology, tile: *mut ActiveTile) -> AggregatedMetrics {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut aggregated = AggregatedMetrics {
        timestamp,
        cpu_usage_percent: 0.0,
        memory_usage_percent: 0.0,
        disk_read_rate: 0.0,
        disk_write_rate: 0.0,
        network_rx_rate: 0.0,
        network_tx_rate: 0.0,
        io_wait_percent: 0.0,
    };

    let topo_ref = &*(topo as *const fd_topo_sys::fd_topo_t);
    let tile_wrapper = fd_topo::Tile::from_raw(tile as *mut _);
    let topo_wrapper = fd_topo::Topo::from_raw(topo_ref as *const _ as *mut _, false);

    for link_index in 0..tile_wrapper.input_cnt() {
        let mut cpu_metrics = CpuMemMetrics {
            timestamp: 0,
            cpu_user: 0,
            cpu_nice: 0,
            cpu_system: 0,
            cpu_idle: 0,
            cpu_iowait: 0,
            mem_total: 0,
            mem_available: 0,
            mem_free: 0,
            mem_cached: 0,
        };
        if tile_wrapper.recv_into(&topo_wrapper, link_index, &mut cpu_metrics) {
            if let Some(prev) = PREV_CPU_METRICS {
                let total_diff = (cpu_metrics.cpu_user
                    + cpu_metrics.cpu_nice
                    + cpu_metrics.cpu_system
                    + cpu_metrics.cpu_idle
                    + cpu_metrics.cpu_iowait)
                    - (prev.cpu_user
                        + prev.cpu_nice
                        + prev.cpu_system
                        + prev.cpu_idle
                        + prev.cpu_iowait);
                let idle_diff = cpu_metrics.cpu_idle - prev.cpu_idle;

                if total_diff > 0 {
                    aggregated.cpu_usage_percent =
                        ((total_diff - idle_diff) as f32 / total_diff as f32) * 100.0;
                    aggregated.io_wait_percent =
                        ((cpu_metrics.cpu_iowait - prev.cpu_iowait) as f32 / total_diff as f32)
                            * 100.0;
                }
            }

            if cpu_metrics.mem_total > 0 {
                aggregated.memory_usage_percent =
                    ((cpu_metrics.mem_total - cpu_metrics.mem_available) as f32
                        / cpu_metrics.mem_total as f32)
                        * 100.0;
            }

            PREV_CPU_METRICS = Some(cpu_metrics);
            fd_debug!(
                "    >> [cpu-metrics]: id={} cpu={:.1}%, memory={:.1}%",
                (*tile).id,
                aggregated.cpu_usage_percent,
                aggregated.memory_usage_percent
            );
            continue;
        }

        let mut disk_metrics = DiskMetrics {
            timestamp: 0,
            reads_completed: 0,
            reads_merged: 0,
            sectors_read: 0,
            time_reading: 0,
            writes_completed: 0,
            writes_merged: 0,
            sectors_written: 0,
            time_writing: 0,
            io_in_progress: 0,
            time_io: 0,
        };
        if tile_wrapper.recv_into(&topo_wrapper, link_index, &mut disk_metrics) {
            if let Some(prev) = PREV_DISK_METRICS {
                let time_diff = if disk_metrics.timestamp > prev.timestamp {
                    disk_metrics.timestamp - prev.timestamp
                } else {
                    1
                };

                let sectors_read_diff = disk_metrics.sectors_read.saturating_sub(prev.sectors_read);
                let sectors_written_diff = disk_metrics
                    .sectors_written
                    .saturating_sub(prev.sectors_written);

                aggregated.disk_read_rate = (sectors_read_diff * 512) as f32 / time_diff as f32;
                aggregated.disk_write_rate = (sectors_written_diff * 512) as f32 / time_diff as f32;
            }

            PREV_DISK_METRICS = Some(disk_metrics);
            fd_debug!(
                "    >> [disk-metrics]: id={} read={:.0}/s, write={:.0}/s",
                (*tile).id,
                aggregated.disk_read_rate,
                aggregated.disk_write_rate
            );
            continue;
        }

        let mut network_metrics = NetworkMetrics {
            timestamp: 0,
            rx_bytes: 0,
            rx_packets: 0,
            rx_errors: 0,
            rx_dropped: 0,
            tx_bytes: 0,
            tx_packets: 0,
            tx_errors: 0,
            tx_dropped: 0,
        };
        if tile_wrapper.recv_into(&topo_wrapper, link_index, &mut network_metrics) {
            if let Some(prev) = PREV_NETWORK_METRICS {
                let time_diff = if network_metrics.timestamp > prev.timestamp {
                    network_metrics.timestamp - prev.timestamp
                } else {
                    1
                };

                let rx_bytes_diff = network_metrics.rx_bytes.saturating_sub(prev.rx_bytes);
                let tx_bytes_diff = network_metrics.tx_bytes.saturating_sub(prev.tx_bytes);

                aggregated.network_rx_rate = rx_bytes_diff as f32 / time_diff as f32;
                aggregated.network_tx_rate = tx_bytes_diff as f32 / time_diff as f32;
            }

            PREV_NETWORK_METRICS = Some(network_metrics);
            fd_debug!(
                "    >> [net-metrics]: id={} rx={:.0}/s, tx={:.0}/s",
                (*tile).id,
                aggregated.network_rx_rate,
                aggregated.network_tx_rate
            );
        }
    }

    aggregated
}

unsafe fn recv_metrics(
    topo: *mut ActiveTopology,
    tile: *mut ActiveTile,
) -> Option<AggregatedMetrics> {
    let topo_ref = &*(topo as *const fd_topo_sys::fd_topo_t);
    let tile_wrapper = fd_topo::Tile::from_raw(tile as *mut _);
    let topo_wrapper = fd_topo::Topo::from_raw(topo_ref as *const _ as *mut _, false);

    let mut aggregated_metrics = AggregatedMetrics {
        timestamp: 0,
        cpu_usage_percent: 0.0,
        memory_usage_percent: 0.0,
        disk_read_rate: 0.0,
        disk_write_rate: 0.0,
        network_rx_rate: 0.0,
        network_tx_rate: 0.0,
        io_wait_percent: 0.0,
    };
    if tile_wrapper.recv_into(&topo_wrapper, 0, &mut aggregated_metrics) {
        fd_info!("    >> [proc-metrics]: id={} cpu={:.1}%, memory={:.1}%, disk_read={:.0}/s, disk_write={:.0}/s, network_rx={:.0}/s, network_tx={:.0}/s, io_wait={:.1}%", (*tile).id, aggregated_metrics.cpu_usage_percent, aggregated_metrics.memory_usage_percent, aggregated_metrics.disk_read_rate, aggregated_metrics.disk_write_rate, aggregated_metrics.network_rx_rate, aggregated_metrics.network_tx_rate, aggregated_metrics.io_wait_percent);
        return Some(aggregated_metrics);
    }

    None
}
