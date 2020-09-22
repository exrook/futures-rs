//! Abstractions for asynchronous programming.
//!
//! This crate provides a number of core abstractions for writing asynchronous
//! code:
//!
//! - [Futures](crate::future::Future) are single eventual values produced by
//!   asynchronous computations. Some programming languages (e.g. JavaScript)
//!   call this concept "promise".
//! - [Streams](crate::stream::Stream) represent a series of values
//!   produced asynchronously.
//! - [Sinks](crate::sink::Sink) provide support for asynchronous writing of
//!   data.
//! - [Executors](crate::executor) are responsible for running asynchronous
//!   tasks.
//!
//! The crate also contains abstractions for [asynchronous I/O](crate::io) and
//! [cross-task communication](crate::channel).
//!
//! Underlying all of this is the *task system*, which is a form of lightweight
//! threading. Large asynchronous computations are built up using futures,
//! streams and sinks, and then spawned as independent tasks that are run to
//! completion, but *do not block* the thread running them.
//!
//! The following example describes how the task system context is built and used
//! within macros and keywords such as async and await!.
//!
//! ```rust
//! # use futures::channel::mpsc;
//! # use futures::executor; ///standard executors to provide a context for futures and streams
//! # use futures::executor::ThreadPool;
//! # use futures::StreamExt;
//!
//! fn main() {
//!     let pool = ThreadPool::new().expect("Failed to build pool");
//!     let (tx, rx) = mpsc::unbounded::<i32>();
//!
//!     // Create a future by an async block, where async is responsible for an
//!     // implementation of Future. At this point no executor has been provided
//!     // to this future, so it will not be running.
//!     let fut_values = async {
//!         // Create another async block, again where the Future implementation
//!         // is generated by async. Since this is inside of a parent async block,
//!         // it will be provided with the executor of the parent block when the parent
//!         // block is executed.
//!         //
//!         // This executor chaining is done by Future::poll whose second argument
//!         // is a std::task::Context. This represents our executor, and the Future
//!         // implemented by this async block can be polled using the parent async
//!         // block's executor.
//!         let fut_tx_result = async move {
//!             (0..100).for_each(|v| {
//!                 tx.unbounded_send(v).expect("Failed to send");
//!             })
//!         };
//!
//!         // Use the provided thread pool to spawn the generated future
//!         // responsible for transmission
//!         pool.spawn_ok(fut_tx_result);
//!
//!         let fut_values = rx
//!             .map(|v| v * 2)
//!             .collect();
//!
//!         // Use the executor provided to this async block to wait for the
//!         // future to complete.
//!         fut_values.await
//!     };
//!
//!     // Actually execute the above future, which will invoke Future::poll and
//!     // subsequenty chain appropriate Future::poll and methods needing executors
//!     // to drive all futures. Eventually fut_values will be driven to completion.
//!     let values: Vec<i32> = executor::block_on(fut_values);
//!
//!     println!("Values={:?}", values);
//! }
//! ```
//!
//! The majority of examples and code snippets in this crate assume that they are
//! inside an async block as written above.

#![cfg_attr(feature = "cfg-target-has-atomic", feature(cfg_target_has_atomic))]
#![cfg_attr(feature = "read-initializer", feature(read_initializer))]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms, unreachable_pub)]
// It cannot be included in the published code because this lints have false positives in the minimum required version.
#![cfg_attr(test, warn(single_use_lifetimes))]
#![warn(clippy::all)]
// mem::take requires Rust 1.40, matches! requires Rust 1.42
// Can be removed if the minimum supported version increased or if https://github.com/rust-lang/rust-clippy/issues/3941
// get's implemented.
#![allow(clippy::mem_replace_with_default, clippy::match_like_matches_macro)]
#![doc(test(attr(deny(warnings), allow(dead_code, unused_assignments, unused_variables))))]
#![doc(html_root_url = "https://docs.rs/futures/0.3.5")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(feature = "cfg-target-has-atomic", not(feature = "unstable")))]
compile_error!("The `cfg-target-has-atomic` feature requires the `unstable` feature as an explicit opt-in to unstable features");

