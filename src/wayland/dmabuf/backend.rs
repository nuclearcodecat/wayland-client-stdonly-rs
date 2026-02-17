use crate::{
	App, Rl, rl,
	wayland::{
		Boxed, God, Id, IdentManager, WaylandError,
		buffer::{Buffer, BufferBackend},
		dmabuf::objects::{DmaBuf, DmaFeedback},
		registry::Registry,
		surface::Surface,
		wire::MessageManager,
	},
};

pub struct DmaBackend {
	pub(crate) dmabuf: Option<Rl<DmaBuf>>,
}

impl BufferBackend for DmaBackend {
	fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
		registry: &Rl<Registry>,
	) -> Result<Rl<Buffer>, WaylandError> {
		let lib = unsafe { libloading::Library::new("libgbm.so")? };
		let dmabuf = DmaBuf::new_registered_bound(god, &registry, surface)?;
		let feedback = DmaFeedback::new_registered_gotten(god, &dmabuf);
		self.dmabuf = Some(dmabuf);
		todo!()
	}

	fn resize(
		&mut self,
		wlmm: &mut MessageManager,
		wlim: &mut IdentManager,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaylandError> {
		todo!()
	}
}

impl DmaBackend {
	#[allow(clippy::new_ret_no_self)]
	pub fn new(app: &mut App) -> Result<Rl<Box<dyn BufferBackend>>, WaylandError> {
		Ok(rl!(DmaBackend {
			dmabuf: None,
		}
		.boxed() as Box<dyn BufferBackend>))
	}
}
