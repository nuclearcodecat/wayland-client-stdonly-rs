use std::{
	os::fd::{AsRawFd, OwnedFd},
	ptr::null_mut,
	rc::Rc,
};

use libc::{MAP_FAILED, MAP_PRIVATE, PROT_READ};

use crate::{
	DebugLevel, PixelFormat, Rl, Wl, rl,
	wayland::{
		ExpectRc, God, Id, OpCode, Raw, WaylandObject, WaylandObjectKind, WaytinierError,
		registry::Registry,
		surface::Surface,
		wire::{Action, FromWirePayload, WireArgument, WireRequest},
	},
};

pub(crate) struct DmaBuf {
	pub(crate) id: Id,
	pub(crate) formats: Vec<u32>,
	pub(crate) modifiers: Vec<u32>,
	pub(crate) surface: Wl<Surface>,
}

pub(crate) struct DmaFeedback {
	pub(crate) id: Id,
	pub(crate) parent: Wl<DmaBuf>,
	pub(crate) done: bool,
	pub(crate) format_table: Vec<(u32, u64)>,
	pub(crate) format_indices: Vec<u16>,
	pub(crate) target_device: Option<u32>,
}

impl WaylandObject for DmaBuf {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let format = u32::from_wire(payload)?;
				match PixelFormat::from_u32(format) {
					Ok(_) => {
						self.formats.push(format);
					}
					Err(_) => {
						pending.push(Action::Trace(
							DebugLevel::Error,
							self.kind_str(),
							format!("found unrecognized pixel format: 0x{:08x}", format),
						));
					}
				};
				pending.push(Action::Trace(
					DebugLevel::Trivial,
					self.kind_str(),
					format!("found format for dmabuf: 0x{:08x}", format),
				));
			}
			1 => {
				let modifier = u32::from_wire(payload)?;
				self.modifiers.push(modifier);
				pending.push(Action::Trace(
					DebugLevel::Trivial,
					self.kind_str(),
					format!("found modifier for dmabuf: {modifier}"),
				));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaBuf
	}
}

impl DmaBuf {
	pub(crate) fn new(id: Id, surface: &Rl<Surface>) -> Rl<Self> {
		rl!(Self {
			id,
			surface: Rc::downgrade(surface),
			formats: vec![],
			modifiers: vec![],
		})
	}

	pub(crate) fn new_registered(god: &mut God, surface: &Rl<Surface>) -> Rl<DmaBuf> {
		let dmabuf = DmaBuf::new(Id(0), surface);
		let id = god.wlim.new_id_registered(dmabuf.clone());
		dmabuf.borrow_mut().id = id;
		dmabuf
	}

	pub(crate) fn new_registered_bound(
		god: &mut God,
		registry: &Rl<Registry>,
		surface: &Rl<Surface>,
	) -> Result<Rl<Self>, WaytinierError> {
		let new = Self::new_registered(god, surface);
		let dbuf = new.clone();
		let dbuf = dbuf.borrow();
		registry.borrow_mut().bind(god, dbuf.id, dbuf.kind(), 5)?;
		Ok(new)
	}

	fn wl_get_default_feedback(&self, new_id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(2),
			opname: "get_default_feedback",
			args: vec![WireArgument::NewId(new_id)],
		}
	}
}