#[cfg(all(feature = "bilock", not(feature = "unstable")))]
compile_error!("The `bilock` feature requires the `unstable` feature as an explicit opt-in to unstable features");

#[cfg(all(feature = "read-initializer", not(feature = "unstable")))]
compile_error!("The `read-initializer` feature requires the `unstable` feature as an explicit opt-in to unstable features");

#[doc(hidden)]
pub use futures_core::future::{Future, TryFuture};
#[doc(hidden)]
pub use futures_util::future::{FutureExt, TryFutureExt};

#[doc(hidden)]
pub use futures_core::stream::{Stream, TryStream};
#[doc(hidden)]
pub use futures_util::stream::{StreamExt, TryStreamExt};

#[doc(hidden)]
pub use futures_sink::Sink;
#[doc(hidden)]
pub use futures_util::sink::SinkExt;

#[cfg(feature = "std")]
#[doc(hidden)]
pub use futures_io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite};
#[cfg(feature = "std")]
#[doc(hidden)]
pub use futures_util::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

// Macro reexports
pub use futures_core::ready; // Readiness propagation
pub use futures_util::pin_mut;
#[cfg(feature = "std")]
#[cfg(feature = "async-await")]
pub use futures_util::select;
#[cfg(feature = "async-await")]
pub use futures_util::{join, pending, poll, select_biased, try_join}; // Async-await

#[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
#[cfg(feature = "alloc")]
pub mod channel {
    //! Cross-task communication.
    //!
    //! Like threads, concurrent tasks sometimes need to communicate with each
    //! other. This module contains two basic abstractions for doing so:
    //!
    //! - [oneshot](crate::channel::oneshot), a way of sending a single value
    //!   from one task to another.
    //! - [mpsc](crate::channel::mpsc), a multi-producer, single-consumer
    //!   channel for sending values between tasks, analogous to the
    //!   similarly-named structure in the standard library.
    //!
    //! This module is only available when the `std` or `alloc` feature of this
    //! library is activated, and it is activated by default.

    pub use futures_channel::oneshot;

    #[cfg(feature = "std")]
    pub use futures_channel::mpsc;
}

#[cfg(feature = "compat")]
#[cfg_attr(docsrs, doc(cfg(feature = "compat")))]
pub mod compat {
    //! Interop between `futures` 0.1 and 0.3.
    //!
    //! This module is only available when the `compat` feature of this
    //! library is activated.

    pub use futures_util::compat::{
        Compat, Compat01As03, Compat01As03Sink, CompatSink, Executor01As03, Executor01CompatExt,
        Executor01Future, Future01CompatExt, Sink01CompatExt, Stream01CompatExt,
    };

    #[cfg(feature = "io-compat")]
    #[cfg_attr(docsrs, doc(cfg(feature = "io-compat")))]
    pub use futures_util::compat::{AsyncRead01CompatExt, AsyncWrite01CompatExt};
}

