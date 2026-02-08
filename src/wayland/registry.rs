use std::{collections::HashMap, error::Error, fmt::Display, os::fd::OwnedFd};

use crate::{
	NONE, Rl, WHITE, rl,
	wayland::{
		AppRequest, Boxed, DebugLevel, Id, IdentManager, OpCode, Raw, Request, WaylandError,
		WaylandObject, WaylandObjectKind,
		wire::{FromWirePayload, WireArgument, WireRequest},
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

impl Display for RegistryName {
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

	pub(crate) fn new_registered(wlim: &mut IdentManager) -> Rl<Self> {
		let reg = Self::new(Id(0));
		let id = wlim.new_id_registered(reg.clone());
		reg.borrow_mut().id = id;
		reg
	}

	fn wl_bind(&self, id: Id, object: u32, name: &'static str, version: u32) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			opcode: OpCode(0),
			args: vec![
				WireArgument::UnInt(object),
				WireArgument::NewIdSpecific(name, version, id.raw()),
			],
		}
	}

	pub(crate) fn bind(
		&mut self,
		id: Id,
		kind: WaylandObjectKind,
		version: u32,
	) -> Result<Vec<AppRequest>, Box<dyn Error>> {
		let global_id = self
			.inner
			.iter()
			.find(|(_, v)| v.interface == kind.as_str())
			.map(|(k, _)| k)
			.copied()
			.ok_or(WaylandError::NotInRegistry(kind))?;
		wlog!(
			DebugLevel::Important,
			self.kind_str(),
			format!("bind global id for {}: {}", kind.as_str(), global_id),
			WHITE,
			NONE
		);
		Ok(vec![AppRequest::Request(Request {
			inner: self.wl_bind(id, global_id.raw(), kind.as_str(), version),
			opname: "bind",
			kind: kind.as_str(),
		})])
	}

	pub fn does_implement(&self, query: &str) -> Option<u32> {
		self.inner.iter().find(|(_, v)| v.interface == query).map(|(_, v)| v.version)
	}
}

impl WaylandObject for Registry {
	fn handle(
		&self,
		p: &[u8],
		opcode: OpCode,
		_fds: Vec<OwnedFd>,
	) -> Result<Vec<AppRequest>, Box<dyn Error>> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let name = u32::from_wire(p)?;
				let interface = String::from_wire(&p[4..])?;
				let version = u32::from_wire(&p[p.len() - 4..])?;
				let msg = format!("inserted interface {} version {}", interface, version);
				pending.push(AppRequest::RegistryPush(
					RegistryName(name),
					RegistryEntry {
						interface,
						version,
					},
				));
				pending.push(AppRequest::DebugMessage(DebugLevel::Trivial, msg));
			}
			// can global_remove even happen
			1 => {
				// let name = decode_event_payload(&p[8..], WireArgumentKind::UnInt)?;
				todo!()
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(OpCode(inv), self.kind_str()).boxed());
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
