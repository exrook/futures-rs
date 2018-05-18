use std::io;
use std::marker::Unpin;
use std::mem::{self, PinMut};

use {task, Future, Poll};

use futures_io::AsyncWrite;

/// A future used to write the entire contents of some data to a stream.
///
/// This is created by the [`write_all`] top-level method.
///
/// [`write_all`]: fn.write_all.html
#[derive(Debug)]
pub struct WriteAll<'a, A: ?Sized + 'a> {
    a: &'a mut A,
    buf: &'a [u8],
}

// Pinning is never projected to fields
unsafe impl<'a, A: ?Sized> Unpin for WriteAll<'a, A> {}

pub fn write_all<'a, A>(a: &'a mut A, buf: &'a [u8]) -> WriteAll<'a, A>
where
    A: AsyncWrite + ?Sized,
{
    WriteAll { a, buf }
}

fn zero_write() -> io::Error {
    io::Error::new(io::ErrorKind::WriteZero, "zero-length write")
}

impl<'a, A> Future for WriteAll<'a, A>
where
    A: AsyncWrite + ?Sized,
{
    type Output = io::Result<()>;

    fn poll(mut self: PinMut<Self>, cx: &mut task::Context) -> Poll<io::Result<()>> {
        let this = &mut *self;
        while this.buf.len() > 0 {
            let n = match this.a.poll_write(cx, this.buf) {
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            {
                let (rest, _) = mem::replace(&mut this.buf, &[]).split_at(n);
                this.buf = rest;
            }
            if n == 0 {
                return Poll::Ready(Err(zero_write()));
            }
        }

        Poll::Ready(Ok(()))
    }
}
