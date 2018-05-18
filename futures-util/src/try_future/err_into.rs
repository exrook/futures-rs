use core::marker::PhantomData;
use core::mem::PinMut;

use futures_core::task;
use futures_core::{Future, Poll, TryFuture};

/// Future for the `err_into` combinator, changing the error type of a future.
///
/// This is created by the `Future::err_into` method.
#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct ErrInto<A, E> {
    future: A,
    f: PhantomData<E>,
}

pub fn new<A, E>(future: A) -> ErrInto<A, E> {
    ErrInto { future: future, f: PhantomData }
}

impl<A, E> Future for ErrInto<A, E>
where
    A: TryFuture,
    A::Error: Into<E>,
{
    type Output = Result<A::Item, E>;

    fn poll(mut self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        match unsafe { pinned_field!(self, future) }.try_poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(e) => Poll::Ready(e.map_err(Into::into)),
        }
    }
}
