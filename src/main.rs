mod queue;

use clap::Parser;
use queue::Queue;
use rand::Rng;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

/// Command line arguments using clap
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Number of consumer threads
    #[arg(short = 'c', default_value = "1")]
    consumers: usize,

    /// Number of producer threads
    #[arg(short = 'p', default_value = "1")]
    producers: usize,

    /// Total items to produce per thread
    #[arg(short = 'i', default_value = "10")]
    items: usize,

    /// Size of the queue
    #[arg(short = 's', default_value = "5")]
    size: usize,

    /// Introduce delay between enqueue/dequeue
    #[arg(short = 'd', default_value_t = false)]
    delay: bool,
}

fn main() {
    let args = Args::parse();

    let nump = args.producers.min(8);
    let numc = args.consumers.min(8);
    let per_thread = args.items / nump;

    println!(
        "{} SAMPLE OUTPUT FROM MAIN {}",
        "-".repeat(10),
        "-".repeat(10)
    );

    println!(
        "Simulating {} producers {} consumers with {} items per thread and a queue size of {}",
        nump, numc, per_thread, args.size
    );

    let queue = Arc::new(Queue::new(args.size));
    let start = Instant::now();

    let produced = Arc::new(Mutex::new(0usize));
    let consumed = Arc::new(Mutex::new(0usize));

    // Spawn producers
    let producers: Vec<_> = (0..nump)
        .map(|_| {
            let q = Arc::clone(&queue);
            let prod_count = Arc::clone(&produced);
            thread::spawn(move || {
                let mut rng = rand::rng();
                for i in 0..per_thread {
                    if args.delay {
                        let delay = rng.random_range(0..1_000_000);
                        thread::sleep(Duration::from_nanos(delay));
                    }

                    q.enqueue(Box::new(i));
                    *prod_count.lock().unwrap() += 1;
                }
            })
        })
        .collect();

    // Spawn consumers
    let consumers: Vec<_> = (0..numc)
        .map(|_| {
            let q = Arc::clone(&queue);
            let cons_count = Arc::clone(&consumed);
            thread::spawn(move || {
                let mut rng = rand::rng();
                loop {
                    if args.delay {
                        let delay = rng.random_range(0..1_000_000);
                        thread::sleep(Duration::from_nanos(delay));
                    }

                    if let Some(item) = q.dequeue() {
                        drop(item); // free the boxed int
                        *cons_count.lock().unwrap() += 1;
                    } else if q.is_shutdown() {
                        break;
                    }
                }
            })
        })
        .collect();

    // Wait for all producers
    for p in producers {
        p.join().unwrap();
    }

    // Shutdown the queue to unblock consumers
    queue.shutdown();

    // Wait for all consumers
    for c in consumers {
        c.join().unwrap();
    }

    let total_produced = *produced.lock().unwrap();
    let total_consumed = *consumed.lock().unwrap();

    if total_produced != total_consumed {
        eprintln!("ERROR! produced != consumed");
        std::process::abort();
    }

    println!("Queue is empty: {}", queue.is_empty());
    println!("Total produced: {}", total_produced);
    println!("Total consumed: {}", total_consumed);

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    println!("Took {}s with {} produced.", elapsed, total_produced);
}
