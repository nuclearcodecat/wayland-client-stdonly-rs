use std::{error::Error, os::fd::OwnedFd};

use crate::{
	Rl, rl,
	wayland::{God, Id, PixelFormat, WaylandObject, WaylandObjectKind, compositor::Compositor},
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
		_god: &mut God,
		_payload: &[u8],
		_opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<(), Box<dyn Error>> {
		todo!()
	}

	fn kind(&self) -> WaylandObjectKind {
		todo!()
	}
}
