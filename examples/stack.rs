use std::cell::RefCell;
use std::pin::Pin;
use sharing_coroutines_nostd::*;

async fn make_future(data: &RefCell<u32>) {
	println!("hi {}", *data.borrow());
	*data.borrow_mut() = 42;
	fyield().await;
	println!("hello {}", *data.borrow());
	fyield().await;
	println!("bye {}", *data.borrow());
}

fn main() {
	let mut future_container_unpinned = FutureContainer::new(RefCell::new(1));
	let mut future_container = unsafe { Pin::new_unchecked(&mut future_container_unpinned) };
	// SAFETY: we may not use future_container_unpinned from this point on. Doing so might break
	// the Pin contract which states that the data pointed to by future_container may not be moved
	// in memory, e.g. by moving out of or into future_container_unpinned.

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