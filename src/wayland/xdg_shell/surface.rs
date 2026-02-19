use std::os::fd::OwnedFd;

use crate::{
	DebugLevel, Rl, handle_log, qpush, rl,
	wayland::{
		God, Id, OpCode, Raw, WaytinierError, WaylandObject, WaylandObjectKind,
		surface::Surface,
		wire::{Action, FromWirePayload, WireArgument, WireRequest},
		xdg_shell::wm_base::XdgWmBase,
	},
};

pub(crate) struct XdgSurface {
	pub(crate) id: Id,
	pub(crate) is_configured: bool,
	pub(crate) parent: Rl<Surface>,
}

impl XdgSurface {
	pub(crate) fn new(id: Id, parent: Rl<Surface>) -> Rl<Self> {
		rl!(Self {
			id,
			is_configured: false,
			parent,
		})
	}

	pub(crate) fn new_registered(
		god: &mut God,
		wm_base: &Rl<XdgWmBase>,
		surface: &Rl<Surface>,
	) -> Rl<Self> {
		let new = Self::new(Id(0), surface.clone());
		let id = god.wlim.new_id_registered(new.clone());
		new.borrow_mut().id = id;
		wm_base.borrow().get_xdg_surface(god, surface.borrow().id, id);
		new
	}

	fn wl_get_toplevel(&self, xdg_toplevel_id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(1),
			opname: "get_toplevel",
			args: vec![WireArgument::NewId(xdg_toplevel_id)],
		}
	}

	pub(crate) fn get_toplevel(&self, god: &mut God, id: Id) {
		god.wlmm.queue_request(self.wl_get_toplevel(id));
	}

	fn wl_ack_configure(&self, serial: u32) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(4),
			opname: "ack_configure",
			args: vec![WireArgument::UnInt(serial)],
		}
	}

	fn wl_destroy(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "destroy",
			args: vec![],
		}
	}
}

impl WaylandObject for XdgSurface {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			// configure
			0 => {
				handle_log!(
					pending,
					self,
					DebugLevel::Important,
					format!("configure received, acking")
				);
				self.is_configured = true;
				let serial = u32::from_wire(payload)?;

				qpush!(pending, self.wl_ack_configure(serial));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::XdgSurface
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}
