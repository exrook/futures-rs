use criterion::{criterion_group, criterion_main, Criterion};

use futures::channel::oneshot;
use futures::executor::block_on;
use futures::future;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::task::Poll;
use std::collections::VecDeque;
use std::thread;

fn oneshots(c: &mut Criterion) {
    c.bench_function("oneshots", |b| {
        const NUM: usize = 10_000;

        b.iter(|| {
            let mut txs = VecDeque::with_capacity(NUM);
            let mut rxs = FuturesUnordered::new();

            for _ in 0..NUM {
                let (tx, rx) = oneshot::channel();
                txs.push_back(tx);
                rxs.push(rx);
            }

            thread::spawn(move || {
                while let Some(tx) = txs.pop_front() {
                    let _ = tx.send("hello");
                }
            });

            block_on(future::poll_fn(move |cx| {
                loop {
                    if let Poll::Ready(None) = rxs.poll_next_unpin(cx) {
                        break;
                    }
                }
                Poll::Ready(())
            }))
        });
    });
}
criterion_group!(bench, oneshots);
criterion_main!(bench);
