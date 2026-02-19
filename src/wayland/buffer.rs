use std::rc::Rc;

use crate::{
	DebugLevel, Rl, Wl, handle_log, rl,
	wayland::{
		God, Id, IdentManager, OpCode, Raw, WaytinierError, WaylandObject, WaylandObjectKind,
		registry::Registry,
		surface::Surface,
		wire::{Action, MessageManager, WireRequest},
	},
};

pub trait BufferBackend {
	fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
		registry: &Rl<Registry>,
	) -> Result<Rl<Buffer>, WaytinierError>;

	fn resize(
		&mut self,
		// this stinks
		wlmm: &mut MessageManager,
		wlim: &mut IdentManager,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaytinierError>;
}

pub(crate) struct Buffer {
	pub(crate) id: Id,
	pub(crate) offset: u32,
	pub(crate) w: u32,
	pub(crate) h: u32,
	pub(crate) in_use: bool,
	pub(crate) master: Wl<Surface>,
	pub(crate) slice: Option<*mut [u8]>,
	pub(crate) backend: Rl<Box<dyn BufferBackend>>,
}

impl Buffer {
	pub(crate) fn new(
		id: Id,
		(offset, width, height): (u32, u32, u32),
		master: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
	) -> Rl<Self> {
		rl!(Self {
			id,
			offset,
			w: width,
			h: height,
			in_use: false,
			master: Rc::downgrade(master),
			slice: None,
			backend: backend.clone(),
		})
	}

	pub(crate) fn new_registered(
		god: &mut God,
		(offset, width, height): (u32, u32, u32),
		master: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		let buf = Self::new(Id(0), (offset, width, height), master, backend);
		let id = god.wlim.new_id_registered(buf.clone());
		buf.borrow_mut().id = id;
		Ok(buf)
	}

	pub(crate) fn get_slice(&mut self) -> Result<*mut [u8], WaytinierError> {
		self.slice.ok_or(WaytinierError::ExpectedSomeValue("no memory slice in buf"))
	}

	pub(crate) fn wl_destroy(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "destroy",
			args: vec![],
		}
	}
}

impl WaylandObject for Buffer {
	fn handle(
		&mut self,
		_payload: &[u8],
		opcode: OpCode,
		_fds: &[std::os::unix::prelude::OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			// release
			0 => {
				self.in_use = false;
				handle_log!(pending, self, DebugLevel::Verbose, String::from("released"));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Buffer
	}
}
