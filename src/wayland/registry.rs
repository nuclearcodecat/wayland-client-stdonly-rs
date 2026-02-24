use std::{collections::HashMap, fmt, os::fd::OwnedFd};

use crate::{
	NONE, Rl, WHITE, handle_log, rl,
	wayland::{
		DebugLevel, God, Id, OpCode, Raw, WaylandObject, WaylandObjectKind, WaytinierError,
		display::Display,
		wire::{Action, FromWirePayload, WireArgument, WireRequest},
	},
	wlog,
};

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub(crate) struct RegistryName(pub(crate) u32);

impl Raw for RegistryName {
	fn raw(&self) -> u32 {
		self.0
	}
}

impl fmt::Display for RegistryName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.raw())
	}
}

pub(crate) struct Registry {
	pub(crate) id: Id,
	pub(crate) inner: HashMap<RegistryName, RegistryEntry>,
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct RegistryEntry {
	pub(crate) interface: String,
	pub(crate) version: u32,
}

impl Registry {
	pub(crate) fn new(id: Id) -> Rl<Self> {
		rl!(Self {
			id,
			inner: HashMap::new(),
		})
	}

	pub(crate) fn new_registered(god: &mut God) -> Rl<Self> {
		let reg = Self::new(Id(0));
		let id = god.wlim.new_id_registered(reg.clone());
		reg.borrow_mut().id = id;
		reg
	}

	pub(crate) fn new_registered_made(god: &mut God, display: &Rl<Display>) -> Rl<Self> {
		let reg = Self::new_registered(god);
		display.borrow().get_registry(god, reg.borrow().id);
		reg
	}

	fn wl_bind(
		&self,
		id: Id,
		object: RegistryName,
		name: &'static str,
		version: u32,
	) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "bind",
			args: vec![
				WireArgument::UnInt(object.raw()),
				WireArgument::NewIdSpecific(name, version, id),
			],
		}
	}

	pub(crate) fn bind(
		&mut self,
		god: &mut God,
		id: Id,
		kind: WaylandObjectKind,
		version: u32,
	) -> Result<(), WaytinierError> {
		let global_id = self
			.inner
			.iter()
			.find(|(_, v)| v.interface == kind.as_str())
			.map(|(k, _)| k)
			.copied()
			.ok_or(WaytinierError::NotInRegistry(kind))?;
		wlog!(
			DebugLevel::Important,
			self.kind_str(),
			format!("bind global id for {}: {}", kind.as_str(), global_id),
			WHITE,
			NONE
		);
		god.wlmm.queue_request(self.wl_bind(id, global_id, kind.as_str(), version));
		Ok(())
	}
}

impl WaylandObject for Registry {
	fn handle(
		&mut self,
		p: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let name = u32::from_wire(p)?;
				let interface = String::from_wire(&p[4..])?;
				let version = u32::from_wire(&p[p.len() - 4..])?;
				let msg = format!("inserted interface {interface} version {version}");
				self.inner.insert(
					RegistryName(name),
					RegistryEntry {
						interface,
						version,
					},
				);
				handle_log!(pending, self, DebugLevel::Trivial, msg);
			}
			// can global_remove even happen
			1 => {
				// let name = decode_event_payload(&p[8..], WireArgumentKind::UnInt)?;
				todo!()
			}
			_ => {
				return Err(WaytinierError::InvalidOpCode(opcode, self.kind()));
			}
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Registry
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}
