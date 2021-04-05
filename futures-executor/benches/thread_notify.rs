use criterion::{criterion_group, criterion_main, Criterion};

use futures::executor::block_on;
use futures::future::Future;
use futures::task::{Context, Poll, Waker};
use std::pin::Pin;

fn thread_yield_single_thread_one_wait(c: &mut Criterion) {
    c.bench_function("thread_yield_single_thread_one_wait", |b| {
        const NUM: usize = 10_000;

        struct Yield {
            rem: usize,
        }

        impl Future for Yield {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.rem == 0 {
                    Poll::Ready(())
                } else {
                    self.rem -= 1;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        b.iter(|| {
            let y = Yield { rem: NUM };
            block_on(y);
        });
    });
}

fn thread_yield_single_thread_many_wait(c: &mut Criterion) {
    c.bench_function("thread_yield_single_thread_many_wait", |b| {
        const NUM: usize = 10_000;

        struct Yield {
            rem: usize,
        }

        impl Future for Yield {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.rem == 0 {
                    Poll::Ready(())
                } else {
                    self.rem -= 1;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        b.iter(|| {
            for _ in 0..NUM {
                let y = Yield { rem: 1 };
                block_on(y);
            }
        });
    });
}

fn thread_yield_multi_thread(c: &mut Criterion) {
    c.bench_function("thread_yield_multi_thread", |b| {
        use std::sync::mpsc;
        use std::thread;

        const NUM: usize = 1_000;

        let (tx, rx) = mpsc::sync_channel::<Waker>(10_000);

        struct Yield {
            rem: usize,
            tx: mpsc::SyncSender<Waker>,
        }
        impl Unpin for Yield {}

        impl Future for Yield {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.rem == 0 {
                    Poll::Ready(())
                } else {
                    self.rem -= 1;
                    self.tx.send(cx.waker().clone()).unwrap();
                    Poll::Pending
                }
            }
        }

        thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                task.wake();
            }
        });

        b.iter(move || {
            let y = Yield { rem: NUM, tx: tx.clone() };

            block_on(y);
        });
    });
}

criterion_group!(
    bench,
    thread_yield_single_thread_one_wait,
    thread_yield_single_thread_many_wait,
    thread_yield_multi_thread
);
criterion_main!(bench);
