use crate::wayland::{buffer::BufferBackend, dmabuf::objects::DmaBuf};

pub(crate) struct DmaBackend {
	pub(crate) buf_obj: Rl<DmaBuf>,
}

impl BufferBackend for DmaBackend {
	fn make_buffer(
		&mut self,
		god: &mut super::God,
		w: u32,
		h: u32,
		surface: &crate::Rl<super::surface::Surface>,
		backend: &crate::Rl<Box<dyn BufferBackend>>,
	) -> Result<crate::Rl<super::buffer::Buffer>, super::WaylandError> {
		todo!()
	}

	fn resize(
		&mut self,
		// this stinks
		wlmm: &mut super::wire::MessageManager,
		wlim: &mut super::IdentManager,
		buf: &crate::Rl<super::buffer::Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), super::WaylandError> {
		todo!()
	}
}
