//! Combinators and utilities for working with `Future`s, `Stream`s, `Sink`s,
//! and the `AsyncRead` and `AsyncWrite` traits.

#![feature(pin, arbitrary_self_types)]
#![no_std]
#![deny(missing_docs, missing_debug_implementations, warnings)]
#![doc(html_root_url = "https://docs.rs/futures/0.1")]

#[cfg(test)]
extern crate futures_channel;
#[macro_use]
extern crate futures_core;
#[cfg(test)]
extern crate futures_executor;

extern crate futures_io;
// extern crate futures_sink;

extern crate either;

#[cfg(feature = "std")]
use futures_core::{task, Future, Poll};

macro_rules! if_std {
    ($($i:item)*) => ($(
        #[cfg(feature = "std")]
        $i
    )*)
}

#[cfg(feature = "std")]
//#[macro_use]
extern crate std;

/*
macro_rules! delegate_sink {
    ($field:ident) => {
        fn poll_ready(&mut self, cx: &mut task::Context) -> Poll<(), Self::SinkError> {
            self.$field.poll_ready(cx)
        }

        fn start_send(&mut self, item: Self::SinkItem) -> Result<(), Self::SinkError> {
            self.$field.start_send(item)
        }

        fn poll_flush(&mut self, cx: &mut task::Context) -> Poll<(), Self::SinkError> {
            self.$field.poll_flush(cx)
        }

        fn poll_close(&mut self, cx: &mut task::Context) -> Poll<(), Self::SinkError> {
            self.$field.poll_close(cx)
        }

    }
}
*/

#[cfg(all(feature = "std", any(test, feature = "bench")))]
pub mod lock;
#[cfg(all(feature = "std", not(any(test, feature = "bench"))))]
mod lock;

pub mod future;
pub use future::FutureExt;

pub mod try_future;
pub use try_future::TryFutureExt;

#[cfg(feature = "std")]
pub mod io;
#[cfg(feature = "std")]
pub use io::{AsyncReadExt, AsyncWriteExt};

// pub mod stream;
// pub use stream::StreamExt;

// pub mod sink;
// pub use sink::SinkExt;

pub mod prelude {
    //! Prelude containing the extension traits, which add functionality to
    //! existing asynchronous types.
    // pub use {FutureExt, StreamExt, SinkExt};
    // #[cfg(feature = "std")]
    // pub use {AsyncReadExt, AsyncWriteExt};
}
