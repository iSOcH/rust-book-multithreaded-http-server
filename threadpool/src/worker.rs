use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use crate::Job;

pub struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
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