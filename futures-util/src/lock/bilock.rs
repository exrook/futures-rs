//! Futures-powered synchronization primitives.

#[cfg(feature = "bilock")]
use futures_core::future::Future;
use futures_core::task::{Context, Poll, Waker};
use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering::{SeqCst, Acquire, Release, AcqRel, Relaxed};
use core::mem;
use alloc::boxed::Box;
use alloc::sync::Arc;

/// A type of futures-powered synchronization primitive which is a mutex between
/// two possible owners.
///
/// This primitive is not as generic as a full-blown mutex but is sufficient for
/// many use cases where there are only two possible owners of a resource. The
/// implementation of `BiLock` can be more optimized for just the two possible
/// owners.
///
/// Note that it's possible to use this lock through a poll-style interface with
/// the `poll_lock` method but you can also use it as a future with the `lock`
/// method that consumes a `BiLock` and returns a future that will resolve when
/// it's locked.
///
/// A `BiLock` is typically used for "split" operations where data which serves
/// two purposes wants to be split into two to be worked with separately. For
/// example a TCP stream could be both a reader and a writer or a framing layer
/// could be both a stream and a sink for messages. A `BiLock` enables splitting
/// these two and then using each independently in a futures-powered fashion.
///
/// This type is only available when the `bilock` feature of this
/// library is activated.
#[cfg_attr(docsrs, doc(cfg(feature = "bilock")))]
pub struct BiLock<T> {
    arc: Arc<Inner<T>>,
    index: bool,
}

impl<T> fmt::Debug for BiLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BiLock").field("arc", &self.arc).field("index", &self.index).finish()
    }
}

struct Inner<T> {
    state: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
    value: Option<UnsafeCell<T>>,
}

impl<T> fmt::Debug for Inner<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner").field("state", &self.state).field("waker", &self.waker).finish()
    }
}

unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send> Sync for Inner<T> {}

impl<T> BiLock<T> {
    /// Creates a new `BiLock` protecting the provided data.
    ///
    /// Two handles to the lock are returned, and these are the only two handles
    /// that will ever be available to the lock. These can then be sent to separate
    /// tasks to be managed there.
    ///
    /// The data behind the bilock is considered to be pinned, which allows `Pin`
    /// references to locked data. However, this means that the locked value
    /// will only be available through `Pin<&mut T>` (not `&mut T`) unless `T` is `Unpin`.
    /// Similarly, reuniting the lock and extracting the inner value is only
    /// possible when `T` is `Unpin`.
    pub fn new(t: T) -> (Self, Self) {
        let arc = Arc::new(Inner {
            state: AtomicUsize::new(0),
            value: Some(UnsafeCell::new(t)),
            waker: UnsafeCell::new(None),
        });

        (Self { arc: arc.clone(), index: true }, Self { arc, index: false })
    }

