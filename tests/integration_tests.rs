use fifo_bounded_buffer::Queue;
use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const MAX_SLEEP_NS: u64 = 1_000_000;

fn spawn_producers(
    queue: Arc<Queue<Box<usize>>>,
    num_producers: usize,
    items_per_thread: usize,
    delay: bool,
) -> Vec<thread::JoinHandle<usize>> {
    (0..num_producers)
        .map(|_| {
            let q = Arc::clone(&queue);
            thread::spawn(move || {
                let mut produced = 0;
                for i in 0..items_per_thread {
                    if delay {
                        let sleep = rand::rng().random_range(0..MAX_SLEEP_NS);
                        thread::sleep(Duration::from_nanos(sleep));
                    }
                    q.enqueue(Box::new(i));
                    produced += 1;
                }
                produced
            })
        })
        .collect()
}

fn spawn_consumers(
    queue: Arc<Queue<Box<usize>>>,
    num_consumers: usize,
    delay: bool,
) -> Vec<thread::JoinHandle<usize>> {
    (0..num_consumers)
        .map(|_| {
            let q = Arc::clone(&queue);
            thread::spawn(move || {
                let mut consumed = 0;
                loop {
                    if delay {
                        let sleep = rand::rng().random_range(0..MAX_SLEEP_NS);
                        thread::sleep(Duration::from_nanos(sleep));
                    }
                    match q.dequeue() {
                        Some(_) => consumed += 1,
                        None => {
                            if q.is_shutdown() {
                                break;
                            }
                        }
                    }
                }
                consumed
            })
        })
        .collect()
}

fn run_test(
    num_producers: usize,
    num_consumers: usize,
    items: usize,
    queue_size: usize,
    delay: bool,
) {
    let queue = Arc::new(Queue::new(queue_size));
    let start = Instant::now();

    let producers = spawn_producers(Arc::clone(&queue), num_producers, items, delay);
    let consumers = spawn_consumers(Arc::clone(&queue), num_consumers, delay);

    let total_produced: usize = producers
        .into_iter()
        .map(|h| h.join().expect("Producer thread panicked"))
        .sum();

    queue.shutdown();

    let total_consumed: usize = consumers
        .into_iter()
        .map(|h| h.join().expect("Consumer thread panicked"))
        .sum();

    let elapsed = start.elapsed();

    assert_eq!(
        total_produced, total_consumed,
        "Mismatch: {} != {}",
        total_produced, total_consumed
    );
    assert!(queue.is_empty());
    println!(
        "Test completed: {} producers, {} consumers, {} items/thread, queue size {}, delay {} — Time: {:.2?}",
        num_producers, num_consumers, items, queue_size, delay, elapsed
    );
}

#[test]
fn integration_scenarios() {
    let configs = vec![
        (2, 2, 10, 5, false),
        (4, 4, 20, 10, true),
        (8, 2, 15, 8, false),
        (2, 8, 15, 8, true),
        (3, 3, 50, 20, true),
        (1, 1, 10, 5, false),
        (2, 2, 50, 10, false),
        (4, 2, 100, 20, false),
        (2, 4, 100, 10, false),
        (1, 8, 80, 5, false),
        (8, 1, 10, 2, false),
        (4, 4, 1000, 50, true),
        (8, 8, 1000, 100, true),
        (16, 4, 500, 10, true),
        (4, 16, 500, 10, true),
        (1, 1, 1000, 1, true),
        (3, 5, 200, 3, true),
        (5, 3, 200, 3, true),
        (6, 6, 600, 6, true),
        (10, 2, 150, 2, true),
        (2, 10, 150, 2, true),
    ];

    for (p, c, items, size, delay) in configs {
        run_test(p, c, items, size, delay);
    }
}
