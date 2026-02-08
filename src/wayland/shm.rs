use crate::wayland::buffer::BufferBackend;

pub struct ShmBackend {}

impl BufferBackend for ShmBackend {
	fn new() -> Self {
		Self {}
	}
}
