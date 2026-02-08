use crate::wayland::buffer::BufferBackend;

#[derive(Default)]
pub struct ShmBackend {}

impl BufferBackend for ShmBackend {
	fn new_buffer(&mut self) -> super::buffer::Buffer {
		todo!()
	}
}
