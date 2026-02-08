use std::os::fd::OwnedFd;

use crate::{
	Rl, rl,
	wayland::{
		Boxed, God, Id, Raw, WaylandError, WaylandObject, WaylandObjectKind,
		wire::{FromWirePayload, QueueEntry},
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
		god: &mut God,
		payload: &[u8],
		opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<(), Box<dyn std::error::Error>> {
		match opcode.raw() {
			0 => {
				let data = u32::from_wire(payload)?;
				self.done = true;
				self.data = Some(data);
				god.wlmm.queue(QueueEntry::CallbackDone(self.id, data));
				Ok(())
			}
			_ => Err(WaylandError::InvalidOpCode(opcode, self.kind_str()).boxed()),
		}
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Callback
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}