#[cfg(feature = "executor")]
pub mod executor {
    //! Task execution.
    //!
    //! All asynchronous computation occurs within an executor, which is
    //! capable of spawning futures as tasks. This module provides several
    //! built-in executors, as well as tools for building your own.
    //!
    //! This module is only available when the `executor` feature of this
    //! library is activated, and it is activated by default.
    //!
    //! # Using a thread pool (M:N task scheduling)
    //!
    //! Most of the time tasks should be executed on a [thread
    //! pool](crate::executor::ThreadPool). A small set of worker threads can
    //! handle a very large set of spawned tasks (which are much lighter weight
    //! than threads). Tasks spawned onto the pool with the
    //! [`spawn_ok()`](crate::executor::ThreadPool::spawn_ok)
    //! function will run ambiently on the created threads.
    //!
    //! # Spawning additional tasks
    //!
    //! Tasks can be spawned onto a spawner by calling its
    //! [`spawn_obj`](crate::task::Spawn::spawn_obj) method directly.
    //! In the case of `!Send` futures,
    //! [`spawn_local_obj`](crate::task::LocalSpawn::spawn_local_obj)
    //! can be used instead.
    //!
    //! # Single-threaded execution
    //!
    //! In addition to thread pools, it's possible to run a task (and the tasks
    //! it spawns) entirely within a single thread via the
    //! [`LocalPool`](crate::executor::LocalPool) executor. Aside from cutting
    //! down on synchronization costs, this executor also makes it possible to
    //! spawn non-`Send` tasks, via
    //! [`spawn_local_obj`](crate::task::LocalSpawn::spawn_local_obj).
    //! The `LocalPool` is best suited for running I/O-bound tasks that do
    //! relatively little work between I/O operations.
    //!
    //! There is also a convenience function
    //! [`block_on`](crate::executor::block_on) for simply running a future to
    //! completion on the current thread.

    pub use futures_executor::{
        block_on, block_on_stream, enter, BlockingStream, Enter, EnterError, LocalPool,
        LocalSpawner,
    };

    #[cfg(feature = "thread-pool")]
    #[cfg_attr(docsrs, doc(cfg(feature = "thread-pool")))]
    pub use futures_executor::{ThreadPool, ThreadPoolBuilder};
}

pub mod future {
    //! Asynchronous values.
    //!
    //! This module contains:
    //!
    //! - The [`Future` trait](crate::future::Future).
    //! - The [`FutureExt`](crate::future::FutureExt) trait, which provides
    //!   adapters for chaining and composing futures.
    //! - Top-level future combinators like [`lazy`](crate::future::lazy) which
    //!   creates a future from a closure that defines its return value, and
    //!   [`ready`](crate::future::ready), which constructs a future with an
    //!   immediate defined value.

    pub use futures_core::future::{FusedFuture, Future, TryFuture};

    #[cfg(feature = "alloc")]
    pub use futures_core::future::{BoxFuture, LocalBoxFuture};

    pub use futures_task::{FutureObj, LocalFutureObj, UnsafeFutureObj};

    pub use futures_util::future::{
        err, join, join3, join4, join5, lazy, maybe_done, ok, pending, poll_fn, ready, select,
        try_join, try_join3, try_join4, try_join5, try_select, AndThen, Either, ErrInto, Flatten,
        FlattenSink, FlattenStream, Fuse, FutureExt, Inspect, InspectErr, InspectOk, IntoFuture,
        IntoStream, Join, Join3, Join4, Join5, Lazy, Map, MapErr, MapOk, MaybeDone, NeverError,
        OptionFuture, OrElse, Pending, PollFn, Ready, Select, Then, TryFlattenStream, TryFutureExt,
        TryJoin, TryJoin3, TryJoin4, TryJoin5, TrySelect, UnitError, UnwrapOrElse,
    };

    #[cfg(feature = "alloc")]
    pub use futures_util::future::{
        join_all, select_all, select_ok, try_join_all, JoinAll, SelectAll, SelectOk, TryJoinAll,
    };

    #[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
    #[cfg(feature = "alloc")]
    pub use futures_util::future::{abortable, AbortHandle, AbortRegistration, Abortable, Aborted};

    #[cfg(feature = "std")]
    pub use futures_util::future::{CatchUnwind, Remote, RemoteHandle, Shared};
}

