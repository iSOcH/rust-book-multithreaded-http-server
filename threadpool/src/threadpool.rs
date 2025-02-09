use crate::worker::Worker;
use std::{
    sync::{
        mpsc,
        Arc,
        Mutex,
    }
};
use crate::{Job, PoolCreationError};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>
}

impl ThreadPool {
    pub fn build(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size <= 0 {
            return Err(PoolCreationError::InvalidSize);
        }

        let (sender, receiver) = mpsc::channel();
        let shareable_receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for thread_id in 0..size {
            workers.push(Worker::new(thread_id, Arc::clone(&shareable_receiver)));
        }

        Ok(ThreadPool { workers, sender: Some(sender) })
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        // without as_ref sender would be moved which causes compile error "cannot move out of `self.sender` which is behind a shared reference"
        // with note: `Option::<T>::expect` takes ownership of the receiver `self`, which moves `self.sender`
        self.sender.as_ref().expect("Execute called on dropped ThreadPool").send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        if let Some(_) = self.sender.take() {}
        assert!(self.sender.is_none(), "Should have moved out sender");

        while let Some(worker) = self.workers.pop() {
            print!("Waiting for shutdown of worker {}...", worker.id);

            // without this call `worker` would be dropped anyway, but only after the loop iteration.
            // by calling std::mem::drop explicitly here, we can control the point in time when it happens
            drop(worker);

            println!(" done");
        }
    }
}