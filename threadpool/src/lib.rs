use std::{
    thread,
    sync::{
        mpsc,
        Arc,
        Mutex,
    }
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>
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

        Ok(ThreadPool { workers, sender })
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let maybe_next_job = receiver.lock().unwrap().recv();

                match maybe_next_job {
                    Ok(j) => {
                        println!("Worker {id} got a job; executing...");
                        j();
                    },
                    Err(_) => break,
                }
            }

            println!("Worker thread {id} exiting");
        });

        Worker { id, thread }
    }
}

#[derive(Debug)]
pub enum PoolCreationError {
    InvalidSize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
