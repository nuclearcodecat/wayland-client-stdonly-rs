use std::os::fd::OwnedFd;

use crate::{
	Rl, rl,
	wayland::{
		God, Id, PixelFormat, WaylandError, WaylandObject, WaylandObjectKind,
		compositor::Compositor, wire::Action,
	},
};

pub(crate) struct Surface {
	pub(crate) id: Id,
	pub(crate) pf: PixelFormat,
}

impl Surface {
	pub(crate) fn new(id: Id, pf: PixelFormat) -> Rl<Self> {
		rl!(Self {
			id,
			pf,
		})
	}

	pub(crate) fn new_registered(god: &mut God, pf: PixelFormat) -> Rl<Self> {
		let surf = Self::new(Id(0), pf);
		let id = god.wlim.new_id_registered(surf.clone());
		surf.borrow_mut().id = id;
		surf
	}

	pub(crate) fn new_registered_made(
		god: &mut God,
		compositor: &Rl<Compositor>,
		pf: PixelFormat,
	) -> Rl<Self> {
		let surf = Self::new_registered(god, pf);
		compositor.borrow().create_surface(god, surf.borrow().id);
		surf
	}
}

impl WaylandObject for Surface {
	fn handle(
		&mut self,
		_payload: &[u8],
		_opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaylandError> {
		todo!()
	}

	fn kind(&self) -> WaylandObjectKind {
		todo!()
	}
}
