use std::ffi::CStr;

use fd_topo::{CallbackRegistry, CpuTopology, ObjectCallbacks, Result, TopoBuilder};

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
const AUTO_OBJECTS: [&'static CStr; 5] = [c"tile", c"metrics", c"keyswitch", c"mcache", c"dcache"];

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
        Ok("thin") => CpuTopology::new_simple()?,
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
            CpuTopology::new_custom(cpu_count, numa_count)?
        }
        _ => match CpuTopology::new_simple() {
            Ok(topo) => {
                eprintln!("    >> ✓ cpu-topology found (simple)");
                topo
            }
            Err(_) => {
                let topo = CpuTopology::new_custom(6, 1)?;
                eprintln!("    >> ✓ cpu-topology configured (fallback)");
                topo
            }
        },
    };

    println!(
        "[sys] cpus={}, numa-nodes={}",
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
    println!("> ✓ auto-layout");

    let mut callbacks = create_callbacks()?;
    let callback_ptr = callbacks.finalize()?;

    let mut topo = builder.build(callback_ptr)?;
    println!("> ✓ built topology");

    analyze_topology(&topo)?;
    simulate_execution(&mut topo)?;

    Ok(())
}

fn create_workspaces(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > creating workspaces");

    builder.add_workspace(c"net")?;
    println!("   >> ✓ net_wksp");

    builder.add_workspace(c"pack")?;
    println!("   >> ✓ pack_wksp");

    builder.add_workspace(c"bank")?;
    println!("   >> ✓ bank_wksp");

    builder.add_workspace(c"metric")?;
    println!("   >> ✓ metric_wksp");

    Ok(())
}

fn create_links(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > creating links");

    builder.add_link(c"net_quic", c"net", 1024, 2048, 16)?;
    println!("   >> ✓ link: net_quic (depth: 1024, mtu: 2048)");

    builder.add_link(c"quic_verify", c"pack", 2048, 1500, 32)?;
    println!("   >> ✓ link: quic_verify (depth: 2048, mtu: 1500)");

    builder.add_link(c"verify_pack", c"pack", 1024, 1024, 16)?;
    println!("   >> ✓ link: verify_pack (depth: 1024, mtu: 1024)");

    builder.add_link(c"pack_bank", c"bank", 512, 4096, 8)?;
    println!("   >> ✓ link: pack_bank (depth: 512, mtu: 4096)");

    builder.add_link(c"metric", c"metric", 256, 512, 4)?;
    println!("   >> ✓ link: metric_collect (depth: 256, mtu: 512)");

    Ok(())
}

fn create_tiles(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > creating tiles");

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

    Ok(())
}

fn wire_topology(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > wiring tiles");

    builder.add_tile_output(c"net", 0, c"net_quic", 0)?;
    builder.add_tile_input(c"quic", 0, c"net", c"net_quic", 0, true, true)?;
    builder.add_tile_output(c"quic", 0, c"quic_verify", 0)?;

    for i in 0..2 {
        builder.add_tile_input(c"verify", i, c"pack", c"quic_verify", 0, true, true)?;
        builder.add_tile_output(c"verify", i, c"verify_pack", 0)?;
    }

    builder.add_tile_input(c"pack", 0, c"pack", c"verify_pack", 0, true, true)?;
    builder.add_tile_output(c"pack", 0, c"pack_bank", 0)?;

    for i in 0..2 {
        builder.add_tile_input(c"bank", i, c"bank", c"pack_bank", 0, true, true)?;
    }

    for tile_name in &METRIC_TILE_LINKS {
        builder.add_tile_output(tile_name, 0, c"metric", 0)?;
    }

    builder.add_tile_input(c"metric", 0, c"metric", c"metric", 0, false, true)?;

    println!("   >> ✓ topology wired");
    Ok(())
}

fn analyze_topology(topo: &fd_topo::Topo) -> Result<()> {
    println!("> analyzing structure");

    println!(
        "   >> wksps={} links={} tiles={} objs={}",
        topo.workspace_count(),
        topo.link_count(),
        topo.tile_count(),
        topo.object_count()
    );

    if let Some(net_wksp_id) = topo.find_workspace("net") {
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

    let verify_tile_count = topo.tile_name_count("verify");
    let bank_tile_count = topo.tile_name_count("bank");
    println!("   >> parallelism: verify={verify_tile_count} bank={bank_tile_count}",);

    Ok(())
}

fn simulate_execution(topo: &mut fd_topo::Topo) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        println!("   > joining workspaces: {}", topo.workspace_count());

        match topo.join_workspaces(false) {
            Ok(()) => println!("   >> ✓ workspaces joined"),
            Err(e) => eprintln!("   >> ✗ err={e:?}"),
        }

        println!("   > filling objects");
        topo.fill();

        println!("   > initializing tile contexts");
        for tile_id in 0..topo.tile_count() {
            match topo.join_tile_workspaces(tile_id) {
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

        match topo.run_all_tiles(uid, gid) {
            Ok(()) => println!("   >> ✓ tiles started"),
            Err(e) => eprintln!("   >> ✗ err={e:?}"),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!(
            "   > workspaces={}, tiles={}, links={}",
            topo.workspace_count(),
            topo.tile_count(),
            topo.link_count()
        );
    }

    Ok(())
}

fn create_callbacks() -> Result<CallbackRegistry> {
    let mut registry = CallbackRegistry::new();

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
    _topo: *const fd_topo_sys::fd_topo_t,
    _obj: *const fd_topo_sys::fd_topo_obj_t,
) -> u64 {
    4096
}

unsafe extern "C" fn basic_align(
    _topo: *const fd_topo_sys::fd_topo_t,
    _obj: *const fd_topo_sys::fd_topo_obj_t,
) -> u64 {
    64
}
