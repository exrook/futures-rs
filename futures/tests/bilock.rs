#![cfg(feature = "bilock")]
use futures::channel::mpsc;
use futures::executor::{block_on, ThreadPool};
use futures::future::{self, ready, FutureExt, Future};
use futures::lock::BiLock;
use futures::stream::{self, StreamExt};
use futures::task::{self, Poll, Context, SpawnExt};
use futures_test::future::FutureTestExt;
use futures_test::task::{new_count_waker, panic_context};
use std::thread;
use std::sync::Arc;
use std::pin::Pin;

#[test]
fn smoke() {
    let future = future::lazy(|ctx| {
        let (mut a, mut b) = BiLock::new(1);

        {
            let mut lock = match a.poll_lock(ctx) {
                Poll::Ready(l) => l,
                Poll::Pending => panic!("poll not ready"),
            };
            assert_eq!(*lock, 1);
            *lock = 2;

            assert!(b.poll_lock(ctx).is_pending());
            //assert!(a.poll_lock(ctx).is_pending());
        }

        assert!(b.poll_lock(ctx).is_ready());
        assert!(a.poll_lock(ctx).is_ready());

        {
            let lock = match b.poll_lock(ctx) {
                Poll::Ready(l) => l,
                Poll::Pending => panic!("poll not ready"),
            };
            assert_eq!(*lock, 2);
        }

        assert_eq!(a.reunite(b).expect("bilock/smoke: reunite error"), 2);

        Ok::<(), ()>(())
    });

    block_on(future).expect("failure in poll")
}

#[test]
fn concurrent() {
    const N: usize = 10000;
    let (mut a, mut b) = BiLock::new(0);

    let a = Increment {
        a: Some(a),
        remaining: N,
    };
    let b = stream::iter(0..N).fold(b, |b, _n| {
        b.lock().map(|mut b| {
            *b += 1;
            b.unlock()
        })
    });

    let mut ctx = panic_context();

    let t1 = thread::spawn(move || block_on(a));
    let mut b = block_on(b);
    let mut a = t1.join().expect("a error");

    match a.poll_lock(&mut ctx) {
        Poll::Ready(l) => assert_eq!(*l, 2 * N),
        Poll::Pending => panic!("poll not ready"),
    }
    match b.poll_lock(&mut ctx) {
        Poll::Ready(l) => assert_eq!(*l, 2 * N),
        Poll::Pending => panic!("poll not ready"),
    }

    assert_eq!(a.reunite(b).expect("bilock/concurrent: reunite error"), 2 * N);

    struct Increment {
        remaining: usize,
        a: Option<BiLock<usize>>,
    }

    impl Future for Increment {
        type Output = BiLock<usize>;

        fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_> ) -> Poll<BiLock<usize>> {
            loop {
                if self.remaining == 0 {
                    return Poll::Ready(self.a.take().unwrap().into())
                }

                {
                    let a = self.a.as_mut().unwrap();
                    let mut a = match a.poll_lock(ctx) {
                        Poll::Ready(l) => l,
                        Poll::Pending => return Poll::Pending,
                    };
                    *a += 1;
                }
                self.remaining -= 1;
            }
        }
    }
}
