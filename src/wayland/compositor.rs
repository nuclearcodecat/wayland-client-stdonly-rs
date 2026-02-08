use std::{error::Error, os::fd::OwnedFd};

use crate::{
	Rl, rl,
	wayland::{
		AppRequest, Id, IdentManager, OpCode, PixelFormat, Raw, Request, WaylandObject,
		WaylandObjectKind,
		registry::Registry,
		surface::Surface,
		wire::{WireArgument, WireRequest},
	},
};

pub(crate) struct Compositor {
	pub(crate) id: Id,
}

impl Compositor {
	pub(crate) fn new(id: Id) -> Self {
		Self {
			id,
		}
	}

	pub fn new_bound(
		wlim: &mut IdentManager,
		registry: Rl<Registry>,
	) -> Result<Rl<Self>, Box<dyn Error>> {
		let compositor = rl!(Self::new(Id(0)));
		let id = wlim.new_id_registered(compositor.clone());
		compositor.borrow_mut().id = id;
		registry.borrow_mut().bind(id, WaylandObjectKind::Compositor, 5)?;
		Ok(compositor)
	}

	fn wl_create_surface(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			opcode: OpCode(0),
			args: vec![WireArgument::UnInt(id.raw())],
		}
	}

	pub fn make_surface(
		&self,
		wlim: &mut IdentManager,
	) -> Result<(Vec<AppRequest>, Rl<Surface>), Box<dyn Error>> {
		// TODO allow choice by user
		let surface = Surface::new(Id(0), PixelFormat::Argb888);
		let id = wlim.new_id_registered(surface.clone());
		surface.borrow_mut().id = id;

		Ok((
			vec![AppRequest::Request(Request {
				inner: self.wl_create_surface(id),
				opname: "create_surface",
				kind: self.kind_str(),
			})],
			surface,
		))
	}
}

impl WaylandObject for Compositor {
	fn handle(
		&self,
		_payload: &[u8],
		_opcode: super::OpCode,
		_fds: Vec<OwnedFd>,
	) -> Result<Vec<AppRequest>, Box<dyn Error>> {
		todo!()
	}

	#[inline]
	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Compositor
	}
}
