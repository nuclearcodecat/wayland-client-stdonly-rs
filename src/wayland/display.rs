use std::{error::Error, os::fd::OwnedFd};

use crate::{
	Rl, rl,
	wayland::{
		God, Id, OpCode, Raw, WaytinierError, WaylandObject, WaylandObjectKind,
		callback::Callback,
		registry::Registry,
		wire::{Action, FromWirePayload, RecvError, WireArgument, WireRequest},
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
			opname: "get_registry",
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
			opname: "sync",
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn sync(&self, god: &mut God) -> Result<Rl<Callback>, WaytinierError> {
		let cb = Callback::new_registered(god);
		let id = cb.borrow().id;
		god.wlmm.queue(Action::RequestRequest(self.wl_sync(id)));
		god.wlmm.queue(Action::Sync(id));
		Ok(cb)
	}
}

impl WaylandObject for Display {
	fn handle(
		&mut self,
		p: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let obj_id = u32::from_wire(p)?;
				let code = u32::from_wire(&p[4..])?;
				let message = String::from_wire(&p[8..])?;
				pending.push(Action::Error(RecvError {
					recv_id: self.id,
					id: Id(obj_id),
					code: OpCode(code),
					msg: message,
				}));
			}
			1 => {
				let deleted_id = u32::from_wire(p)?;
				pending.push(Action::IdDeletion(Id(deleted_id)));
			}
			inv => {
				return Err(WaytinierError::InvalidOpCode(OpCode(inv), self.kind()));
			}
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Display
	}
}

#[macro_export]
macro_rules! wait_for_sync {
	($display:expr, $god: expr) => {
		let cb = $display.borrow().sync($god)?;
		while !cb.borrow().done {
			$god.handle_events()?;
		}
	};
}
