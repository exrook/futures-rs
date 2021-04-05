#[cfg(feature = "bilock")]
mod bench {
    use futures::executor::block_on;
    use futures::task::Poll;
    use futures_test::task::noop_context;
    use futures_util::lock::BiLock;

    use criterion::{criterion_group, Criterion};
    use std::mem::drop;

    fn contended(c: &mut Criterion) {
        let mut ctx = noop_context();
        c.bench_function("contended", |b| {
            b.iter(|| {
                let (mut x, mut y) = BiLock::new(1);

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
            });
        });
    }

    fn lock_unlock(c: &mut Criterion) {
        let mut ctx = noop_context();

        c.bench_function("lock_unlock", |b| {
            b.iter(|| {
                let (mut x, mut y) = BiLock::new(1);

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
            })
        });
    }

    fn concurrent(c: &mut Criterion) {
        use std::thread;

        c.bench_function("concurrent", |b| {
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
                        x = guard.unlock();
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
                        y = guard.unlock();
                    }
                });

                a.join().unwrap();
                b.join().unwrap();
            })
        });
    }

    criterion_group!(benches, contended, lock_unlock, concurrent);
}
#[cfg(feature = "bilock")]
criterion::criterion_main!(bench::benches);
#[cfg(not(feature = "bilock"))]
fn main() {}
