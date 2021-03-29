#![feature(test)]

#[cfg(feature = "bilock")]
mod bench {
use futures::Stream;
use futures::pin_mut;
use futures::task::{Context, Waker, Poll};
use futures::executor::LocalPool;
use futures::executor::block_on;
use futures_test::task::noop_context;
use futures_util::lock::BiLock;
use futures_util::lock::BiLockAcquire;
use futures_util::lock::BiLockGuard;
use futures_util::task::ArcWake;

use std::sync::Arc;
use std::pin::Pin;
use std::mem::drop;
extern crate test;
use test::Bencher;

//fn notify_noop() -> Waker {
//    struct Noop;
//
//    impl ArcWake for Noop {
//        fn wake(_: &Arc<Self>) {}
//    }
//
//    ArcWake::into_waker(Arc::new(Noop))
//}


///// Pseudo-stream which simply calls `lock.poll()` on `poll`
//struct LockStream {
//    lock: BiLock<u32>,
//}
//
//impl LockStream {
//    fn new(lock: BiLock<u32>) -> Self {
//        Self {
//            lock
//        }
//    }
//}
//
//impl Stream for LockStream {
//    type Item = BiLockGuard<u32>;
//
//    fn poll_next(self: Pin<&mut LockStream>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//        self.lock.poll_lock(cx).map(Some)
//    }
//}


#[bench]
fn contended(b: &mut Bencher) {
    //let pool = LocalPool::new();
    //let mut exec = pool.executor();
    //let waker = notify_noop();
    //let mut map = task::LocalMap::new();
    //let mut waker = task::Context::new(&mut map, &waker, &mut exec);
    let mut ctx = noop_context();

    b.iter(|| {
        let (x, y) = BiLock::new(1);

        //let mut x = LockStream::new(x);
        //let mut y = LockStream::new(y);

        for _ in 0..1000 {
            let x_guard = match x.poll_lock(&mut ctx) {
                Poll::Ready(guard) => guard,
                _ => panic!(),
            };

            // Try poll second lock while first lock still holds the lock
            match y.poll_lock(&mut ctx) {
                Poll::Pending => (),
                _ => panic!(),
            };

            drop(x_guard);

            let y_guard = match y.poll_lock(&mut ctx) {
                Poll::Ready(guard) => guard,
                _ => panic!(),
            };

            drop(y_guard);
        }
        (x, y)
    });
}

#[bench]
fn lock_unlock(b: &mut Bencher) {
    //let pool = LocalPool::new();
    //let mut exec = pool.executor();
    //let waker = notify_noop();
    //let mut map = task::LocalMap::new();
    //let mut waker = task::Context::new(&mut map, &waker, &mut exec);
    let mut ctx = noop_context();

    b.iter(|| {
        let (x, y) = BiLock::new(1);

        //let mut x = LockStream::new(x);
        //let mut y = LockStream::new(y);

        for _ in 0..1000 {
            let x_guard = match x.poll_lock(&mut ctx) {
                Poll::Ready(guard) => guard,
                _ => panic!(),
            };

            drop(x_guard);

            let y_guard = match y.poll_lock(&mut ctx) {
                Poll::Ready(guard) => guard,
                _ => panic!(),
            };

            drop(y_guard);
        }
        (x, y)
    })
}
#[bench]
fn concurrent(b: &mut Bencher) {
    use std::thread;

    b.iter(|| {
        let (mut x, mut y) = BiLock::new(false);
        const ITERATION_COUNT: usize = 1000;

        let a = thread::spawn(move || {
            let mut count = 0;
            while count < ITERATION_COUNT {
                let mut guard = block_on(x.lock());
                if *guard {
                    *guard = false;
                    count += 1;
                }
                drop(guard);
            }
        });

        let b = thread::spawn(move || {
            let mut count = 0;
            while count < ITERATION_COUNT {
                let mut guard = block_on(y.lock());
                if !*guard {
                    *guard = true;
                    count += 1;
                }
                drop(guard);
            }
        });

        a.join().unwrap();
        b.join().unwrap();
    })
}
}
