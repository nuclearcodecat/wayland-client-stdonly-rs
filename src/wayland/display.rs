use std::{error::Error, os::fd::OwnedFd};

use crate::{
	Rl, rl,
	wayland::{
		AppRequest, Boxed, Id, IdentManager, OpCode, Raw, WaylandError, WaylandObject,
		WaylandObjectKind,
		wire::{FromWirePayload, RecvError},
	},
};

pub(crate) struct Display {
	pub(crate) id: Id,
}

impl Display {
	pub(crate) fn new(id: Id) -> Rl<Self> {
		rl!(Self {
			id,
		})
	}

	pub(crate) fn new_registered(wlim: &mut IdentManager) -> Rl<Self> {
		let display = Self::new(Id(0));
		let id = wlim.new_id_registered(display.clone());
		display.borrow_mut().id = id;
		display
	}
}

impl WaylandObject for Display {
	fn handle(
		&self,
		p: &[u8],
		opcode: OpCode,
		_fds: Vec<OwnedFd>,
	) -> Result<Vec<AppRequest>, Box<dyn Error>> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let obj_id = u32::from_wire(p)?;
				let code = u32::from_wire(&p[4..])?;
				let message = String::from_wire(&p[8..])?;
				pending.push(AppRequest::Error {
					0: RecvError {
						recv_id: self.id,
						id: Id(obj_id),
						code: OpCode(code),
						msg: message,
					}
					.boxed(),
				});
			}
			1 => {
				let deleted_id = u32::from_wire(p)?;
				pending.push(AppRequest::IdDeletion(Id(deleted_id)));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(OpCode(inv), self.kind_str()).boxed());
			}
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Display
	}
}
