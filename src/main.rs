//#![no_std]
#![feature(generators, generator_trait, type_alias_impl_trait)]

use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;

struct FutureContainer<T, F: Future> {
	data: RefCell<T>,
	future: Option<F>
}

impl<'a, T: 'a, F: Future<Output=()> + 'a> FutureContainer<T, F> {
	pub fn new(data: T) -> Self {
		FutureContainer {
			data: RefCell::new(data),
			future: None
		}
	}

	pub fn init(self: Pin<&mut Self>, future_factory: impl FnOnce(&RefCell<T>) -> F) {
		assert!(self.future.is_some(), "init must not be called more than once");

		let data_ptr: *const RefCell<T> = &self.data;
		unsafe {
			// SAFETY: No Pin of `future` has been created yet, because `future` was None
			// until now. This is why we may use `future` in an unpinned context here.
			let bla: &'a _ = std::mem::transmute(&*data_ptr); // FIXME
			self.get_unchecked_mut().future = Some(future_factory(bla));
		}
	}

	pub fn poll(self: Pin<&mut Self>) {
		assert!(self.future.is_some(), "init must be called before polling");
		let pinned_future = unsafe {
			Pin::new_unchecked(self.get_unchecked_mut().future.as_mut().unwrap())
		};

		let waker = null_waker::create();
		let mut dummy_context = core::task::Context::from_waker(&waker);
		let _ = pinned_future.poll(&mut dummy_context);
	}

}

type MyFuture<'a> = impl Future<Output=()> + 'a;

fn make_future<'a>(data: &'a RefCell<u32>) -> MyFuture<'a> {
	async fn blah(data: &RefCell<u32>) {

	}
	blah(data)
}

fn main() {
	let mut bla = Box::pin(FutureContainer::<u32, MyFuture>::new(42));
	bla.as_mut().init(make_future);
	//bla.as_mut().init(|blubb: &_| make_future(blubb));
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
