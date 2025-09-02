use fd_env::cmdline;

fn main() {
    let args = vec![
        "my_program".to_string(),
        "--count".to_string(),
        "42".to_string(),
        "--name".to_string(),
        "firedancer".to_string(),
        "--verbose".to_string(),
        "--threshold".to_string(),
        "3.14".to_string(),
        "--other".to_string(),
        "remaining".to_string(),
    ];

    println!("original-args={:?}\n", args);

    let mut cmdline = cmdline::CommandLine::new(args);

    let count = cmdline.strip_ulong("--count", None, 0);
    println!("   parsed={} (default)", count);
    println!("   remaining={:?}\n", cmdline.to_vec());

    let name = cmdline.strip_cstr("--name", None, "default");
    println!("   parsed={} (default)", name);
    println!("   remaining={:?}\n", cmdline.to_vec());

    let is_verbose = cmdline.contains_and_strip("--verbose");
    println!("   verbose={}", is_verbose);
    println!("   remaining={:?}\n", cmdline.to_vec());

    let threshold = cmdline.strip_float("--threshold", None, 1.0);
    println!("   parsed={} (default)", threshold);
    println!("   remaining={:?}\n", cmdline.to_vec());

    let missing = cmdline.strip_int("--missing", None, -1);
    println!("   parsed={} (default)", missing);
    println!("   remaining={:?}\n", cmdline.to_vec());

    let mut cmdline2 = cmdline::CommandLine::new(vec![
        "program".to_string(),
        "--port".to_string(),
        "8080".to_string(),
    ]);

    let port = cmdline2.strip_uint("--port", Some("MYAPP_PORT"), 3000);
    println!("   port (env/flag, default 3000): {}", port);
    println!("   remaining={:?}\n", cmdline2.to_vec());

    let mut type_demo = cmdline::CommandLine::new(vec![
        "program".to_string(),
        "--uint".to_string(),
        "4294967295".to_string(), // u32::MAX
        "--int".to_string(),
        "-2147483648".to_string(), // i32::MIN
        "--long".to_string(),
        "9223372036854775807".to_string(), // i64::MAX
        "--float".to_string(),
        "2.718281828".to_string(), // e
    ]);

    let uint_val = type_demo.strip_uint("--uint", None, 0);
    let int_val = type_demo.strip_int("--int", None, 0);
    let long_val = type_demo.strip_long("--long", None, 0);
    let float_val = type_demo.strip_float("--float", None, 0.0);

    println!("   uint={} (u32)", uint_val);
    println!("   int={} (i32)", int_val);
    println!("   long={} (i64)", long_val);
    println!("   float={} (f32)", float_val);
}