impl WaylandObject for DmaFeedback {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			// done
			0 => {
				self.done = true;
			}
			// format_table
			1 => {
				let size = u32::from_wire(payload)? as usize;
				let fd = _fds.first().ok_or(WaytinierError::FdExpected)?;
				let ptr = unsafe {
					libc::mmap(null_mut(), size, PROT_READ, MAP_PRIVATE, fd.as_raw_fd(), 0)
				};
				if ptr == MAP_FAILED {
					return Err(std::io::Error::last_os_error().into());
				}
				let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as *mut u8, size) };
				self.parse_format_table(slice)?;

				pending.push(Action::Trace(
					DebugLevel::Important,
					self.kind_str(),
					format!("size: {size}, fd: {:?}", _fds),
				));
			}
			// main_device
			2 => {
				let main_device: Vec<u32> = Vec::from_wire(payload)?;
				pending.push(Action::Trace(
					DebugLevel::Important,
					self.kind_str(),
					format!("main_device: {:?}", main_device),
				));
			}
			// tranche_done
			3 => {
				pending.push(Action::Trace(
					DebugLevel::Important,
					self.kind_str(),
					String::from("tranche done"),
				));
			}
			// tranche_target_device
			4 => {
				let target_device: Vec<u32> = Vec::from_wire(payload)?;
				self.target_device = Some(target_device[0]);
				pending.push(Action::Trace(
					DebugLevel::Important,
					self.kind_str(),
					format!("tranche target device: {:?}", target_device),
				));
			}
			// tranche_formats
			5 => {
				let indices: Vec<u16> = Vec::from_wire(payload)?;
				self.format_indices = indices;
				pending.push(Action::Trace(
					DebugLevel::SuperVerbose,
					self.kind_str(),
					format!("tranche indices: {:?}", self.format_indices),
				));
				let pf = self
					.parent
					.upgrade()
					.to_wl_err()?
					.borrow()
					.surface
					.upgrade()
					.to_wl_err()?
					.borrow()
					.pf;
				for ix in &self.format_indices {
					let entry = self.format_table[*ix as usize];
					pending.push(Action::Trace(
						DebugLevel::Verbose,
						self.kind_str(),
						format!("tranche format {ix}: {:?}", entry),
					));
					if entry.0 == pf.to_fourcc() {
						pending.push(Action::Trace(
							DebugLevel::Important,
							self.kind_str(),
							String::from("found desired pixelformat, {}: {:?}"),
						));
					}
				}
			}
			// tranche_flags
			6 => {
				let flags = u32::from_wire(payload)?;
				let mut v = vec![];
				if flags & TrancheFlags::Scanout as u32 != 0 {
					v.push(TrancheFlags::Scanout);
				};
				pending.push(Action::Trace(
					DebugLevel::Trivial,
					self.kind_str(),
					format!("tranche flags: {:?}", v),
				));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaFeedback
	}
}

impl DmaFeedback {
	pub(crate) fn new(id: Id, parent: &Rl<DmaBuf>) -> Rl<Self> {
		rl!(Self {
			id,
			parent: Rc::downgrade(parent),
			done: false,
			format_table: vec![],
			format_indices: vec![],
			target_device: None,
		})
	}

	pub(crate) fn new_registered(god: &mut God, parent: &Rl<DmaBuf>) -> Rl<Self> {
		let new = Self::new(Id(0), parent);
		let id = god.wlim.new_id_registered(new.clone());
		new.borrow_mut().id = id;
		new
	}

	pub(crate) fn new_registered_gotten(god: &mut God, dmabuf: &Rl<DmaBuf>) -> Rl<Self> {
		let new = Self::new_registered(god, dmabuf);
		god.wlmm.queue_request(dmabuf.borrow().wl_get_default_feedback(new.borrow().id));
		new
	}

	fn parse_format_table(&mut self, slice: &[u8]) -> Result<(), WaytinierError> {
		for chunk in slice.chunks(16) {
			let format = u32::from_wire(chunk)?;
			let _padding = u32::from_wire(&chunk[4..])?;
			let modifier = u64::from_wire(&chunk[8..])?;
			self.format_table.push((format, modifier));
		}
		Ok(())
	}
}

#[repr(u32)]
#[derive(Debug)]
pub(crate) enum TrancheFlags {
	Scanout = 1 << 0,
}

pub(crate) struct DmaParams {
	pub(crate) id: Id,
}

impl WaylandObject for DmaParams {
	fn handle(
		&mut self,
		_payload: &[u8],
		_opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		todo!()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaParams
	}
}

impl DmaParams {}
