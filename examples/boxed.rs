use std::cell::RefCell;
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
	// SAFETY: make_future keeps its `data` reference for itself and never hands it out
	let future_container = Box::pin(unsafe{ FutureContainer::new(RefCell::new(1), make_future) } );

	// as soon we have the future_container pinned in memory, we need to initialize
	// it so it becomes self-referential.
	future_container.as_ref().init();
	
	println!("poll {}", *future_container.as_ref().data().borrow()); // "poll 1"
	future_container.as_ref().poll(); // "hi 1"
	println!("poll {}", *future_container.as_ref().data().borrow()); // "poll 42"
	future_container.as_ref().poll(); // "hello 42"
	println!("poll {}", *future_container.as_ref().data().borrow()); // "poll 42"
	*future_container.as_ref().data().borrow_mut() = 1337;
	future_container.as_ref().poll(); // "bye 1337"
}