    /// Attempt to acquire this lock, returning `Pending` if it can't be
    /// acquired.
    ///
    /// This function will acquire the lock in a nonblocking fashion, returning
    /// immediately if the lock is already held. If the lock is successfully
    /// acquired then `Poll::Ready` is returned with a value that represents
    /// the locked value (and can be used to access the protected data). The
    /// lock is unlocked when the returned `BiLockGuard` is dropped.
    ///
    /// If the lock is already held then this function will return
    /// `Poll::Pending`. In this case the current task will also be scheduled
    /// to receive a notification when the lock would otherwise become
    /// available.
    ///
    /// # Panics
    ///
    /// This function will panic if called outside the context of a future's
    /// task.
    pub fn poll_lock(&mut self, cx: &mut Context<'_>) -> Poll<BiLockGuard<'_, T>> {
        let xor = if self.index { 0b10 } else { 0b01 };
        let other = if !self.index { 0b10 } else { 0b01 };
        match self.arc.state.fetch_or(xor, AcqRel) {
            x if x & (xor<<2) != 0 => { // Other half has set our waker lock bit high
                self.arc.state.fetch_and((other<<2)|0b11, Relaxed); // clear the lock bit
                Poll::Ready(BiLockGuard { bilock: self })
            }
            x if x&other == 0 => { // uncontended
                Poll::Ready(BiLockGuard { bilock: self })
            }
            x if x & other != 0 => { // Other 
                let mut state = self.arc.state.fetch_or(xor<<2, AcqRel); // attempt to lock waker
                if state & (xor<<2) != 0 {
                    self.arc.state.fetch_and((other<<2)|0b11, Release); // clear the lock bit
                    Poll::Ready(BiLockGuard { bilock: self })
                } else {
                loop {
                    match state {
                        x if x&0b11 == xor => { // lock is ours
                            self.arc.state.fetch_and((other<<2) | 0b11, Release); // free waker lock
                            break Poll::Ready(BiLockGuard{ bilock: self })
                        }
                        x if x&(other<<2) == 0 => { // waker lock is ours
                            unsafe {
                                ptr::replace(self.arc.waker.get(), Some(cx.waker().clone()));
                            }
                            if self.arc.state.fetch_and((other<<2)|0b11, AcqRel)&0b11 == xor { // free waker lock
                                break Poll::Ready(BiLockGuard{ bilock: self }) // lock is now ours
                            } else {
                                break Poll::Pending
                            }
                        }
                        x if x & (other << 2) != 0 => { // contention on waker lock
                            state = self.arc.state.load(Acquire)
                        }
                        _ => unreachable!()
                    }
                }
                }
            }
            _ => unreachable!()
        }
    }

    /// Perform a "blocking lock" of this lock, consuming this lock handle and
    /// returning a future to the acquired lock.
    ///
    /// This function consumes the `BiLock<T>` and returns a sentinel future,
    /// `BiLockAcquire<T>`. The returned future will resolve to
    /// `BiLockAcquired<T>` which represents a locked lock similarly to
    /// `BiLockGuard<T>`.
    ///
    /// Note that the returned future will never resolve to an error.
    #[cfg(feature = "bilock")]
    #[cfg_attr(docsrs, doc(cfg(feature = "bilock")))]
    pub fn lock(self) -> BiLockAcquire<T> {
        BiLockAcquire {
            bilock: Some(self),
        }
    }

    /// Attempts to put the two "halves" of a `BiLock<T>` back together and
    /// recover the original value. Succeeds only if the two `BiLock<T>`s
    /// originated from the same call to `BiLock::new`.
    pub fn reunite(self, other: Self) -> Result<T, ReuniteError<T>>
    where
        T: Unpin,
    {
        if Arc::ptr_eq(&self.arc, &other.arc) {
            drop(other);
            let inner = Arc::try_unwrap(self.arc)
                .ok()
                .expect("futures: try_unwrap failed in BiLock<T>::reunite");
            Ok(unsafe { inner.into_value() })
        } else {
            Err(ReuniteError(self, other))
        }
    }

    fn unlock(&mut self) {
        let xor = if self.index { 0b10 } else { 0b01 };
        let other = if !self.index { 0b10 } else { 0b01 };
        match self.arc.state.fetch_xor(xor, AcqRel) {
            x if x&other == 0 => {} // uncontended, do nothing
            x if x&other != 0 => { // other half is waiting, attempt to wake
                let mut state = self.arc.state.fetch_or(xor<<2, AcqRel); // attempt to lock waker
                loop {
                    match state {
                        x if x&(other<<2) == 0 => { // waker lock is ours
                            let waker = unsafe {
                                ptr::replace(self.arc.waker.get(), None)
                            };
                            if let Some(waker) = waker {
                                waker.wake()
                            } else {
                            }
                            self.arc.state.fetch_xor(0b1100, Release);// free waker lock, try to set other half's bit high
                            break;
                        }
                        x if x & (other << 2) != 0 => { // contention on waker lock
                            state = self.arc.state.load(Acquire)
                        }
                        _ => unreachable!()
                    }
                }
            }
            _ => unreachable!()
        }
    }
}

impl<T: Unpin> Inner<T> {
    unsafe fn into_value(mut self) -> T {
        self.value.take().unwrap().into_inner()
    }
}

impl<T> Drop for Inner<T> {
    fn drop(&mut self) {
        //assert_eq!(self.state.load(SeqCst)|0b11, 0);
    }
}

/// Error indicating two `BiLock<T>`s were not two halves of a whole, and
/// thus could not be `reunite`d.
#[cfg_attr(docsrs, doc(cfg(feature = "bilock")))]
pub struct ReuniteError<T>(pub BiLock<T>, pub BiLock<T>);

impl<T> fmt::Debug for ReuniteError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReuniteError")
            .field(&"...")
            .finish()
    }
}

