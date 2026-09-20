use clap::Parser;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// High-performance UDP load generator (sendmmsg-based).
///
/// Paced mode (--rate / --pps) reproduces iperf-style offered loads with no
/// kernel-side loss, so "Total Packets" can be cross-checked against VPP
/// counters. Unlimited mode (--rate 0) is a saturation flood.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Target IP and port (e.g., 10.10.2.2:5201)
    #[arg(short, long)]
    target: String,

    /// Worker threads
    #[arg(short = 'T', long, default_value_t = 1)]
    threads: usize,

    /// Test duration in seconds
    #[arg(short = 'd', long, default_value_t = 30)]
    duration: u64,

    /// UDP payload size in bytes (iperf-comparable byte accounting)
    #[arg(short = 's', long, default_value_t = 1448)]
    size: usize,

    /// Packets per sendmmsg batch
    #[arg(short = 'b', long, default_value_t = 64)]
    batch_size: usize,

    /// Total rate cap, iperf-style: 50M, 200M, 600M, 1G. "0" = unlimited flood
    #[arg(short = 'r', long, default_value = "0")]
    rate: String,

    /// Exact total rate cap in pps (mutually exclusive with --rate)
    #[arg(long, default_value_t = 0)]
    pps: u64,

    /// Bind all threads to this local source port (SO_REUSEPORT)
    #[arg(short = 'p', long)]
    source_port: Option<u16>,
}

/// Parses iperf-style rate suffixes: "50M" -> 50_000_000, "1G" -> 1e9, "100000" -> as-is.
fn parse_rate(s: &str) -> Result<u64, String> {
    let t = s.trim().to_uppercase();
    let (num, mult) = match t.chars().last() {
        Some('K') => (&t[..t.len() - 1], 1_000u64),
        Some('M') => (&t[..t.len() - 1], 1_000_000),
        Some('G') => (&t[..t.len() - 1], 1_000_000_000),
        _ => (&t[..], 1),
    };
    num.parse::<u64>()
        .map(|n| n * mult)
        .map_err(|_| format!("invalid rate '{s}' (expected e.g. 50M, 600M, 1G, or 0)"))
}

fn die(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2);
}

