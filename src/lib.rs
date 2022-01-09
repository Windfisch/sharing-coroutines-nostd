#![no_std]

use core::cell::{Cell, UnsafeCell};
use core::pin::Pin;
use core::future::Future;
use core::task::{Poll, Context};

/** A Future that needs to be `poll`ed exactly twice in order to get `Ready`.
  * Note that this future can not be used in the usual async executors such as tokio etc,
  * because it does not register a waker. Awaiting this future in such an executor will
  * block forever.
  */
pub struct YieldFuture {
	first: Cell<bool>
}

impl Future for YieldFuture {
	type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		if self.first.get() == true {
			self.first.set(false);
			return Poll::Pending;
		}
		else {
			return Poll::Ready(());
		}
	}
}

pub fn fyield() -> YieldFuture {
	YieldFuture { first: Cell::new(true) }
}

/** Contains a Future and a piece of data, sharing the data between
  * the Future and the outside world.
  * After creating with `new`, the resulting object must be pinned in memory
  * and then initialized using `init`. This makes the struct self-referential,
  * which is why it needs to be pinned from this point on.
  * `data`'s type is usually something like RefCell. Ensure that the coroutine
  * does not borrow the RefCell across yield points. */
pub struct FutureContainer<T, F: Future> {
	data: T,
	future: UnsafeCell<Option<F>>,
	init_func: for<'a> fn(&'a T) -> F
}

impl<'a, T: 'a, F: Future<Output=()>> FutureContainer<T, F> {
	/** Returns a new, but not yet usable container. The resulting
	 * container must be pinned in memory first and then initialized
	 * using `init`.
	 * `init_func` receives a reference of any lifetime, which is a lie!
	 * `init_func` must ensure to use that reference only within the future
	 * (which lives short enough). That's why `new` is unsafe.
	 */
	pub unsafe fn new(data: T, init_func: fn(&'a T) -> F) -> Self {
		FutureContainer {
			data,
			future: UnsafeCell::new(None),
			init_func: core::mem::transmute(init_func) // FIXME is this safe and why?
		}
	}

	/** Initializes the container, establishing the self-reference. This function
	  * must be called exactly once and must be called before any calls to `poll`. */
	pub fn init(self: Pin<&Self>) {
		unsafe {
			assert!((*self.future.get()).is_none(), "init must not be called more than once");
		}

		let data_ptr: *const T = &self.data;
		unsafe {
			// SAFETY: No Pin of `future` has been created yet, because `future` was None
			// until now. This is why we may use `future` in an unpinned context here.
			// Dereferencing `data_ptr` will store a reference to &self.data in self.future.
			// Since self.data can not be invalidated without destroying the whole self,
			// this is sound.
			*self.future.get() = Some((self.init_func)(&*data_ptr));
		}
	}

	/** Polls the underlying future. Must not be called before calling `init`. */
	pub fn poll(self: Pin<&Self>) {
		unsafe {
			assert!((*self.future.get()).is_some(), "init must be called before polling");
		}

		let pinned_future = unsafe {
			// SAFETY: Pin::new_unchecked is sound because self is pinned and we are never
			// moving out of pin; the only function that could do this is init, but poll
			// asserts that init has been called once and init asserts that init has not been
			// called yet.
			Pin::new_unchecked((*self.future.get()).as_mut().unwrap())
		};

		let waker = null_waker::create();
		let mut dummy_context = Context::from_waker(&waker);
		let _ = pinned_future.poll(&mut dummy_context);
	}

	/** Allows to access the data shared with the coroutine. */
	pub fn data(self: &'a Pin<&Self>) -> &'a T {
		&self.data
	}
}

mod null_waker
{
	// from https://blog.aloni.org/posts/a-stack-less-rust-coroutine-100-loc/

	use core::task::{RawWaker, RawWakerVTable, Waker};

	pub fn create() -> Waker {
		// Safety: The waker points to a vtable with functions that do nothing. Doing
		// nothing is memory-safe.
		unsafe { Waker::from_raw(RAW_WAKER) }
	}

	const RAW_WAKER: RawWaker = RawWaker::new(core::ptr::null(), &VTABLE);
	const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

	unsafe fn clone(_: *const ()) -> RawWaker { RAW_WAKER }
	unsafe fn wake(_: *const ()) { }
	unsafe fn wake_by_ref(_: *const ()) { }
	unsafe fn drop(_: *const ()) { }
}
