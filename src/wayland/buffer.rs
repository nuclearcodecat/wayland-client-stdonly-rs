use std::{os::fd::OwnedFd, rc::Rc};

use crate::{
	DebugLevel, DmaBackend, Rl, ShmBackend, Wl, handle_log, rl,
	wayland::{
		God, Id, OpCode, Raw, WaylandObject, WaylandObjectKind, WaytinierError,
		registry::Registry,
		surface::Surface,
		wire::{Action, WireRequest},
	},
};

pub enum BufferBackend {
	Shm(ShmBackend),
	Dma(DmaBackend),
}

impl BufferBackend {
	pub(crate) fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
		backend: &Rl<BufferBackend>,
		registry: &Rl<Registry>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		match self {
			BufferBackend::Shm(shm_backend) => {
				shm_backend.make_buffer(god, w, h, surface, backend, registry)
			}
			BufferBackend::Dma(dma_backend) => {
				dma_backend.make_buffer(god, w, h, surface, backend, registry)
			}
		}
	}

	pub(crate) fn resize(
		&mut self,
		god: &mut God,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaytinierError> {
		match self {
			BufferBackend::Shm(shm_backend) => shm_backend.resize(god, buf, w, h),
			BufferBackend::Dma(dma_backend) => dma_backend.resize(god, buf, w, h),
		}
	}
}

pub enum BufferAccessor {
	ShmSlice(*mut [u8]),
	DmaBufFd(OwnedFd),
}

pub(crate) struct Buffer {
	pub(crate) id: Id,
	pub(crate) offset: u32,
	pub(crate) w: u32,
	pub(crate) h: u32,
	pub(crate) in_use: bool,
	pub(crate) master: Wl<Surface>,
	pub(crate) backend: Rl<BufferBackend>,
	pub(crate) accessor: Option<BufferAccessor>,
}

impl Buffer {
	pub(crate) fn new(
		id: Id,
		(offset, width, height): (u32, u32, u32),
		master: &Rl<Surface>,
		backend: &Rl<BufferBackend>,
		accessor: Option<BufferAccessor>,
	) -> Rl<Self> {
		rl!(Self {
			id,
			offset,
			w: width,
			h: height,
			in_use: false,
			master: Rc::downgrade(master),
			backend: backend.clone(),
			accessor,
		})
	}

	pub(crate) fn new_registered(
		god: &mut God,
		(offset, width, height): (u32, u32, u32),
		master: &Rl<Surface>,
		backend: &Rl<BufferBackend>,
		accessor: Option<BufferAccessor>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		let buf = Self::new(Id(0), (offset, width, height), master, backend, accessor);
		let id = god.wlim.new_id_registered(buf.clone());
		buf.borrow_mut().id = id;
		Ok(buf)
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