impl<T> fmt::Display for ReuniteError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tried to reunite two BiLocks that don't form a pair")
    }
}

#[cfg(feature = "std")]
impl<T: core::any::Any> std::error::Error for ReuniteError<T> {}

/// Returned RAII guard from the `poll_lock` method.
///
/// This structure acts as a sentinel to the data in the `BiLock<T>` itself,
/// implementing `Deref` and `DerefMut` to `T`. When dropped, the lock will be
/// unlocked.
#[derive(Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "bilock")))]
pub struct BiLockGuard<'a, T> {
    bilock: &'a mut BiLock<T>,
}

impl<T> Deref for BiLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.bilock.arc.value.as_ref().unwrap().get() }
    }
}

impl<T: Unpin> DerefMut for BiLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.bilock.arc.value.as_ref().unwrap().get() }
    }
}

impl<T> BiLockGuard<'_, T> {
    /// Get a mutable pinned reference to the locked value.
    pub fn as_pin_mut(&mut self) -> Pin<&mut T> {
        // Safety: we never allow moving a !Unpin value out of a bilock, nor
        // allow mutable access to it
        unsafe { Pin::new_unchecked(&mut *self.bilock.arc.value.as_ref().unwrap().get()) }
    }
}

impl<T> Drop for BiLockGuard<'_, T> {
    fn drop(&mut self) {
        self.bilock.unlock();
    }
}

#[derive(Debug)]
pub struct BiLockAcquired<T> {
    bilock: BiLock<T>
}

impl<T> Deref for BiLockAcquired<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.bilock.arc.value.as_ref().unwrap().get() }
    }
}

impl<T: Unpin> DerefMut for BiLockAcquired<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.bilock.arc.value.as_ref().unwrap().get() }
    }
}

impl<T> BiLockAcquired<T> {
    /// Get a mutable pinned reference to the locked value.
    pub fn as_pin_mut(&mut self) -> Pin<&mut T> {
        // Safety: we never allow moving a !Unpin value out of a bilock, nor
        // allow mutable access to it
        unsafe { Pin::new_unchecked(&mut *self.bilock.arc.value.as_ref().unwrap().get()) }
    }
    pub fn unlock(self) -> BiLock<T> {
        let mut bilock = self.bilock;
        mem::drop(BiLockGuard { bilock: &mut bilock });
        bilock
    }
}

/// Future returned by `BiLock::lock` which will resolve when the lock is
/// acquired.
#[cfg(feature = "bilock")]
#[cfg_attr(docsrs, doc(cfg(feature = "bilock")))]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[derive(Debug)]
pub struct BiLockAcquire<T> {
    bilock: Option<BiLock<T>>,
}

// Pinning is never projected to fields
#[cfg(feature = "bilock")]
impl<T> Unpin for BiLockAcquire<T> {}

#[cfg(feature = "bilock")]
impl<T> Future for BiLockAcquire<T> {
    type Output = BiLockAcquired<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let bilock = self.bilock.as_mut().expect("Cannot poll after Ready");
        match bilock.poll_lock(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(guard) => {
                mem::forget(guard);
            }
        }
        Poll::Ready(BiLockAcquired {
            bilock: self.bilock.take().unwrap()
        })
    }
}