#[cfg(feature = "std")]
pub mod io {
    //! Asynchronous I/O.
    //!
    //! This module is the asynchronous version of `std::io`. It defines four
    //! traits, [`AsyncRead`](crate::io::AsyncRead),
    //! [`AsyncWrite`](crate::io::AsyncWrite),
    //! [`AsyncSeek`](crate::io::AsyncSeek), and
    //! [`AsyncBufRead`](crate::io::AsyncBufRead), which mirror the `Read`,
    //! `Write`, `Seek`, and `BufRead` traits of the standard library. However,
    //! these traits integrate
    //! with the asynchronous task system, so that if an I/O object isn't ready
    //! for reading (or writing), the thread is not blocked, and instead the
    //! current task is queued to be woken when I/O is ready.
    //!
    //! In addition, the [`AsyncReadExt`](crate::io::AsyncReadExt),
    //! [`AsyncWriteExt`](crate::io::AsyncWriteExt),
    //! [`AsyncSeekExt`](crate::io::AsyncSeekExt), and
    //! [`AsyncBufReadExt`](crate::io::AsyncBufReadExt) extension traits offer a
    //! variety of useful combinators for operating with asynchronous I/O
    //! objects, including ways to work with them using futures, streams and
    //! sinks.
    //!
    //! This module is only available when the `std` feature of this
    //! library is activated, and it is activated by default.

    pub use futures_io::{
        AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, Error, ErrorKind, IoSlice, IoSliceMut,
        Result, SeekFrom,
    };

    #[cfg(feature = "read-initializer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "read-initializer")))]
    pub use futures_io::Initializer;

    pub use futures_util::io::{
        copy, copy_buf, empty, repeat, sink, AllowStdIo, AsyncBufReadExt, AsyncReadExt,
        AsyncSeekExt, AsyncWriteExt, BufReader, BufWriter, Chain, Close, Copy, CopyBuf, Cursor,
        Empty, Flush, IntoSink, Lines, Read, ReadExact, ReadHalf, ReadLine, ReadToEnd,
        ReadToString, ReadUntil, ReadVectored, Repeat, ReuniteError, Seek, Sink, Take, Window,
        Write, WriteAll, WriteHalf, WriteVectored,
    };
}

#[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
#[cfg(feature = "alloc")]
pub mod lock {
    //! Futures-powered synchronization primitives.
    //!
    //! This module is only available when the `std` or `alloc` feature of this
    //! library is activated, and it is activated by default.

    #[cfg(feature = "bilock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "bilock")))]
    pub use futures_util::lock::{BiLock, BiLockAcquire, BiLockGuard, ReuniteError};

    #[cfg(feature = "std")]
    pub use futures_util::lock::{MappedMutexGuard, Mutex, MutexGuard, MutexLockFuture};
}

pub mod prelude {
    //! A "prelude" for crates using the `futures` crate.
    //!
    //! This prelude is similar to the standard library's prelude in that you'll
    //! almost always want to import its entire contents, but unlike the
    //! standard library's prelude you'll have to do so manually:
    //!
    //! ```
    //! # #[allow(unused_imports)]
    //! use futures::prelude::*;
    //! ```
    //!
    //! The prelude may grow over time as additional items see ubiquitous use.

    pub use crate::future::{self, Future, TryFuture};
    pub use crate::sink::{self, Sink};
    pub use crate::stream::{self, Stream, TryStream};

    #[doc(no_inline)]
    pub use crate::future::{FutureExt as _, TryFutureExt as _};
    #[doc(no_inline)]
    pub use crate::sink::SinkExt as _;
    #[doc(no_inline)]
    pub use crate::stream::{StreamExt as _, TryStreamExt as _};

    #[cfg(feature = "std")]
    pub use crate::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite};

    #[cfg(feature = "std")]
    #[doc(no_inline)]
    pub use crate::io::{
        AsyncBufReadExt as _, AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _,
    };
}

pub mod sink {
    //! Asynchronous sinks.
    //!
    //! This module contains:
    //!
    //! - The [`Sink` trait](crate::sink::Sink), which allows you to
    //!   asynchronously write data.
    //! - The [`SinkExt`](crate::sink::SinkExt) trait, which provides adapters
    //!   for chaining and composing sinks.

    pub use futures_sink::Sink;

    pub use futures_util::sink::{
        drain, Close, Drain, Fanout, Flush, Send, SendAll, SinkErrInto, SinkExt, SinkMapErr, With,
        WithFlatMap,
    };

