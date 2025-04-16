use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// A thread-safe, bounded, blocking FIFO queue implemented with a monitor pattern.
///
/// This queue supports multiple producers and multiple consumers. Operations block
/// when the queue is full (on `enqueue`) or empty (on `dequeue`) until progress is possible.
///
/// # Shutdown behavior
///
/// After `shutdown()` is called:
/// - No new items will be enqueued.
/// - Any blocked or future `dequeue` calls will either return an item (if any are left)
///   or `None` if the queue is empty and shut down.
/// - Any blocked `enqueue` calls will exit silently without enqueuing.
///
/// Internally uses a `Mutex` and two `Condvar`s to synchronize access.
///
/// # Example
///
/// ```
/// use std::sync::Arc;
/// use std::thread;
/// use std::time::Duration;
/// use fifo_bounded_buffer::Queue;
///
/// let queue = Queue::new(2);
/// let producer = {
///     let q = Arc::clone(&queue);
///     thread::spawn(move || {
///         q.enqueue(1);
///         q.enqueue(2);
///     })
/// };
///
/// let consumer = {
///     let q = Arc::clone(&queue);
///     thread::spawn(move || {
///         assert_eq!(q.dequeue(), Some(1));
///         assert_eq!(q.dequeue(), Some(2));
///     })
/// };
///
/// producer.join().unwrap();
/// consumer.join().unwrap();
/// ```
#[derive(Debug)]
pub struct Queue<T> {
    inner: Mutex<Inner<T>>,
    not_empty: Condvar,
    not_full: Condvar,
    capacity: usize,
}

/// Inner shared state of the queue, protected by the mutex.
///
/// - `buffer`: the actual queue storage
/// - `shutdown`: a flag that signals termination to all threads
#[derive(Debug)]
struct Inner<T> {
    buffer: VecDeque<T>,
    shutdown: bool,
}

