use std::os::fd::OwnedFd;

use crate::{
	Rl, rl,
	wayland::{
		God, Id, OpCode, PixelFormat, WaylandObject, WaylandObjectKind, WaytinierError,
		buffer::Buffer,
		callback::Callback,
		compositor::Compositor,
		wire::{Action, WireArgument, WireRequest},
	},
};

pub(crate) struct Surface {
	pub(crate) id: Id,
	pub(crate) pf: PixelFormat,
	pub(crate) w: u32,
	pub(crate) h: u32,
	pub(crate) attached_buf: Option<Rl<Buffer>>,
}

impl Surface {
	pub(crate) fn new(id: Id, w: u32, h: u32, pf: PixelFormat) -> Rl<Self> {
		rl!(Self {
			w,
			h,
			id,
			pf,
			attached_buf: None,
		})
	}

	pub(crate) fn new_registered(god: &mut God, w: u32, h: u32, pf: PixelFormat) -> Rl<Self> {
		let surf = Self::new(Id(0), w, h, pf);
		let id = god.wlim.new_id_registered(surf.clone());
		surf.borrow_mut().id = id;
		surf
	}

	pub(crate) fn new_registered_made(
		god: &mut God,
		compositor: &Rl<Compositor>,
		w: u32,
		h: u32,
		pf: PixelFormat,
	) -> Rl<Self> {
		let surf = Self::new_registered(god, w, h, pf);
		compositor.borrow().create_surface(god, surf.borrow().id);
		surf
	}

	pub(crate) fn wl_attach(&self, buf_id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(1),
			opname: "attach",
			args: vec![WireArgument::Obj(buf_id), WireArgument::UnInt(0), WireArgument::UnInt(0)],
		}
	}

	pub(crate) fn attach_buffer_obj(
		&mut self,
		god: &mut God,
		to_att: Rl<Buffer>,
	) -> Result<(), WaytinierError> {
		self.attached_buf = Some(to_att.clone());
		self.attach_buffer(god)
	}

	pub(crate) fn attach_buffer(&mut self, god: &mut God) -> Result<(), WaytinierError> {
		let buf = self
			.attached_buf
			.clone()
			.ok_or(WaytinierError::ExpectedSomeValue("no buffer attached to surface"))?;
		god.wlmm.queue_request(self.wl_attach(buf.borrow().id));
		Ok(())
	}

	fn wl_commit(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(6),
			opname: "commit",
			args: vec![],
		}
	}

	pub(crate) fn commit(&self, god: &mut God) {
		god.wlmm.queue_request(self.wl_commit())
	}

	fn wl_frame(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(3),
			opname: "frame",
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn frame(&self, god: &mut God) -> Result<Rl<Callback>, WaytinierError> {
		let cb = Callback::new_registered(god);
		god.wlmm.queue_request(self.wl_frame(cb.borrow().id));
		Ok(cb)
	}

	pub fn wl_damage_buffer(&self, x: i32, y: i32, w: i32, h: i32) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(9),
			opname: "damage_buffer",
			args: vec![
				WireArgument::Int(x),
				WireArgument::Int(y),
				WireArgument::Int(w),
				WireArgument::Int(h),
			],
		}
	}

	pub(crate) fn damage_buffer(&self, god: &mut God, (x, y): (i32, i32), (w, h): (i32, i32)) {
		god.wlmm.queue_request(self.wl_damage_buffer(x, y, w, h))
	}

	pub(crate) fn repaint(&self, god: &mut God) -> Result<(), WaytinierError> {
		if self.attached_buf.is_some() {
			self.damage_buffer(god, (0, 0), (self.w as i32, self.h as i32));
			Ok(())
		} else {
			Err(WaytinierError::ExpectedSomeValue("no buffer attached to surface"))
		}
	}
}

impl WaylandObject for Surface {
	fn handle(
		&mut self,
		_payload: &[u8],
		_opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		todo!()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Surface
	}
}
