use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: mpsc::Sender<Message>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
  NewJob(Job),
  Terminate,
}

impl ThreadPool {
  /// Create a new ThreadPool.
  ///
  /// The size is the number of threads in the pool.
  ///
  /// # Panics
  ///
  /// The `new` function will panic if the size is zero.
  pub fn new(size: usize) -> ThreadPool {
    assert!(size > 0);

    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));
    let mut workers = Vec::with_capacity(size);

    for id in 0..size {
      let worker = Worker::new(id, Arc::clone(&receiver));
      workers.push(worker);
    }

    ThreadPool { workers, sender }
  }

  /// Send a closure into it's own thread, inside the ThreadPool to be executed
  pub fn execute<F>(&self, f: F)
  where
    F: FnOnce() + Send + 'static,
  {
    let job = Box::new(f);

    self
      .sender
      .send(Message::NewJob(job))
      .expect("Did not execute job.");
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    println!("Send 'Terminate' message to all workers");

    for worker in self.workers.iter() {
      self.sender.send(Message::Terminate).expect(&format!(
        "Did not send 'Terminate' Message to worker {}",
        worker.id
      ))
    }

    println!("Shutting down all workers.");

    for worker in &mut self.workers {
      println!("Shuting down worker {}", worker.id);

      if let Some(thread) = worker.thread.take() {
        thread
          .join()
          .expect("Couldn't join on the associated thread");
      }
    }
  }
}

struct Worker {
  id: usize,
  thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
  /// Create a new worker with a unique `id` and a message `receiver`
  pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
    let thread = std::thread::spawn(move || loop {
      let handle = receiver.lock().expect("Could not obtain `receiver` handle");
      let message = handle.recv().expect("Error on `recv`");

      match message {
        Message::NewJob(job) => {
          println!("Worker {} got a job; executing", id);

          job();
        }
        Message::Terminate => {
          println!("Worker {} was told to terminate", id);

          break;
        }
      }
    });

    Worker {
      id,
      thread: Some(thread),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_thread_pool() {
    let pool = ThreadPool::new(2);

    assert_eq!(pool.workers.len(), 2);
  }

  #[test]
  #[should_panic]
  fn thread_pool_panic_with_zero_pools_arg() {
    ThreadPool::new(0);
  }

  #[test]
  fn execute_runs() {
    let pool = ThreadPool::new(2);

    pool.execute(|| {
      assert!(true);
    })
  }

  #[test]
  fn create_new_worker() {
    let (_, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));
    let worker = Worker::new(1, receiver);

    assert_eq!(worker.id, 1);
    assert!(worker.thread.is_some());
  }
}
