use std::error::Error;

use crate::{
	drop,
	wayland::{
		CtxType, OpCode, WaylandError, WaylandObject, WaylandObjectKind, shm::PixelFormat, wire::{Id, WireRequest}
	},
};

pub struct Buffer {
	pub id: Id,
	pub(crate) ctx: CtxType,
	pub offset: i32,
	pub width: i32,
	pub height: i32,
	pub stride: i32,
	pub format: PixelFormat,
	pub in_use: bool,
}

impl Buffer {
	pub(crate) fn wl_destroy(&self) -> Result<(), Box<dyn Error>> {
		self.ctx.borrow().wlmm.send_request(&mut WireRequest {
			sender_id: self.id,
			opcode: 0,
			args: vec![],
		})
	}

	pub fn destroy(&self) -> Result<(), Box<dyn Error>> {
		self.wl_destroy()?;
		self.ctx.borrow_mut().wlim.free_id(self.id)?;
		Ok(())
	}
}

impl WaylandObject for Buffer {
	fn handle(&mut self, opcode: super::OpCode, _payload: &[u8]) -> Result<(), Box<dyn Error>> {
		match opcode {
			// release
			0 => {
				self.in_use = false;
				Ok(())
			}
			inv => {
				Err(WaylandError::InvalidOpCode(inv as OpCode, self.as_str()).boxed())
			}
		}
	}

	fn as_str(&self) -> &'static str {
		WaylandObjectKind::Buffer.as_str()
	}
}

drop!(Buffer);
