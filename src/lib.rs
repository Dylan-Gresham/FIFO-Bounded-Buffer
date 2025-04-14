use std::{
    collections::VecDeque,
    ffi::c_void,
    os::raw::c_int,
    ptr,
    sync::{Condvar, Mutex},
};

#[repr(C)]
pub struct Queue {
    inner: Mutex<Inner>,
    not_empty: Condvar,
    not_full: Condvar,
    capacity: usize,
    shutdown: Mutex<bool>,
}

struct Inner {
    queue: VecDeque<*mut c_void>,
}

#[allow(non_camel_case_types)]
pub type queue_t = *mut Queue;

#[unsafe(no_mangle)]
pub extern "C" fn queue_init(capacity: c_int) -> queue_t {
    if capacity <= 0 {
        return ptr::null_mut();
    }

    let q = Box::new(Queue {
        inner: Mutex::new(Inner {
            queue: VecDeque::with_capacity(capacity as usize),
        }),
        not_empty: Condvar::new(),
        not_full: Condvar::new(),
        capacity: capacity as usize,
        shutdown: Mutex::new(false),
    });

    Box::into_raw(q)
}

#[unsafe(no_mangle)]
pub extern "C" fn queue_destroy(q: queue_t) {
    if q.is_null() {
        return;
    }

    let q = unsafe { Box::from_raw(q) };

    let mut shutdown_flag = q.shutdown.lock().unwrap();
    *shutdown_flag = true;

    q.not_empty.notify_all();
    q.not_full.notify_all();
}

#[unsafe(no_mangle)]
pub extern "C" fn enqueue(q: queue_t, _data: *mut c_void) {
    if q.is_null() {
        return;
    }

    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn dequeue(q: queue_t) -> *mut c_void {
    if q.is_null() {
        return ptr::null_mut();
    }

    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn queue_shutdown(q: queue_t) {
    if q.is_null() {
        return;
    }

    todo!()
}

#[unsafe(no_mangle)]
pub extern "C" fn is_empty(q: queue_t) -> bool {
    if q.is_null() {
        return true;
    }

    let q = unsafe { &*q };
    let inner = q.inner.lock().unwrap();

    inner.queue.is_empty()
}

#[unsafe(no_mangle)]
pub extern "C" fn is_shutdown(q: queue_t) -> bool {
    if q.is_null() {
        return true;
    }

    let q = unsafe { &*q };
    let shutdown = q.shutdown.lock().unwrap();

    *shutdown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_destroy() {
        let q: queue_t = queue_init(10);
        assert!(!q.is_null());
        queue_destroy(q);
    }

    #[test]
    fn test_queue_dequeue() {
        let q: queue_t = queue_init(10);
        assert!(!q.is_null());

        let data: isize = 1;
        enqueue(q, data as *mut c_void);
        assert_eq!(dequeue(q) as isize, data);
        queue_destroy(q);
    }

    #[test]
    fn test_queue_dequeue_multiple() {
        let q: queue_t = queue_init(10);
        assert!(!q.is_null());

        let data1: isize = 1;
        let data2: isize = 1;
        let data3: isize = 1;

        enqueue(q, data1 as *mut c_void);
        enqueue(q, data2 as *mut c_void);
        enqueue(q, data3 as *mut c_void);

        assert_eq!(dequeue(q) as isize, data1);
        assert_eq!(dequeue(q) as isize, data2);
        assert_eq!(dequeue(q) as isize, data3);

        queue_destroy(q);
    }

    #[test]
    fn test_queue_dequeue_shutdown() {
        let q: queue_t = queue_init(10);
        assert!(!q.is_null());

        let data1: isize = 1;
        let data2: isize = 1;
        let data3: isize = 1;

        enqueue(q, data1 as *mut c_void);
        enqueue(q, data2 as *mut c_void);
        enqueue(q, data3 as *mut c_void);

        assert_eq!(dequeue(q) as isize, data1);
        assert_eq!(dequeue(q) as isize, data2);

        queue_shutdown(q);

        assert_eq!(dequeue(q) as isize, data3);
        assert!(is_shutdown(q));
        assert!(is_empty(q));

        queue_destroy(q);
    }
}
