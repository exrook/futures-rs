//! Task notification

cfg_target_has_atomic! {
    #[cfg(feature = "alloc")]
    pub use futures_task::ArcWake;

    #[cfg(feature = "alloc")]
    pub use futures_task::waker;

    #[cfg(feature = "alloc")]
    pub use futures_task::{waker_ref, WakerRef};

    pub use futures_core::task::__internal::AtomicWaker;
}

mod spawn;
pub use self::spawn::{LocalSpawnExt, SpawnExt};

pub use futures_core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub use futures_task::{FutureObj, LocalFutureObj, LocalSpawn, Spawn, SpawnError, UnsafeFutureObj};

pub use futures_task::noop_waker;
#[cfg(feature = "std")]
pub use futures_task::noop_waker_ref;
