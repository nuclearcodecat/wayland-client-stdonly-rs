use std::os::fd::OwnedFd;

use crate::{
	Rl, qpush, rl,
	wayland::{
		God, Id, OpCode, Raw, WaytinierError, WaylandObject, WaylandObjectKind,
		registry::Registry,
		wire::{Action, FromWirePayload, WireArgument, WireRequest},
	},
};

pub(crate) struct XdgWmBase {
	pub(crate) id: Id,
}

impl XdgWmBase {
	pub(crate) fn new_registered_bound(
		registry: &Rl<Registry>,
		god: &mut God,
	) -> Result<Rl<Self>, WaytinierError> {
		let mut reg = registry.borrow_mut();
		let obj = rl!(Self {
			id: Id(0),
		});
		let id = god.wlim.new_id_registered(obj.clone());
		obj.borrow_mut().id = id;
		reg.bind(god, id, WaylandObjectKind::XdgWmBase, 1)?;
		Ok(obj)
	}

	pub fn wl_destroy(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "destroy",
			args: vec![],
		}
	}

	pub(crate) fn destroy(&self, god: &mut God) {
		god.wlmm.queue_request(self.wl_destroy())
	}

	pub fn wl_pong(&self, serial: u32) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(3),
			opname: "pong",
			args: vec![WireArgument::UnInt(serial)],
		}
	}

	fn wl_get_xdg_surface(&self, wl_surface_id: Id, xdg_surface_id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(2),
			opname: "get_xdg_surface",
			args: vec![WireArgument::NewId(xdg_surface_id), WireArgument::Obj(wl_surface_id)],
		}
	}

	pub(crate) fn get_xdg_surface(&self, god: &mut God, surface_id: Id, xdg_surface_id: Id) {
		god.wlmm.queue_request(self.wl_get_xdg_surface(surface_id, xdg_surface_id));
	}
}

impl WaylandObject for XdgWmBase {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			// ping
			0 => {
				let serial = u32::from_wire(payload)?;
				qpush!(pending, self.wl_pong(serial));
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::XdgWmBase
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}