    #[cfg(feature = "alloc")]
    pub use futures_util::sink::Buffer;
}

pub mod stream {
    //! Asynchronous streams.
    //!
    //! This module contains:
    //!
    //! - The [`Stream` trait](crate::stream::Stream), for objects that can
    //!   asynchronously produce a sequence of values.
    //! - The [`StreamExt`](crate::stream::StreamExt) trait, which provides
    //!   adapters for chaining and composing streams.
    //! - Top-level stream constructors like [`iter`](crate::stream::iter)
    //!   which creates a stream from an iterator.

    pub use futures_core::stream::{FusedStream, Stream, TryStream};

    #[cfg(feature = "alloc")]
    pub use futures_core::stream::{BoxStream, LocalBoxStream};

    pub use futures_util::stream::{
        empty, iter, once, pending, poll_fn, repeat, select, try_unfold, unfold, AndThen, Chain,
        Collect, Concat, Empty, Enumerate, ErrInto, Filter, FilterMap, FlatMap, Flatten, Fold,
        ForEach, Forward, Fuse, Inspect, InspectErr, InspectOk, IntoStream, Iter, Map, MapErr,
        MapOk, Next, Once, OrElse, Peek, Peekable, Pending, PollFn, Repeat, Scan, Select,
        SelectNextSome, Skip, SkipWhile, StreamExt, StreamFuture, Take, TakeWhile, Then,
        TryCollect, TryConcat, TryFilter, TryFilterMap, TryFlatten, TryFold, TryForEach, TryNext,
        TrySkipWhile, TryStreamExt, TryTakeWhile, TryUnfold, Unfold, Zip,
    };

    #[cfg(feature = "alloc")]
    pub use futures_util::stream::{
        // For StreamExt:
        Chunks,
        ReadyChunks,
    };

    #[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
    #[cfg(feature = "alloc")]
    pub use futures_util::stream::{
        futures_unordered,
        select_all,
        // For StreamExt:
        BufferUnordered,
        Buffered,
        ForEachConcurrent,
        FuturesOrdered,
        FuturesUnordered,

        ReuniteError,

        SelectAll,
        SplitSink,
        SplitStream,
    };

    #[cfg(feature = "std")]
    pub use futures_util::stream::CatchUnwind;

    #[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
    #[cfg(feature = "alloc")]
    pub use futures_util::stream::{
        // For TryStreamExt:
        TryBufferUnordered,
        TryForEachConcurrent,
    };

    #[cfg(feature = "std")]
    pub use futures_util::stream::IntoAsyncRead;
}

pub mod task {
    //! Tools for working with tasks.
    //!
    //! This module contains:
    //!
    //! - [`Spawn`](crate::task::Spawn), a trait for spawning new tasks.
    //! - [`Context`](crate::task::Context), a context of an asynchronous task,
    //!   including a handle for waking up the task.
    //! - [`Waker`](crate::task::Waker), a handle for waking up a task.
    //!
    //! The remaining types and traits in the module are used for implementing
    //! executors or dealing with synchronization issues around task wakeup.

    pub use futures_core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    pub use futures_task::{
        FutureObj, LocalFutureObj, LocalSpawn, Spawn, SpawnError, UnsafeFutureObj,
    };

    pub use futures_util::task::noop_waker;

    #[cfg(feature = "std")]
    pub use futures_util::task::noop_waker_ref;

    #[cfg(feature = "alloc")]
    pub use futures_util::task::{LocalSpawnExt, SpawnExt};

    #[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
    #[cfg(feature = "alloc")]
    pub use futures_util::task::{waker, waker_ref, ArcWake, WakerRef};

    #[cfg_attr(feature = "cfg-target-has-atomic", cfg(target_has_atomic = "ptr"))]
    pub use futures_util::task::AtomicWaker;
}

pub mod never {
    //! This module contains the `Never` type.
    //!
    //! Values of this type can never be created and will never exist.

    pub use futures_util::never::Never;
}
