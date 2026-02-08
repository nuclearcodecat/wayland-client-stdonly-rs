pub trait BufferBackend {
	fn new_buffer(&mut self) -> Buffer;
}

pub(crate) struct Buffer {}
