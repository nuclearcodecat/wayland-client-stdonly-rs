use std::{cell::RefCell, error::Error, os::fd::RawFd, rc::Rc};

use crate::{
	DebugLevel, dbug,
	wayland::{
		EventAction, ExpectRc, God, OpCode, RcCell, WaylandError, WaylandObject, WaylandObjectKind,
		WeRcGod, WeakCell,
		registry::Registry,
		wire::{FromWirePayload, Id, WireArgument, WireRequest},
	},
};

pub(crate) struct DmaBuf {
	pub(crate) id: Id,
	pub(crate) god: WeakCell<God>,
}

impl DmaBuf {
	pub(crate) fn new(god: RcCell<God>) -> Self {
		Self {
			id: 0,
			god: Rc::downgrade(&god),
		}
	}

	pub(crate) fn new_bound(
		registry: RcCell<Registry>,
		god: RcCell<God>,
	) -> Result<RcCell<Self>, Box<dyn Error>> {
		let me = Rc::new(RefCell::new(Self::new(god.clone())));
		let id = god.borrow_mut().wlim.new_id_registered(WaylandObjectKind::DmaBuf, me.clone());
		me.borrow_mut().id = id;
		registry.borrow_mut().bind(id, me.borrow().kind(), 4)?;
		Ok(me)
	}

	pub(crate) fn wl_get_default_feedback(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			opcode: 2,
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn get_default_feedback(&mut self) -> Result<RcCell<DmaFeedback>, Box<dyn Error>> {
		let fb = Rc::new(RefCell::new(DmaFeedback::new()));
		let id = self
			.god
			.upgrade()
			.to_wl_err()?
			.borrow_mut()
			.wlim
			.new_id_registered(fb.borrow().kind(), fb.clone());
		fb.borrow_mut().id = id;
		dbug!(format!("{}", id));
		self.queue_request(self.wl_get_default_feedback(id))?;
		Ok(fb)
	}
}

impl DmaFeedback {
	pub(crate) fn new() -> Self {
		Self {
			id: 0,
			done: false,
		}
	}
}

impl WaylandObject for DmaBuf {
	fn id(&self) -> Id {
		self.id
	}

	fn god(&self) -> WeRcGod {
		self.god.clone()
	}

	fn handle(
		&mut self,
		opcode: OpCode,
		payload: &[u8],
		_fds: &[RawFd],
	) -> Result<Vec<EventAction>, Box<dyn Error>> {
		let mut pending = vec![];
		match opcode {
			// format
			0 => {
				dbug!(format!("format: {:?}", payload));
				pending.push(EventAction::DebugMessage(
					crate::DebugLevel::Important,
					format!("format for dmabuf: {:?}", payload),
				));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(inv as OpCode, self.kind_as_str()).boxed());
			}
		};
		Ok(pending)
	}

	fn kind_as_str(&self) -> &'static str {
		self.kind().as_str()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaBuf
	}
}

pub(crate) struct DmaFeedback {
	pub(crate) id: Id,
	pub(crate) done: bool,
}

impl WaylandObject for DmaFeedback {
	fn id(&self) -> Id {
		self.id
	}

	fn god(&self) -> WeRcGod {
		panic!("god is dead")
	}

	fn handle(
		&mut self,
		opcode: OpCode,
		payload: &[u8],
		_fds: &[RawFd],
	) -> Result<Vec<EventAction>, Box<dyn Error>> {
		let mut pending = vec![];
		match opcode {
			// done
			0 => {
				self.done = true;
			}
			// format_table
			1 => {
				let size = u32::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("size: {size}, fd: {:?}", _fds),
				));
			}
			// main_device
			2 => {
				let main_device = Vec::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("main_device: {:?}", main_device),
				));
			}
			// tranche_done
			3 => {
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					String::from("tranche done"),
				));
			}
			// tranche_target_device
			4 => {
				let target_device = Vec::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("tranche target device: {:?}", target_device),
				));
			}
			// tranche_formats
			5 => {
				let formats = Vec::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("tranche formats: {:?}", formats),
				));
			}
			// tranche_flags
			6 => {
				let flags = Vec::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("tranche flags: {:?}", flags),
				));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(inv as OpCode, self.kind_as_str()).boxed());
			}
		}
		Ok(pending)
	}

	fn kind_as_str(&self) -> &'static str {
		self.kind().as_str()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaFeedback
	}
}
