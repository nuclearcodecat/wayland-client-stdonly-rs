use std::{
	collections::HashMap,
	os::fd::{AsRawFd, OwnedFd, RawFd},
	ptr::null_mut,
	rc::Rc,
};

use libc::{MAP_FAILED, MAP_PRIVATE, PROT_READ};

use crate::{
	DebugLevel, PixelFormat, Rl, Wl, handle_log, rl,
	wayland::{
		ExpectRc, God, Id, OpCode, Raw, WaylandObject, WaylandObjectKind, WaytinierError,
		registry::Registry,
		surface::Surface,
		wire::{Action, FromWirePayload, WireArgument, WireRequest},
	},
};

pub(crate) struct DmaBuf {
	pub(crate) id: Id,
	// format: modifier
	pub(crate) formats: HashMap<u32, Option<u64>>,
	pub(crate) surface: Rl<Surface>,
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
			// format
			0 => {
				let format = u32::from_wire(payload)?;
				match PixelFormat::from_u32(format) {
					Ok(f) => {
						self.formats.insert(format, None);
						pending.push(Action::Trace(
							DebugLevel::Trivial,
							self.kind_str(),
							format!("found format for dmabuf: 0x{:08x} ({:?})", format, f),
						));
					}
					Err(_) => {
						pending.push(Action::Trace(
							DebugLevel::Error,
							self.kind_str(),
							format!("found unrecognized pixel format: 0x{:08x}", format),
						));
					}
				};
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
			surface: surface.clone(),
			formats: HashMap::new(),
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

	fn wl_create_params(&self, new_id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(1),
			opname: "create_params",
			args: vec![WireArgument::NewId(new_id)],
		}
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
				let pf = self.parent.upgrade().to_wl_err()?.borrow().surface.borrow().pf;
				for ix in &self.format_indices {
					let entry = self.format_table[*ix as usize];
					pending.push(Action::Trace(
						DebugLevel::SuperVerbose,
						self.kind_str(),
						format!("tranche format {ix}: {:?}", entry),
					));
					if entry.0 == pf.to_fourcc() {
						pending.push(Action::Trace(
							DebugLevel::Important,
							self.kind_str(),
							format!("found desired pixelformat, {}: {:?}", entry.0, pf),
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
	pub(crate) new_buf_id: Option<Id>,
}

impl WaylandObject for DmaParams {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			// created
			0 => {
				let id = Id(u32::from_wire(payload)?);
				self.new_buf_id = Some(id);
				handle_log!(pending, self, DebugLevel::Important, format!("created, {id}"));
			}
			// failed
			1 => {
				handle_log!(pending, self, DebugLevel::Important, String::from("failed"));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaParams
	}
}

impl DmaParams {
	pub(crate) fn new(id: Id) -> Rl<DmaParams> {
		rl!(DmaParams {
			id,
			new_buf_id: None,
		})
	}

	pub(crate) fn new_registered(god: &mut God) -> Rl<Self> {
		let new = Self::new(Id(0));
		let id = god.wlim.new_id_registered(new.clone());
		new.borrow_mut().id = id;
		new
	}

	pub(crate) fn new_registered_gotten(god: &mut God, dmabuf: &Rl<DmaBuf>) -> Rl<Self> {
		let new = Self::new_registered(god);
		god.wlmm.queue_request(dmabuf.borrow().wl_create_params(new.borrow().id));
		new
	}

	// todo add some sort of on_destroy to Wlto which will return the reqs
	fn wl_destroy(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "destroy",
			args: vec![],
		}
	}

	fn wl_add(
		&self,
		fd: RawFd,
		plane_id: u32,
		offset: u32,
		stride: u32,
		modifier_hi: u32,
		modifier_lo: u32,
	) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(1),
			opname: "add",
			args: vec![
				WireArgument::FileDescriptor(fd),
				WireArgument::UnInt(plane_id),
				WireArgument::UnInt(offset),
				WireArgument::UnInt(stride),
				WireArgument::UnInt(modifier_hi),
				WireArgument::UnInt(modifier_lo),
			],
		}
	}

	pub(crate) fn add_fd(&self, god: &mut God, fd: RawFd, stride: u32, modf: u64) {
		let mod_hi = (modf >> 32) as u32;
		let mod_lo = modf as u32;
		god.wlmm.queue_request(self.wl_add(fd, 0, 0, stride, mod_hi, mod_lo));
	}

	fn wl_create(&self, w: i32, h: i32, format: u32, flags: u32) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(2),
			opname: "create",
			args: vec![
				WireArgument::Int(w),
				WireArgument::Int(h),
				WireArgument::UnInt(format),
				WireArgument::UnInt(flags),
			],
		}
	}

	pub(crate) fn create(&self, god: &mut God, w: u32, h: u32, format: PixelFormat) {
		god.wlmm.queue_request(self.wl_create(w as i32, h as i32, format.to_fourcc(), 0))
	}
}
