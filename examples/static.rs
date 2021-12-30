#![feature(type_alias_impl_trait)]
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::mem::MaybeUninit;

use sharing_coroutines_nostd::*;

type MyFuture<'a> = impl Future<Output=()> + 'a;

// The usual trick to generate a "defining use" of the MyFuture type. This is needed
// to give the name `MyFuture` to the anonymous Future returned by the `foo` function.
fn make_future<'a>(data: &'a RefCell<u32>) -> MyFuture<'a> {
	async fn foo(data: &RefCell<u32>) {
		println!("hi {}", *data.borrow());
		*data.borrow_mut() = 42;
		fyield().await;
		println!("hello {}", *data.borrow());
		fyield().await;
		println!("bye {}", *data.borrow());
	}
	foo(data)
}

static mut FUTURE_CONTAINER: MaybeUninit<FutureContainer::<RefCell<u32>, MyFuture>> = MaybeUninit::uninit();

fn main() {
	// SAFTEY: This must be the only use of FUTURE_CONTAINER, to ensure that 1. there are no concurrent / aliasing
	// references to it, and 2. that the Pin contract is upheld.
	let mut future_container = unsafe {
		FUTURE_CONTAINER.write(FutureContainer::new(RefCell::new(1)));
		Pin::new_unchecked(FUTURE_CONTAINER.assume_init_mut())
	};

	// as soon we have the future_container pinned in memory, we need to initialize
	// it so it becomes self-referential.
	future_container.as_mut().init(make_future);
	
	println!("poll {}", *future_container.as_ref().data().borrow()); // "poll 1"
	future_container.as_mut().poll(); // "hi 1"
	println!("poll {}", *future_container.as_ref().data().borrow()); // "poll 42"
	future_container.as_mut().poll(); // "hello 42"
	println!("poll {}", *future_container.as_ref().data().borrow()); // "poll 42"
	*future_container.as_ref().data().borrow_mut() = 1337;
	future_container.as_mut().poll(); // "bye 1337"
}
