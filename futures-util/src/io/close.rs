use std::io;
use std::marker::Unpin;
use std::mem::PinMut;

use {task, Future, Poll};

use futures_io::AsyncWrite;

/// A future used to fully close an I/O object.
///
/// Created by the [`close`] function.
///
/// [`close`]: fn.close.html
#[derive(Debug)]
pub struct Close<'a, A: ?Sized + 'a> {
    a: &'a mut A,
}

// PinMut is never projected to fields
unsafe impl<'a, A: ?Sized> Unpin for Close<'a, A> {}

pub fn close<'a, A: ?Sized>(a: &'a mut A) -> Close<'a, A>
where
    A: AsyncWrite,
{
    Close { a }
}

impl<'a, A> Future for Close<'a, A>
where
    A: AsyncWrite + ?Sized,
{
    type Output = io::Result<()>;

    fn poll(mut self: PinMut<Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        self.a.poll_close(cx)
    }
}
