#![no_std]
use pin_project::pin_project;
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

/** Contains a Option<Future> and a piece of data, sharing the data between
  * the Future and the outside world.
  * After creating with `new`, the resulting object must be pinned in memory
  * and then initialized using `init`. This makes the struct self-referential,
  * which is why it needs to be pinned from this point on.
  * `FutureContainer` has Option-like semantics, as it can either contain or
  * not contain an initialized Future.
  * `data`'s type is usually something like RefCell. Ensure that the coroutine
  * does not borrow the RefCell across yield points. */
#[pin_project]
pub struct FutureContainer<T, F: Future> {
	data: T,
	data_locked: bool,
	#[pin]
	future: Option<F>,
	init_func: fn(PointerWrapper<T>) -> F
}

pub struct PointerWrapper<T> {
	data_ptr: *mut T,
	data_locked_ptr: *mut bool
}

impl<T> PointerWrapper<T> {
	pub fn lock(&mut self) -> PointerGuard<T> {
		unsafe {
			// SAFETY: no reference to *data_locked_ptr can exist TODO WHY
			if *self.data_locked_ptr {
				unreachable!("Attempted to lock PointerWrapper twice!");
			}
			*self.data_locked_ptr = true;
		
			return PointerGuard { data_ptr: &mut *self.data_ptr, data_locked_ptr: self.data_locked_ptr }
		}
	}
}

pub struct PointerGuard<'a, T> {
	data_ptr: &'a mut T,
	data_locked_ptr: *mut bool
}

impl<T> core::ops::Deref for PointerGuard<'_, T> {
	type Target = T;

	fn deref(&self) -> &T {
		return self.data_ptr;
	}
}

impl<T> core::ops::DerefMut for PointerGuard<'_, T> {
	fn deref_mut(&mut self) -> &mut T {
		return self.data_ptr;
	}
}

impl<T> core::ops::Drop for PointerGuard<'_, T> {
	fn drop(&mut self) {
		unsafe {
			*self.data_locked_ptr = false;
		}
	}
}

impl<'a, T: 'a, F: Future<Output=()>> FutureContainer<T, F> {
	/** Returns a new, but not yet usable container. The resulting
	 * container must be pinned in memory first and then initialized
	 * using `init`.
	 * `init_func` receives a reference of any lifetime, which is a lie!
	 * `init_func` must ensure to use that reference only within the future
	 * (which lives short enough). That's why `new` is unsafe.
	 */
	pub unsafe fn new(data: T, init_func: fn(PointerWrapper<T>) -> F) -> Self {
		FutureContainer {
			data,
			data_locked: false,
			future: None,
			init_func
		}
	}

	/** Initializes the container, establishing the self-reference. This function
	  * must be called before any calls to `poll`. */
	pub fn init(self: Pin<&mut Self>) {
		let mut this = self.project();
		let mut pointer_wrapper = PointerWrapper {
			data_ptr: this.data,
			data_locked_ptr: this.data_locked
		};
		{ pointer_wrapper.lock(); } // DEBUG
		this.future.set(Some((this.init_func)(pointer_wrapper)));
	}

	pub fn clear(self: Pin<&mut Self>) {
		let mut this = self.project();
		this.future.set(None);
	}

	pub fn is_init(&self) -> bool {
		self.future.is_some()
	}

	/** Polls the underlying future. Must not be called before calling `init`. */
	pub fn poll(self: Pin<&mut Self>) {
		let this = self.project();

		let pinned_future = this.future.as_pin_mut().expect("cannot poll an uninitialized future");
		let waker = null_waker::create();
		let mut dummy_context = Context::from_waker(&waker);
		let _ = pinned_future.poll(&mut dummy_context);

		assert!(!*this.data_locked, "Coroutine data lock held across yield point!");
	}

	/** Allows to access the data shared with the coroutine. */
	pub fn data(self: Pin<&'a mut Self>) -> &'a mut T {
		self.project().data
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
