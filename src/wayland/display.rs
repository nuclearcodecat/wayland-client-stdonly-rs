use std::{error::Error, os::fd::OwnedFd};

use crate::{
	Rl, rl,
	wayland::{
		Boxed, God, Id, OpCode, Raw, WaylandError, WaylandObject, WaylandObjectKind,
		callback::Callback,
		registry::Registry,
		wire::{FromWirePayload, QueueEntry, WireArgument, WireRequest},
	},
};

pub(crate) struct Display {
	pub(crate) id: Id,
}

impl Display {
	pub(crate) fn new(id: Id) -> Rl<Self> {
		rl!(Self {
			id,
		})
	}

	pub(crate) fn new_registered(god: &mut God) -> Rl<Self> {
		let display = Self::new(Id(0));
		let id = god.wlim.new_id_registered(display.clone());
		display.borrow_mut().id = id;
		display
	}

	fn wl_get_registry(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(1),
			opname: Some("get_registry"),
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn get_registry(&self, god: &mut God, id: Id) {
		god.wlmm.queue_request(self.wl_get_registry(id));
	}

	pub(crate) fn make_registry(&mut self, god: &mut God) -> Result<Rl<Registry>, Box<dyn Error>> {
		let reg = Registry::new(Id(0));
		let id = god.wlim.new_id_registered(reg.clone());
		reg.borrow_mut().id = id;
		self.get_registry(god, id);
		Ok(reg)
	}

	pub(crate) fn wl_sync(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: Some("sync"),
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn sync(&self, god: &mut God) -> Result<Rl<Callback>, Box<dyn Error>> {
		let cb = Callback::new_registered(god);
		let id = cb.borrow().id;
		god.wlmm.queue(QueueEntry::Request(self.wl_sync(id)));
		god.wlmm.queue(QueueEntry::Sync(id));
		Ok(cb)
	}
}

impl WaylandObject for Display {
	fn handle(
		&mut self,
		god: &mut God,
		p: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<(), Box<dyn Error>> {
		match opcode.raw() {
			0 => {
				let obj_id = u32::from_wire(p)?;
				let code = u32::from_wire(&p[4..])?;
				let message = String::from_wire(&p[8..])?;
				god.wlmm.queue(QueueEntry::Error(self.id, Id(obj_id), OpCode(code), message));
			}
			1 => {
				let deleted_id = u32::from_wire(p)?;
				god.wlmm.queue(QueueEntry::IdDeletion(Id(deleted_id)));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(OpCode(inv), self.kind_str()).boxed());
			}
		}
		Ok(())
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Display
	}
}

#[macro_export]
macro_rules! wait_for_sync {
	($display:expr, $god: expr) => {
		let cb = $display.borrow().sync()?;
		while !cb.borrow().done {
			$god.handle_events()?;
		}
	};
}
