use std::os::fd::OwnedFd;

use crate::{
	DebugLevel, Rl, handle_log, rl,
	wayland::{
		God, Id, OpCode, Raw, WaylandObject, WaylandObjectKind, WaytinierError,
		wire::{Action, FromWirePayload, WireArgument, WireRequest},
		xdg_shell::surface::XdgSurface,
	},
};

pub struct XdgTopLevel {
	pub(crate) id: Id,
	pub(crate) close_requested: bool,
	pub(crate) parent: Rl<XdgSurface>,
}

impl XdgTopLevel {
	// pub fn new_from_xdg_surface(
	// 	xdg_surface: RcCell<XdgSurface>,
	// 	god: RcCell<God>,
	// ) -> Result<RcCell<Self>, Box<dyn Error>> {
	// 	let xdgtl = Rc::new(RefCell::new(Self {
	// 		id: 0,
	// 		god: Rc::downgrade(&god),
	// 		parent: Rc::downgrade(&xdg_surface),
	// 		title: None,
	// 		appid: None,
	// 		close_requested: false,
	// 	}));
	// 	let mut god = god.borrow_mut();
	// 	let id = god.wlim.new_id_registered(WaylandObjectKind::XdgTopLevel, xdgtl.clone());
	// 	{
	// 		let mut tl_borrow = xdgtl.borrow_mut();
	// 		god.wlmm.queue_request(xdg_surface.borrow().wl_get_toplevel(id), tl_borrow.kind());
	// 		tl_borrow.id = id;
	// 	}
	// 	Ok(xdgtl)
	// }

	pub(crate) fn new(id: Id, parent: Rl<XdgSurface>) -> Rl<Self> {
		rl!(Self {
			id,
			close_requested: false,
			parent,
		})
	}

	pub(crate) fn new_registered(god: &mut God, parent: &Rl<XdgSurface>) -> Rl<Self> {
		let tl = Self::new(Id(0), parent.clone());
		let id = god.wlim.new_id_registered(tl.clone());
		tl.borrow_mut().id = id;
		tl
	}

	pub(crate) fn new_registered_gotten(god: &mut God, xdg_surface: &Rl<XdgSurface>) -> Rl<Self> {
		let tl = Self::new_registered(god, xdg_surface);
		xdg_surface.borrow().get_toplevel(god, tl.borrow().id);
		tl
	}

	pub(crate) fn wl_set_app_id(&self, id: &str) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(3),
			opname: "set_app_id",
			args: vec![WireArgument::String(String::from(id))],
		}
	}

	pub(crate) fn set_app_id(&mut self, god: &mut God, id: &str) {
		// self.appid = Some(id.to_string());
		god.wlmm.queue_request(self.wl_set_app_id(id));
	}

	fn wl_set_title(&self, id: &str) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(2),
			opname: "set_title",
			args: vec![WireArgument::String(String::from(id))],
		}
	}

	pub(crate) fn set_title(&mut self, god: &mut God, id: &str) {
		// self.title = Some(id.to_string());
		god.wlmm.queue_request(self.wl_set_title(id))
	}

	// fn wl_destroy(&self) -> WireRequest {
	// 	WireRequest {
	// 		sender_id: self.id,
	// 		kind: self.kind(),
	// 		opcode: OpCode(0),
	// 		opname: "destroy",
	// 		args: vec![],
	// 	}
	// }
}

#[allow(dead_code)]
#[repr(u32)]
#[derive(Debug)]
enum XdgTopLevelStates {
	Maximized = 1,
	Fullscreen,
	Resizing,
	Activated,
	TiledLeft,
	TiledRight,
	TiledTop,
	TiledBottom,
	Suspended,
	ConstrainedLeft,
	ConstrainedRight,
	ConstrainedTop,
	ConstrainedBottom,
}

impl WaylandObject for XdgTopLevel {
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
				let w = i32::from_wire(payload)? as u32;
				let h = i32::from_wire(&payload[4..])? as u32;
				let states: Vec<XdgTopLevelStates> = Vec::from_wire(&payload[8..])?
					.iter()
					.map(|en| {
						if (*en as usize) < std::mem::variant_count::<XdgTopLevelStates>() {
							Ok(unsafe { std::mem::transmute::<u32, XdgTopLevelStates>(*en) })
						} else {
							// maybe try formatting this automatically
							Err(WaytinierError::InvalidEnumVariant("XdgTopLevelStates"))
						}
					})
					.collect::<Result<Vec<_>, _>>()?;
				handle_log!(
					pending,
					self,
					DebugLevel::Important,
					format!("configure // w: {w}, h: {h}, states: {states:?}")
				);
				if w != 0 && h != 0 {
					pending.push(Action::Resize(w, h, self.parent.borrow().parent.clone()));
				}
			}
			// close
			1 => {
				self.close_requested = true;
				handle_log!(pending, self, DebugLevel::Important, String::from("close requested"));
			}
			// configure_bounds
			2 => {
				todo!()
			}
			// wm_capabilities
			3 => {
				todo!()
			}
			_ => return Err(WaytinierError::InvalidOpCode(opcode, self.kind())),
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::XdgTopLevel
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}
