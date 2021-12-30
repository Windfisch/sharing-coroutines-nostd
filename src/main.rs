//#![no_std]
#![feature(generators, generator_trait, type_alias_impl_trait)]

use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;

struct YieldFuture {
	first: core::cell::Cell<bool>
}

impl Future for YieldFuture {
	type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut core::task::Context<'_>) -> core::task::Poll<Self::Output> {
		if self.first.get() == true {
			self.first.set(false);
			return core::task::Poll::Pending;
		}
		else {
			return core::task::Poll::Ready(());
		}
	}
}

fn fyield() -> YieldFuture {
	YieldFuture { first: core::cell::Cell::new(true) }
}

/** Contains a Future and a piece of data, sharing the data between
  * the Future and the outside world.
  * After creating with `new`, the resulting object must be pinned in memory
  * and then initialized using `init`. This makes the struct self-referential,
  * which is why it needs to be pinned from this point on.
  * `data`'s type is usually something like RefCell. Ensure that the coroutine
  * does not borrow the RefCell across yield points. */
struct FutureContainer<T, F: Future> {
	data: T,
	future: Option<F>
}

impl<'a, T: 'a, F: Future<Output=()> + 'a> FutureContainer<T, F> {
	/** Returns a new, but not yet usable container. The resulting
	 * container must be pinned in memory first and then initialized
	 * using `init`. */
	pub fn new(data: T) -> Self {
		FutureContainer {
			data,
			future: None
		}
	}

	/** Initializes the container, establishing the self-reference. This function
	  * must be called exactly once and must be called before any calls to `poll`. */
	pub fn init(self: Pin<&mut Self>, future_factory: impl FnOnce(&'a T) -> F) {
		assert!(self.future.is_none(), "init must not be called more than once");

		let data_ptr: *const T = &self.data;
		unsafe {
			// SAFETY: No Pin of `future` has been created yet, because `future` was None
			// until now. This is why we may use `future` in an unpinned context here.
			// Dereferencing `data_ptr` will store a reference to &self.data in self.future.
			// Since self.data can not be invalidated without destroying the whole self,
			// this is sound.
			self.get_unchecked_mut().future = Some(future_factory(&*data_ptr));
		}
	}

	/** Polls the underlying future. Must not be called before calling `init`. */
	pub fn poll(self: Pin<&mut Self>) {
		assert!(self.future.is_some(), "init must be called before polling");
		let pinned_future = unsafe {
			// SAFETY: Pin::new_unchecked is sound because self is pinned and we are never
			// moving out of pin; the only function that could do this is init, but poll
			// asserts that init has been called once and init asserts that init has not been
			// called yet.
			Pin::new_unchecked(self.get_unchecked_mut().future.as_mut().unwrap())
		};

		let waker = null_waker::create();
		let mut dummy_context = core::task::Context::from_waker(&waker);
		let _ = pinned_future.poll(&mut dummy_context);
	}

	pub fn data(self: &'a Pin<&Self>) -> &'a T {
		&self.data
	}
}

type MyFuture<'a> = impl Future<Output=()> + 'a;

fn make_future<'a>(data: &'a RefCell<u32>) -> MyFuture<'a> {
	async fn blah(data: &RefCell<u32>) {

	}
	blah(data)
}

fn main() {
	let mut bla = Box::pin(FutureContainer::<RefCell<u32>, MyFuture>::new(RefCell::new(42)));

	bla.as_mut().init(make_future);
	
	//bla.as_mut().init(|blubb: &_| make_future(blubb));
	
	//let x = |blubb| make_future(blubb);
	//bla.as_mut().init(x);

	bla.as_mut().poll();
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
