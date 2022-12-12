use std::thread;
use std::time::{self, Duration, Instant};

use clap::{Parser, ValueEnum};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use spmc::{Receiver, TryRecvError};

mod bindings;
mod rng;

use rng::{DevUrandom, Rng};

use crate::rng::{OsRng, ThreadRng};

#[derive(Serialize, Deserialize)]
struct Stats {
    num_entries: usize,
    mean: u64,
    stddev: u64,
    min: u64,
    max: u64,
    p50: u64,
    p90: u64,
    p99: u64,
    p999: u64,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum RngType {
    /// Use `rand`-crate's `ThreadRng`
    ThreadRng,
    /// Use `rand`-crate's `OsRng`
    OsRng,
    /// Use directly `/dev/urandom`
    DevUrandom,
}

#[derive(Parser, Debug)]
struct Args {
    /// Number of random bytes per request
    #[arg(short = 'b', long = "bytes", default_value_t = 1024)]
    rand_bytes_num: usize,
    /// Time in msec to wait between requests
    #[arg(short = 'm', long = "msec-per-request", default_value_t = 1000)]
    msec_per_request: u64,
    /// Which RNG implementation to use
    #[arg(long = "rng-type", value_enum, default_value_t = RngType::DevUrandom)]
    rng_type: RngType,
    /// Number of iterations to run before exiting
    #[arg(short = 'i', long = "iterations", default_value_t = 10)]
    num_requests: u32,
    /// Number of threads to spawn
    #[arg(short, long, default_value_t = 1)]
    threads: u32,
    /// Stats file to write json stats
    #[arg(short, long, default_value_t = String::from("results.txt"))]
    stats_file: String,
}

fn set_ctrlc_handler(num_threads: u32) -> Receiver<()> {
    let (mut tx, rx) = spmc::channel();

    ctrlc::set_handler(move || {
        debug!("");
        debug!("Ctrl-C received. Stopping.");
        for _ in 0..num_threads {
            tx.send(())
                .expect("Could not send Ctrl-C signal on channel")
        }
    })
    .expect("Error setting Ctrl-C signal handler");

    rx
}

fn check_for_ctrlc(rx: &Receiver<()>) -> bool {
    match rx.try_recv() {
        Ok(()) => true,
        Err(TryRecvError::Empty) => false,
        Err(err) => panic!("Ctrl-C handler channel broke down: {err}"),
    }
}

fn start_loop(
    rng: &mut dyn Rng,
    num_bytes: usize,
    msec_per_request: u64,
    num_iterations: u32,
    shutdown_channel: Receiver<()>,
) -> Vec<Duration> {
    let mut buffer = vec![0u8; num_bytes];
    let mut entropy_count = rng.get_entropy_count().unwrap();
    let mut stats: Vec<Duration> = Vec::new();

    debug!("Starting experiment");
    debug!("Will make {num_iterations} requests of {num_bytes} every {msec_per_request} msec");
    debug!("Initial entropy count: {entropy_count}");

    for _ in 0..num_iterations {
        let now = Instant::now();
        rng.get_random(buffer.as_mut_slice()).unwrap();
        stats.push(now.elapsed());

        thread::sleep(time::Duration::from_millis(msec_per_request));

        let new_entropy_count = rng.get_entropy_count().unwrap();
        if new_entropy_count != entropy_count {
            warn!("Entropy count changed. New value: {new_entropy_count}");
            entropy_count = new_entropy_count;
        }

        if check_for_ctrlc(&shutdown_channel) {
            break;
        }
    }

    stats
}

fn main() {
    pretty_env_logger::init();
    let args = Args::parse();

    let mut handles = Vec::new();
    let rx = set_ctrlc_handler(args.threads);

    for t in 0..args.threads {
        let rx = rx.clone();
        handles.push(std::thread::spawn(move || {
            debug!("Spawning thread {t}");

            let mut rng: Box<dyn Rng> = match args.rng_type {
                RngType::ThreadRng => Box::new(ThreadRng::new()),
                RngType::OsRng => Box::new(OsRng::new()),
                RngType::DevUrandom => Box::new(DevUrandom::new().unwrap()),
            };

            start_loop(
                rng.as_mut(),
                args.rand_bytes_num,
                args.msec_per_request,
                args.num_requests,
                rx,
            )
        }))
    }

    let mut hist = histogram::Histogram::new();
    for handle in handles {
        let data = handle.join().expect("Could not join thread");
        data.iter()
            .for_each(|val| hist.increment(val.as_nanos() as u64).unwrap());
    }

    let stats = Stats {
        num_entries: hist.entries() as usize,
        mean: hist.mean().unwrap(),
        stddev: hist.stddev().unwrap(),
        min: hist.minimum().unwrap(),
        max: hist.maximum().unwrap(),
        p50: hist.percentile(50.0).unwrap(),
        p90: hist.percentile(90.0).unwrap(),
        p99: hist.percentile(99.0).unwrap(),
        p999: hist.percentile(99.9).unwrap(),
    };

    info!("Number of requests executed: {}", hist.entries());
    info!(
        "Total bytes requested: {}",
        stats.num_entries * args.rand_bytes_num
    );
    info!("Average time per request: {} nsec", stats.mean);
    info!("Standard deviation of request time: {} nsec", stats.stddev);
    info!("Maximum time for request: {} nsec", stats.max);
    info!("Minimum time for request: {} nsec", stats.min);
    info!(
        "Request time percentiles: p50: {} p90: {} p99: {}, p999: {}",
        stats.p50, stats.p90, stats.p99, stats.p999
    );

    /*
    std::fs::write(
        args.stats_file,
        serde_json::to_string_pretty(&stats).unwrap(),
    )
    .unwrap();
    */

    std::fs::write(
        args.stats_file,
        format!(
            "{} {} {} {} {} {} {} {} {}",
            stats.num_entries,
            stats.mean,
            stats.stddev,
            stats.max,
            stats.min,
            stats.p50,
            stats.p90,
            stats.p99,
            stats.p999
        ),
    )
    .unwrap();
}
