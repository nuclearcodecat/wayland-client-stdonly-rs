use std::os::fd::OwnedFd;

use crate::{
	Rl, rl,
	wayland::{
		God, Id, OpCode, Raw, WaylandError, WaylandObject, WaylandObjectKind,
		registry::Registry,
		wire::{Action, WireArgument, WireRequest},
	},
};

pub(crate) struct Compositor {
	pub(crate) id: Id,
}

impl Compositor {
	pub(crate) fn new(id: Id) -> Rl<Self> {
		rl!(Self {
			id,
		})
	}

	pub(crate) fn new_registered(god: &mut God) -> Rl<Self> {
		let compositor = Self::new(Id(0));
		let id = god.wlim.new_id_registered(compositor.clone());
		compositor.borrow_mut().id = id;
		compositor
	}

	pub(crate) fn new_registered_bound(
		god: &mut God,
		registry: &Rl<Registry>,
	) -> Result<Rl<Self>, WaylandError> {
		let compositor = Self::new_registered(god);
		registry.borrow_mut().bind(god, compositor.borrow().id, compositor.borrow().kind(), 5)?;
		Ok(compositor)
	}

	fn wl_create_surface(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "create_surface",
			args: vec![WireArgument::UnInt(id.raw())],
		}
	}

	pub(crate) fn create_surface(&self, god: &mut God, id: Id) {
		god.wlmm.queue_request(self.wl_create_surface(id));
	}
}

impl WaylandObject for Compositor {
	fn handle(
		&mut self,
		_payload: &[u8],
		_opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaylandError> {
		todo!()
	}

	#[inline]
	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Compositor
	}
}