fn main() {
    let args = Args::parse();
    let target: SocketAddr = args
        .target
        .parse()
        .unwrap_or_else(|e| die(&format!("invalid target '{}': {e}", args.target)));

    let rate_bits = parse_rate(&args.rate).unwrap_or_else(|e| die(&e));
    if args.pps > 0 && rate_bits > 0 {
        die("--rate and --pps are mutually exclusive");
    }
    if args.threads == 0 || args.batch_size == 0 {
        die("--threads and --batch-size must be >= 1");
    }
    if args.size == 0 || args.size > 65507 {
        die("--size must be in 1..=65507");
    }

    // bits/s -> pps conversion happens here, so the caller thinks in iperf units.
    let cap_pps: u64 = if args.pps > 0 {
        args.pps
    } else if rate_bits > 0 {
        ((rate_bits / 8) as f64 / args.size as f64).round() as u64
    } else {
        0
    };
    let per_thread_pps = if cap_pps == 0 {
        0
    } else {
        (cap_pps / args.threads as u64).max(1)
    };
    let batch_interval = if per_thread_pps > 0 {
        let ns = (1_000_000_000u64 * args.batch_size as u64)
            .checked_div(per_thread_pps)
            .unwrap_or(0); // per_thread_pps > 0 guarantees no overflow path here
        Duration::from_nanos(ns)
    } else {
        Duration::ZERO
    };

    println!(
        "Target: {}, threads: {}, payload: {}B, batch: {}, duration: {}s",
        target, args.threads, args.size, args.batch_size, args.duration
    );
    if cap_pps == 0 {
        println!("Rate: unlimited (saturation flood — sent will NOT match VPP counters)");
    } else {
        println!(
            "Rate: {cap_pps} pps total ({per_thread_pps} per thread), batch every {:.3} ms",
            batch_interval.as_secs_f64() * 1000.0
        );
    }

    let is_running = Arc::new(AtomicBool::new(true));

    // Per-thread counters: one AtomicUsize shared by all threads would bounce
    // cache lines between cores at millions of updates/sec (false sharing).
    let mut stats_p = Vec::with_capacity(args.threads);
    let mut stats_b = Vec::with_capacity(args.threads);
    let mut stats_e = Vec::with_capacity(args.threads);
    for _ in 0..args.threads {
        stats_p.push(Arc::new(AtomicUsize::new(0)));
        stats_b.push(Arc::new(AtomicUsize::new(0)));
        stats_e.push(Arc::new(AtomicUsize::new(0)));
    }

    let mut handles = vec![];
    let start_time = Instant::now();

    for i in 0..args.threads {
        let is_running = Arc::clone(&is_running);
        let rx_p = Arc::clone(&stats_p[i]);
        let rx_b = Arc::clone(&stats_b[i]);
        let rx_e = Arc::clone(&stats_e[i]);

        let batch_size = args.batch_size;
        let payload_size = args.size;
        let source_port = args.source_port;
        let interval = batch_interval;

        handles.push(thread::spawn(move || {
            let socket = Socket::new(
                Domain::for_address(target),
                Type::DGRAM,
                Some(Protocol::UDP),
            )
            .expect("Failed to create socket");

            socket
                .set_reuse_port(true)
                .expect("Failed to set SO_REUSEPORT");
            socket
                .set_nonblocking(true)
                .expect("Failed to set non-blocking");

            let local_ip = if target.is_ipv4() {
                IpAddr::V4(Ipv4Addr::UNSPECIFIED)
            } else {
                IpAddr::V6(Ipv6Addr::UNSPECIFIED)
            };
            let local_port = source_port.unwrap_or(0);
            socket
                .bind(&SocketAddr::new(local_ip, local_port).into())
                .expect("Failed to bind socket");
            socket
                .connect(&target.into())
                .expect("Failed to connect socket");

            let fd = socket.as_raw_fd();

            // One shared payload buffer; all iovecs point into it. A flood test
            // only cares about size, not content.
            let mut payload = vec![0u8; payload_size];
            let mut iovecs: Vec<libc::iovec> = (0..batch_size)
                .map(|_| libc::iovec {
                    iov_base: payload.as_mut_ptr() as *mut libc::c_void,
                    iov_len: payload_size,
                })
                .collect();
            // msgs holds raw pointers into iovecs — valid as long as iovecs is
            // never resized, which it never is after this point.
            let mut msgs: Vec<libc::mmsghdr> = iovecs
                .iter_mut()
                .map(|iov| libc::mmsghdr {
                    msg_hdr: libc::msghdr {
                        msg_name: std::ptr::null_mut(),
                        msg_namelen: 0,
                        msg_iov: iov as *mut _,
                        msg_iovlen: 1,
                        msg_control: std::ptr::null_mut(),
                        msg_controllen: 0,
                        msg_flags: 0,
                    },
                    msg_len: 0,
                })
                .collect();

            // Drain ICMP errors / stray datagrams: on a connected UDP socket a
            // port-unreachable surfaces as ECONNREFUSED on the next syscall and
            // must be consumed, or the send loop starts failing spuriously.
            let mut discard = [0u8; 2048];
            let mut next_tick = Instant::now();
            let paced = !interval.is_zero();

            while is_running.load(Ordering::Relaxed) {
                let res = unsafe {
                    libc::sendmmsg(fd, msgs.as_mut_ptr(), batch_size as u32, libc::MSG_DONTWAIT)
                };
                if res > 0 {
                    let n = res as usize;
                    rx_p.fetch_add(n, Ordering::Relaxed);
                    rx_b.fetch_add(n * payload_size, Ordering::Relaxed);
                } else if res < 0 {
                    // Kernel refused (ENOBUFS / ECONNREFUSED / ...): the packets
                    // never left the host. These syscalls are the pre-VPP loss
                    // evidence, so count them separately from sent packets.
                    rx_e.fetch_add(1, Ordering::Relaxed);
                }

                loop {
                    let r = unsafe {
                        libc::recv(
                            fd,
                            discard.as_mut_ptr().cast(),
                            discard.len(),
                            libc::MSG_DONTWAIT,
                        )
                    };
                    if r <= 0 {
                        break;
                    }
                }

                if paced {
                    next_tick += interval;
                    let now = Instant::now();
                    if next_tick > now {
                        thread::sleep(next_tick - now);
                    } else {
                        next_tick = now; // fell behind: don't burst to catch up
                    }
                }
            }
        }));
    }

    // Reporting loop
    let mut last_total_p = 0;
    let mut last_total_b = 0;
    let mut elapsed: u64 = 0;

    println!("{:-<55}", "");
    println!(
        "{:>5} | {:>15} | {:>15} | {:>10}",
        "Time", "PPS", "Mbps (Payload)", "Total Pkts"
    );
    println!("{:-<55}", "");

    while elapsed < args.duration {
        thread::sleep(Duration::from_secs(1));
        elapsed += 1;

        let curr_total_p: usize = stats_p.iter().map(|c| c.load(Ordering::Relaxed)).sum();
        let curr_total_b: usize = stats_b.iter().map(|c| c.load(Ordering::Relaxed)).sum();

        let pps = curr_total_p - last_total_p;
        let mbps = ((curr_total_b - last_total_b) as f64 * 8.0) / 1_000_000.0;

        println!(
            "{:>4}s | {:>15} | {:>15.2} | {:>10}",
            elapsed, pps, mbps, curr_total_p
        );

        last_total_p = curr_total_p;
        last_total_b = curr_total_b;
    }

    is_running.store(false, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }

    let final_p: usize = stats_p.iter().map(|c| c.load(Ordering::Relaxed)).sum();
    let final_b: usize = stats_b.iter().map(|c| c.load(Ordering::Relaxed)).sum();
    let final_e: usize = stats_e.iter().map(|c| c.load(Ordering::Relaxed)).sum();
    let total_time = start_time.elapsed().as_secs_f64();

    println!("{:-<55}", "");
    println!("Final Results:");
    println!("Duration:       {:.2} s", total_time);
    println!("Total Packets:  {}", final_p);
    println!("Failed syscalls: {}", final_e);
    println!("Total Bytes:    {} B", final_b);
    println!("Average PPS:    {:.0}", final_p as f64 / total_time);
    println!(
        "Average Mbps:   {:.2}",
        (final_b as f64 * 8.0) / 1_000_000.0 / total_time
    );
}