impl<T> Queue<T> {
    /// Creates a new `Queue` with a fixed capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements the queue can hold.
    ///
    /// # Returns
    ///
    /// A reference-counted pointer (`Arc`) to the new `Queue` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue = Queue::<usize>::new(10);
    /// ```
    ///
    /// ```
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue: std::sync::Arc<Queue<i32>> = Queue::new(5);
    /// ```
    pub fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Inner {
                buffer: VecDeque::with_capacity(capacity),
                shutdown: false,
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
            capacity,
        })
    }

    /// Adds an item to the queue, blocking if the queue is full.
    ///
    /// If the queue is shut down, the item will be silently dropped
    /// and enqueue will return early.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to add to the queue.
    ///
    /// # Blocking
    ///
    /// - Blocks if the queue is full until space becomes available or shutdown occurs.
    ///
    /// # Panics
    ///
    /// Panics if the thread is poisoned while waiting on the condition variable or mutex.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue = Queue::new(2);
    /// queue.enqueue(10);
    /// ```
    pub fn enqueue(&self, item: T) {
        let mut inner = self.inner.lock().unwrap();
        while inner.buffer.len() == self.capacity && !inner.shutdown {
            inner = self.not_full.wait(inner).unwrap();
        }

        if inner.shutdown {
            return;
        }

        inner.buffer.push_back(item);
        self.not_empty.notify_one();
    }

    /// Removes and returns an item from the front of the queue.
    ///
    /// # Returns
    ///
    /// * `Some(item)` - if an item was dequeued.
    /// * `None` - if the queue is shut down and empty.
    ///
    /// # Blocking
    ///
    /// - Blocks if the queue is empty until an item is added or shutdown occurs.
    ///
    /// # Panics
    ///
    /// Panics if the thread is poisoned while waiting on the condition variable or mutex.
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue = Queue::new(1);
    /// queue.enqueue(42);
    /// assert_eq!(queue.dequeue(), Some(42));
    /// ```
    pub fn dequeue(&self) -> Option<T> {
        let mut inner = self.inner.lock().unwrap();
        while inner.buffer.is_empty() && !inner.shutdown {
            inner = self.not_empty.wait(inner).unwrap();
        }

        let item = inner.buffer.pop_front();
        if item.is_some() {
            self.not_full.notify_one();
        }
        item
    }

    /// Shuts down the queue, waking all blocked threads and preventing further enqueues.
    ///
    /// After shutdown:
    ///
    /// - No new items will be accepted via `enqueue`.
    /// - Consumers will continue to dequeue remaining items, but then receive `None`.
    /// - All condition variables are notified so blocked threads can exit.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue = Queue::new(1);
    /// queue.enqueue(1);
    /// queue.shutdown();
    ///
    /// assert!(queue.is_shutdown());
    /// ```
    pub fn shutdown(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.shutdown = true;
        self.not_empty.notify_all();
        self.not_full.notify_all();
    }

    /// Checks if the queue is currently empty.
    ///
    /// # Returns
    ///
    /// `true` if the queue has no items; `false` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue = Queue::new(2);
    /// assert!(queue.is_empty());
    ///
    /// queue.enqueue(5);
    /// assert!(!queue.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.buffer.is_empty()
    }

    /// Checks if the queue has been shut down.
    ///
    /// # Returns
    ///
    /// `true` if shutdown has been initiated; `false` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::Arc;
    /// use fifo_bounded_buffer::Queue;
    ///
    /// let queue = Queue::<usize>::new(3);
    /// assert!(!queue.is_shutdown());
    ///
    /// queue.shutdown();
    /// assert!(queue.is_shutdown());
    /// ```
    pub fn is_shutdown(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.shutdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_create_destroy() {
        let _queue: Arc<Queue<usize>> = Queue::new(10);
        // Simply creating and letting it go out of scope to "destroy" it.
        // In Rust, destruction is handled automatically via Drop.
    }

    #[test]
    fn test_queue_dequeue() {
        let queue = Arc::new(Queue::new(10));
        let data = Box::new(1usize);
        let ptr = data.as_ref() as *const usize;

        queue.enqueue(data);
        if let Some(result) = queue.dequeue() {
            let result_ptr = result.as_ref() as *const usize;
            assert_eq!(ptr, result_ptr);
        } else {
            panic!("Expected to dequeue an item, but got None");
        }
    }

    #[test]
    fn test_queue_dequeue_multiple() {
        let queue = Arc::new(Queue::new(10));

        let d1 = Box::new(1usize);
        let d2 = Box::new(2usize);
        let d3 = Box::new(3usize);

        let p1 = d1.as_ref() as *const usize;
        let p2 = d2.as_ref() as *const usize;
        let p3 = d3.as_ref() as *const usize;

        queue.enqueue(d1);
        queue.enqueue(d2);
        queue.enqueue(d3);

        let r1 = queue.dequeue().expect("Expected item 1");
        let r2 = queue.dequeue().expect("Expected item 2");
        let r3 = queue.dequeue().expect("Expected item 3");

        assert_eq!(p1, r1.as_ref() as *const usize);
        assert_eq!(p2, r2.as_ref() as *const usize);
        assert_eq!(p3, r3.as_ref() as *const usize);
    }

    #[test]
    fn test_queue_dequeue_shutdown() {
        let queue = Arc::new(Queue::new(10));

        let d1 = Box::new(1usize);
        let d2 = Box::new(2usize);
        let d3 = Box::new(3usize);

        let p1 = d1.as_ref() as *const usize;
        let p2 = d2.as_ref() as *const usize;
        let p3 = d3.as_ref() as *const usize;

        queue.enqueue(d1);
        queue.enqueue(d2);
        queue.enqueue(d3);

        let r1 = queue.dequeue().expect("Expected item 1");
        let r2 = queue.dequeue().expect("Expected item 2");

        assert_eq!(p1, r1.as_ref() as *const usize);
        assert_eq!(p2, r2.as_ref() as *const usize);

        queue.shutdown();

        let r3 = queue.dequeue().expect("Expected item 3");
        assert_eq!(p3, r3.as_ref() as *const usize);

        assert!(queue.is_shutdown());
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_enqueue_blocks_when_full() {
        let queue = Arc::new(Queue::new(1));
        queue.enqueue(1);

        let q_clone = Arc::clone(&queue);
        let handle = std::thread::spawn(move || {
            // This should block until the item is dequeued
            q_clone.enqueue(2);
        });

        std::thread::sleep(std::time::Duration::from_millis(100));
        assert_eq!(queue.dequeue(), Some(1));
        handle.join().unwrap();

        assert_eq!(queue.dequeue(), Some(2));
    }

    #[test]
    fn test_queue_dequeue_blocks_when_empty_then_returns() {
        let queue = Arc::new(Queue::new(1));

        let q_clone = Arc::clone(&queue);
        let handle = std::thread::spawn(move || {
            // This should block until an item is enqueued
            q_clone.dequeue()
        });

        std::thread::sleep(std::time::Duration::from_millis(100));
        queue.enqueue(42);
        let result = handle.join().unwrap();

        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_shutdown_unblocks_dequeue_and_returns_none() {
        let queue = Arc::new(Queue::<usize>::new(1));

        let q_clone = Arc::clone(&queue);
        let handle = std::thread::spawn(move || {
            // This should block until shutdown, then return None
            q_clone.dequeue()
        });

        std::thread::sleep(std::time::Duration::from_millis(100));
        queue.shutdown();

        let result = handle.join().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_enqueue_after_shutdown_does_nothing() {
        let queue = Arc::new(Queue::new(2));
        queue.enqueue(10);
        queue.shutdown();

        // This should silently do nothing
        queue.enqueue(20);

        let result = queue.dequeue();
        assert_eq!(result, Some(10));

        // There should be no second item
        assert!(queue.dequeue().is_none());
    }

    #[test]
    fn test_shutdown_multiple_times_does_not_panic() {
        let queue = Queue::<usize>::new(1);
        queue.shutdown();
        queue.shutdown(); // Should not panic or deadlock
        assert!(queue.is_shutdown());
    }

    #[test]
    fn test_is_empty_considers_state_correctly() {
        let queue = Queue::new(3);
        assert!(queue.is_empty());

        queue.enqueue(99);
        assert!(!queue.is_empty());

        let _ = queue.dequeue();
        assert!(queue.is_empty());
    }
}
