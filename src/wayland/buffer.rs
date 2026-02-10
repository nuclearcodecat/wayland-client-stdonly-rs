use std::rc::Rc;

use crate::{
	Rl, Wl, rl,
	wayland::{
		God, Id, OpCode, WaylandError, WaylandObject, WaylandObjectKind, surface::Surface,
		wire::Action,
	},
};

pub trait BufferBackend {
	fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
	) -> Result<Rl<Buffer>, WaylandError>;
}

pub(crate) struct Buffer {
	pub(crate) id: Id,
	pub(crate) offset: u32,
	pub(crate) w: u32,
	pub(crate) h: u32,
	pub(crate) in_use: bool,
	pub(crate) master: Wl<Surface>,
	pub(crate) slice: Option<*mut [u8]>,
}

impl Buffer {
	pub(crate) fn new(
		id: Id,
		(offset, width, height): (u32, u32, u32),
		master: &Rl<Surface>,
	) -> Rl<Self> {
		rl!(Self {
			id,
			offset,
			w: width,
			h: height,
			in_use: false,
			master: Rc::downgrade(master),
			slice: None,
		})
	}

	pub(crate) fn new_registered(
		god: &mut God,
		(offset, width, height): (u32, u32, u32),
		master: &Rl<Surface>,
	) -> Result<Rl<Buffer>, WaylandError> {
		let buf = Self::new(Id(0), (offset, width, height), master);
		let id = god.wlim.new_id_registered(buf.clone());
		buf.borrow_mut().id = id;
		Ok(buf)
	}

	pub(crate) fn get_slice(&mut self) -> Result<*mut [u8], WaylandError> {
		self.slice.ok_or(WaylandError::RequiredValueNone("no memory slice in buf"))
	}
}

impl WaylandObject for Buffer {
	fn handle(
		&mut self,
		_payload: &[u8],
		_opcode: OpCode,
		_fds: &[std::os::unix::prelude::OwnedFd],
	) -> Result<Vec<Action>, WaylandError> {
		todo!()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Buffer
	}
}
