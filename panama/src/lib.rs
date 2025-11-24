use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
};

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders += 1;
        drop(inner);
        Sender {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders -= 1;
        let was_last = inner.senders == 0;
        drop(inner);
        if was_last {
            self.shared.available.notify_one();
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&mut self, t: T) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.queue.push_back(t);
        drop(inner);
        self.shared.available.notify_one();
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Option<T> {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            if let Some(val) = inner.queue.pop_front() {
                return Some(val);
            } else if inner.senders == 0 {
                return None;
            } else {
                inner = self.shared.available.wait(inner).unwrap();
            }
        }
    }
}

impl<T> Iterator for Receiver<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.recv()
    }
}

struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize
}

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Shared {
        inner: Mutex::new(Inner {
            queue: VecDeque::new(),
            senders: 1
        }),
        available: Condvar::new(),
    };
    let shared = Arc::new(shared);
    (
        Sender {
            shared: shared.clone(),
        },
        Receiver {
            shared: shared.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_pong() {
        let (mut tx, mut rx) = channel();
        tx.send(42);
        assert_eq!(rx.recv(), Some(42));
    }

    #[test]
    fn closed() {
        let (tx, mut rx) = channel::<()>();
        drop(tx);
        assert_eq!(rx.recv(), None);
    }
}