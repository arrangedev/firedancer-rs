use fd_topo::{CpuTopology, Result, TopoBuilder};

fn main() -> Result<()> {
    let cpu_topo = CpuTopology::new()?;
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

    let mut builder = TopoBuilder::new("initandexec")?;

    create_workspaces(&mut builder)?;
    create_links(&mut builder)?;
    create_tiles(&mut builder)?;
    wire_topology(&mut builder)?;

    builder.auto_layout(false)?;
    println!("> ✓ auto-layout");

    let mut topo = builder.build()?;
    println!("> ✓ built topology");

    analyze_topology(&topo)?;
    simulate_execution(&mut topo)?;

    Ok(())
}

fn create_workspaces(builder: &mut TopoBuilder) -> Result<()> {
    println!("Creating workspaces...");

    builder.add_workspace("net")?;
    println!("   >> ✓ net_wksp");

    builder.add_workspace("pack")?;
    println!("   >> ✓ pack_wksp");

    builder.add_workspace("bank")?;
    println!("   >> ✓ bank_wksp");

    builder.add_workspace("metrics")?;
    println!("   >> ✓ metrics_wksp");

    Ok(())
}

fn create_links(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > creating links");

    builder.add_link("net_quic", "net", 1024, 2048, 16)?;
    println!("   >> ✓ link: net_quic (depth: 1024, mtu: 2048)");

    builder.add_link("quic_verify", "pack", 2048, 1500, 32)?;
    println!("   >> ✓ link: quic_verify (depth: 2048, mtu: 1500)");

    builder.add_link("verify_pack", "pack", 1024, 1024, 16)?;
    println!("   >> ✓ link: verify_pack (depth: 1024, mtu: 1024)");

    builder.add_link("pack_bank", "bank", 512, 4096, 8)?;
    println!("   >> ✓ link: pack_bank (depth: 512, mtu: 4096)");

    builder.add_link("metrics_collect", "metrics", 256, 512, 4)?;
    println!("   >> ✓ link: metrics_collect (depth: 256, mtu: 512)");

    Ok(())
}

fn create_tiles(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > creating tiles");

    builder.add_tile("net", "net", "metrics", Some(0), false, false)?;
    builder.add_object("net_rx_buffer", "net")?;
    builder.add_object("net_tx_buffer", "net")?;
    println!("   >> ✓ net (cpuid=0)");

    builder.add_tile("quic", "net", "metrics", Some(1), false, false)?;
    builder.add_object("quic_conn_pool", "net")?;
    builder.add_object("quic_stream_pool", "net")?;
    println!("   >> ✓ quic (cpuid=1)");

    for i in 0..2 {
        builder.add_tile("verify", "pack", "metrics", Some(2 + i), false, false)?;
        builder.add_object(&format!("verify_ctx_{}", i), "pack")?;
        println!("   >> ✓ verify {} (cpuid={})", i, 2 + i);
    }

    builder.add_tile("pack", "pack", "metrics", Some(4), false, false)?;
    builder.add_object("pack_pending_txns", "pack")?;
    builder.add_object("pack_microblocks", "pack")?;
    println!("   >> ✓ pack (cpuid=4)");

    for i in 0..2 {
        builder.add_tile("bank", "bank", "metrics", Some(5 + i), false, false)?;
        builder.add_object(&format!("bank_accounts_{}", i), "bank")?;
        builder.add_object(&format!("bank_programs_{}", i), "bank")?;
        println!("   >> ✓ bank {} (cpuid={})", i, 5 + i);
    }

    builder.add_tile("metrics", "metrics", "metrics", Some(7), false, false)?;
    builder.add_object("metrics_data", "metrics")?;
    println!("   >> ✓ metrics (cpuid=7)");

    Ok(())
}

fn wire_topology(builder: &mut TopoBuilder) -> Result<()> {
    println!("   > wiring tiles");

    builder.add_tile_output("net", 0, "net_quic", 0)?;
    builder.add_tile_input("quic", 0, "net", "net_quic", 0, true, true)?;
    builder.add_tile_output("quic", 0, "quic_verify", 0)?;

    for i in 0..2 {
        builder.add_tile_input("verify", i, "pack", "quic_verify", 0, true, true)?;
        builder.add_tile_output("verify", i, "verify_pack", 0)?;
    }

    builder.add_tile_input("pack", 0, "pack", "verify_pack", 0, true, true)?;
    builder.add_tile_output("pack", 0, "pack_bank", 0)?;

    for i in 0..2 {
        builder.add_tile_input("bank", i, "bank", "pack_bank", 0, true, true)?;
    }

    let tile_names = ["net", "quic", "verify", "pack", "bank"];
    for tile_name in &tile_names {
        builder.add_tile_output(tile_name, 0, "metrics_collect", 0)?;
    }

    builder.add_tile_input("metrics", 0, "metrics", "metrics_collect", 0, false, true)?;

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
