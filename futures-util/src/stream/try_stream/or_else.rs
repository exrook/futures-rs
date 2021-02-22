use crate::fns::FnMut1;
use core::fmt;
use core::pin::Pin;
use futures_core::future::TryFuture;
use futures_core::ready;
use futures_core::stream::{FusedStream, Stream, TryStream};
use futures_core::task::{Context, Poll};
#[cfg(feature = "sink")]
use futures_sink::Sink;
use pin_project_lite::pin_project;

pin_project! {
    /// Stream for the [`or_else`](super::TryStreamExt::or_else) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct OrElse<St, Fut, F> {
        #[pin]
        stream: St,
        #[pin]
        future: Option<Fut>,
        f: F,
    }
}

impl<St, Fut, F> fmt::Debug for OrElse<St, Fut, F>
where
    St: fmt::Debug,
    Fut: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrElse")
            .field("stream", &self.stream)
            .field("future", &self.future)
            .finish()
    }
}

impl<St, Fut, F> OrElse<St, Fut, F>
where
    St: TryStream,
    F: FnMut1<St::Error, Output = Fut>,
    Fut: TryFuture<Ok = St::Ok>,
{
    pub(super) fn new(stream: St, f: F) -> Self {
        Self { stream, future: None, f }
    }

    delegate_access_inner!(stream, St, ());
}

impl<St, Fut, F> Stream for OrElse<St, Fut, F>
where
    St: TryStream,
    F: FnMut1<St::Error, Output = Fut>,
    Fut: TryFuture<Ok = St::Ok>,
{
    type Item = Result<St::Ok, Fut::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.try_poll(cx));
                this.future.set(None);
                break Some(item);
            } else {
                match ready!(this.stream.as_mut().try_poll_next(cx)) {
                    Some(Ok(item)) => break Some(Ok(item)),
                    Some(Err(e)) => {
                        this.future.set(Some(this.f.call_mut(e)));
                    }
                    None => break None,
                }
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let future_len = if self.future.is_some() { 1 } else { 0 };
        let (lower, upper) = self.stream.size_hint();
        let lower = lower.saturating_add(future_len);
        let upper = match upper {
            Some(x) => x.checked_add(future_len),
            None => None,
        };
        (lower, upper)
    }
}

impl<St, Fut, F> FusedStream for OrElse<St, Fut, F>
where
    St: TryStream + FusedStream,
    F: FnMut1<St::Error, Output = Fut>,
    Fut: TryFuture<Ok = St::Ok>,
{
    fn is_terminated(&self) -> bool {
        self.future.is_none() && self.stream.is_terminated()
    }
}

// Forwarding impl of Sink from the underlying stream
#[cfg(feature = "sink")]
impl<S, Fut, F, Item> Sink<Item> for OrElse<S, Fut, F>
where
    S: Sink<Item>,
{
    type Error = S::Error;

    delegate_sink!(stream, Item);
}
