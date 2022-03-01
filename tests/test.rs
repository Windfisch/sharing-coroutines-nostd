use sharing_coroutines_nostd::*;
use std::pin::Pin;


async fn make_future(mut data: PointerWrapper<u32>) {
	println!("hi {}", *data.lock());
	*data.lock() = 42;
	fyield().await;
	println!("hello {}", *data.lock());
	fyield().await;
	println!("bye {}", *data.lock());
}

#[test]
fn main() {
	// SAFETY: make_future keeps its `data` reference for itself and never hands it out
	let mut future_container_unpinned = unsafe { FutureContainer::new(1, make_future) };
	
	// SAFETY: we may not use future_container_unpinned from this point on. Doing so might break
	// the Pin contract which states that the data pointed to by future_container may not be moved
	// in memory, e.g. by moving out of or into future_container_unpinned.
	let mut future_container = unsafe { Pin::new_unchecked(&mut future_container_unpinned) };

	// as soon we have the future_container pinned in memory, we need to initialize
	// it so it becomes self-referential.
	future_container.as_mut().init();

	println!("poll {}", *future_container.as_mut().data()); // "poll 1"
	future_container.as_mut().poll(); // "hi 1"
	println!("poll {}", *future_container.as_mut().data()); // "poll 42"
	future_container.as_mut().poll(); // "hello 42"
	println!("poll {}", *future_container.as_mut().data()); // "poll 42"
	*future_container.as_mut().data() = 1337;
	future_container.as_mut().poll(); // "bye 1337"
}
