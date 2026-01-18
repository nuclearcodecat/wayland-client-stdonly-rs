use std::{cell::RefCell, error::Error, rc::Rc};

use crate::wayland::{
	CtxType, DebugLevel, EventAction, RcCell, WaylandError, WaylandObject, WaylandObjectKind,
	wire::{FromWirePayload, Id},
};

pub struct Callback {
	pub(crate) id: Id,
	pub(crate) ctx: CtxType,
	pub done: bool,
	pub data: Option<u32>,
}

impl Callback {
	pub(crate) fn new(ctx: CtxType) -> Result<RcCell<Self>, Box<dyn Error>> {
		let cb = Rc::new(RefCell::new(Self {
			id: 0,
			ctx: ctx.clone(),
			done: false,
			data: None,
		}));
		let id =
			ctx.borrow_mut().wlim.new_id_registered(super::WaylandObjectKind::Callback, cb.clone());
		cb.borrow_mut().id = id;
		Ok(cb)
	}

	pub(crate) fn destroy(&self) -> Result<(), Box<dyn Error>> {
		self.ctx.borrow_mut().wlim.free_id(self.id)
	}
}

impl WaylandObject for Callback {
	fn handle(
		&mut self,
		opcode: super::OpCode,
		payload: &[u8],
	) -> Result<Vec<EventAction>, Box<dyn std::error::Error>> {
		let mut pending = vec![];
		match opcode {
			0 => {
				let data = u32::from_wire(payload)?;
				self.done = true;
				self.data = Some(data);
				pending.push(EventAction::DebugMessage(
					DebugLevel::Verbose,
					format!("callback {} done with data {}", self.id, data),
				));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(inv, self.as_str()).boxed());
			}
		}
		Ok(pending)
	}

	fn as_str(&self) -> &'static str {
		WaylandObjectKind::Callback.as_str()
	}
}
