use std::os::fd::OwnedFd;

use crate::{
	Rl, rl,
	wayland::{
		God, Id, Raw, WaytinierError, WaylandObject, WaylandObjectKind,
		wire::{Action, FromWirePayload},
	},
};

pub(crate) struct Callback {
	pub(crate) id: Id,
	pub(crate) done: bool,
	pub(crate) data: Option<u32>,
}

impl Callback {
	pub(crate) fn new(id: Id) -> Rl<Self> {
		rl!(Self {
			id,
			done: false,
			data: None,
		})
	}

	pub(crate) fn new_registered(god: &mut God) -> Rl<Self> {
		let cb = Self::new(Id(0));
		let id = god.wlim.new_id_registered(cb.clone());
		cb.borrow_mut().id = id;
		cb
	}
}

impl WaylandObject for Callback {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let data = u32::from_wire(payload)?;
				self.done = true;
				self.data = Some(data);
				pending.push(Action::CallbackDone(self.id, data));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Callback
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}
