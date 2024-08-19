use std::fmt::Debug;
use std::sync::{Condvar, Mutex};

// Arc documentation for threading with mutex and condvar here:
// Mutex: https://doc.rust-lang.org/stable/std/sync/struct.Mutex.html
// Condvar: https://doc.rust-lang.org/stable/std/sync/struct.Condvar.html
/// A ring (circular) buffer struct that can only be used in a multi-threaded environment
pub struct MultiThreadedRingBuffer<T, const CAPACITY: usize> {
    num_jobs: (Mutex<usize>, Condvar),
    inner_rb: Mutex<InnerRingBuffer<T, CAPACITY>>, // state: Arc<(Mutex<State>, Condvar)>
}

// An inner ring buffer to contain the items, enqueue, and dequeue index for MultiThreadedRingBuffer struct
struct InnerRingBuffer<T, const CAPACITY: usize> {
    items: [Option<T>; CAPACITY],
    enqueue_index: usize,
    dequeue_index: usize,
}

/// Implements the InnerRingBuffer functions
impl<T: Debug, const CAPACITY: usize> InnerRingBuffer<T, CAPACITY> {
    /// Instantiates the InnerRingBuffer
    const fn new() -> Self {
        InnerRingBuffer {
            // How to initialize a generic array of options with None (without needing to iterate hence making it O(1) init)
            // https://stackoverflow.com/questions/28656387/initialize-a-large-fixed-size-array-with-non-copy-types
            items: [const { None }; CAPACITY],
            enqueue_index: 0,
            dequeue_index: 0,
        }
    }
}

/// Implements the MultiThreadedRingBuffer functions
impl<T: Debug, const CAPACITY: usize> MultiThreadedRingBuffer<T, CAPACITY> {
    /// Instantiates the MultiThreadedRingBuffer. 
    /// 
    /// Time Complexity: O(1), Space complexity: O(N) 
    pub const fn new() -> Self {
        MultiThreadedRingBuffer {
            num_jobs: (Mutex::new(0), Condvar::new()),
            inner_rb: Mutex::new(InnerRingBuffer::new()),
        }
    }

    /// Helper function to add an Option item to the MultiThreadedRingBuffer
    /// This is necessary so that the ring buffer can be poisoned with None values
    /// 
    /// Time Complexity: O(1) if not blocked (arbitrary time if it is), 
    /// Space complexity: O(1)
    async fn enqueue_item(&self, item: Option<T>) {
        // Locks to read how many jobs are in the ring buffer
        let (num_jobs, cvar) = &self.num_jobs;
        let mut num_jobs = num_jobs.lock().unwrap();

        // If ring buffer is at capacity, block until an item is dequeued off the ring buffer
        while *num_jobs == CAPACITY {
            num_jobs = cvar.wait(num_jobs).unwrap();
        }

        // Locks to read the current enqueue index in the ring buffer and write it to the 
        // items of the ring buffer at that specific enqueue index
        let mut inner = self.inner_rb.lock().unwrap();
        let enqueue_index = inner.enqueue_index;
        inner.items[enqueue_index] = item;
        *num_jobs += 1;

        // This enables the enqueue index to remain within the bounds of the
        // array 
        inner.enqueue_index = (inner.enqueue_index + 1) % CAPACITY;

        // Notifies a CondVar to inform that there is a job available
        cvar.notify_one();
    }

    /// Adds an item of type T to the MultiThreadedRingBuffer so long as there is space in the buffer
    /// 
    /// Time Complexity: O(1) if not blocked (arbitrary time if it is), 
    /// Space complexity: O(1)
    pub async fn enqueue(&self, item: T) {
        self.enqueue_item(Some(item)).await;
    }

    /// Retrieves an item of type T from the MultiThreadedRingBuffer if an item exists in the buffer
    ///
    /// Time Complexity: O(1) if not blocked (arbitrary time if it is), 
    /// Space complexity: O(1)
    pub async fn dequeue(&self) -> Option<T> {
        // Locks to read how many jobs are in the ring buffer
        let (num_jobs, cvar) = &self.num_jobs;
        let mut num_jobs = num_jobs.lock().unwrap();

        // If ring buffer is empty, block until an item is enqueued on the ring buffer
        while *num_jobs == 0 {
            num_jobs = cvar.wait(num_jobs).unwrap();
        }

        // Locks to read the current dequeue index in the ring buffer and takes the 
        // item of the ring buffer at that specific enqueue index (replaces it with None
        // in exchange)
        let mut inner = self.inner_rb.lock().unwrap();
        let dequeue_index = inner.dequeue_index;
        let item = inner.items[dequeue_index].take();
        *num_jobs -= 1;

        // This enables the dequeue index to remain within the bounds of the
        // array         
        inner.dequeue_index = (inner.dequeue_index + 1) % CAPACITY;

        // Notifies a CondVar to inform that a job can be enqueued
        cvar.notify_one();

        // Returns dequeued item
        item
    }

    /// Poisons the MultiThreadedRingBuffer with None values up to the capacity of the buffer
    /// 
    /// Time Complexity: O(N) if not blocked (arbitrary time if it is), 
    /// Space complexity: O(1)
    pub async fn poison(&self) {
        for _ in 0..CAPACITY {
            self.enqueue_item(None).await;
        }
    }

    /// If the MultiThreadedRingBuffer is poisoned via the poison()
    /// call or is at capacity, this method will allow the ring buffer
    /// to be used again and resets it to an empty state
    /// 
    /// Time Complexity: O(1), Space complexity: O(1)
    pub async fn clear_poison(&self) {
        let mut num_jobs = self.num_jobs.0.lock().unwrap();
        if *num_jobs == CAPACITY {
            *self.inner_rb.lock().unwrap() = InnerRingBuffer::new();
            *num_jobs = 0;
        } else {
            println!("Ring buffer is not poisoned or it is empty");
        }
    }

    /// Clears the MultiThreadedRingBuffer back to an empty state
    /// 
    /// Time Complexity: O(1), Space complexity: O(1)
    pub async fn clear(&self) {
        let mut num_jobs = self.num_jobs.0.lock().unwrap();
        *num_jobs = 0;
        *self.inner_rb.lock().unwrap() = InnerRingBuffer::new();
    }
}

impl<T: Debug, const CAPACITY: usize> Default for MultiThreadedRingBuffer<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
     }
}