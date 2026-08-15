use std::thread;

#[allow(dead_code)]
pub struct Task<Out: Send + 'static>(Option<std::thread::JoinHandle<Out>>);

impl<Out: Send + 'static> Task<Out> {
    pub fn spawn<T: Fn() -> Out + Send + Sync + 'static>(function: T) -> Task<Out> {
        Self(Some(thread::spawn(move || function())))
    }
}

impl<Out: Send + 'static> Task<Out> {
    pub fn poll(self: &mut Self) -> Poll<Out> {
        let o = self.0.take().unwrap();
        if o.is_finished() {
            Poll::Ready(o.join().unwrap())
        } else {
            self.0 = Some(o);
            Poll::Pending
        }
    }
}

pub enum Poll<T> {
    Ready(T),
    Pending,
}

impl<T> Poll<T> {
    pub fn is_pending(&self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(&Poll::Pending)
    }

    pub fn take_ready(self) -> T {
        match self {
            Poll::Pending => panic!("Didn't check if task was pending before taking its output!"),
            Poll::Ready(a) => return a,
        }
    }
}
