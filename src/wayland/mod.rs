use std::{
	collections::{HashMap, VecDeque},
	error::Error,
	fmt::Display,
	os::fd::OwnedFd,
};

use crate::{
	CYAN, DebugLevel, NONE, RED, Rl, WHITE, YELLOW,
	wayland::wire::{Action, Consequence, MessageManager},
	wlog,
};

pub(crate) mod buffer;
pub(crate) mod callback;
pub(crate) mod compositor;
pub(crate) mod display;
pub(crate) mod registry;
pub mod shm;
pub(crate) mod surface;
pub(crate) mod wire;
pub(crate) mod xdg_shell;

#[derive(Clone, Copy, Debug)]
pub(crate) struct OpCode(pub(crate) u32);
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Id(pub(crate) u32);

impl Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl Display for OpCode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

pub(crate) trait Raw {
	fn raw(&self) -> u32;
}

impl Raw for Id {
	fn raw(&self) -> u32 {
		self.0
	}
}

impl Raw for OpCode {
	fn raw(&self) -> u32 {
		self.0
	}
}

pub(crate) trait WaylandObject {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaylandError>;
	fn kind(&self) -> WaylandObjectKind;
	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}

#[derive(Debug)]
pub enum WaylandError {
	EmptyFromWirePayload,
	RecvLenBad,
	NoWaylandDisplay,
	InvalidOpCode(OpCode, WaylandObjectKind),
	ObjectNonExistent,
	IdMapRemovalFail,
	NotInRegistry(WaylandObjectKind),
	InvalidEnumVariant(&'static str),
	Io(std::io::Error),
	Env(std::env::VarError),
	Utf8(std::string::FromUtf8Error),
	Nul(std::ffi::NulError),
	RequiredValueNone(&'static str),
	InvalidPixelFormat,
}

impl From<std::io::Error> for WaylandError {
	fn from(er: std::io::Error) -> Self {
		WaylandError::Io(er)
	}
}

impl From<std::env::VarError> for WaylandError {
	fn from(er: std::env::VarError) -> Self {
		WaylandError::Env(er)
	}
}

impl From<std::string::FromUtf8Error> for WaylandError {
	fn from(er: std::string::FromUtf8Error) -> Self {
		WaylandError::Utf8(er)
	}
}

impl From<std::ffi::NulError> for WaylandError {
	fn from(er: std::ffi::NulError) -> Self {
		WaylandError::Nul(er)
	}
}

impl Error for WaylandError {}

impl Display for WaylandError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			WaylandError::EmptyFromWirePayload => write!(f, "payload from wire was empty"),
			WaylandError::RecvLenBad => write!(f, "received len of payload was bad"),
			WaylandError::NoWaylandDisplay => {
				write!(f, "provided wayland display identifier doesn't exist")
			}
			WaylandError::InvalidOpCode(code, name) => {
				write!(f, "invalid opcode {code} encountered for {name}")
			}
			WaylandError::ObjectNonExistent => write!(f, "object does not exist in the map"),
			WaylandError::IdMapRemovalFail => write!(f, "failed to remove object from idmap"),
			WaylandError::NotInRegistry(kind) => {
				write!(f, "object of kind {kind} not found in registry")
			}
			WaylandError::InvalidEnumVariant(kind) => {
				write!(f, "an invalid {kind} enum variant has been received")
			}
			WaylandError::Io(er) => write!(f, "std::io::Error received: {:?}", er),
			WaylandError::Env(er) => write!(f, "std::env::VarError received: {:?}", er),
			WaylandError::Utf8(er) => write!(f, "std::string::FromUtf8Error received: {:?}", er),
			WaylandError::Nul(er) => write!(f, "std::ffi::NulError received: {:?}", er),
			WaylandError::RequiredValueNone(er) => write!(f, "expected a Some value: {er}"),
			WaylandError::InvalidPixelFormat => write!(f, "invalid pixel format encountered"),
		}
	}
}

impl Boxed for WaylandError {}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WaylandObjectKind {
	Display,
	Registry,
	Compositor,
	Surface,
	Buffer,
	XdgWmBase,
	XdgTopLevel,
	XdgSurface,
	DmaBuf,
	SharedMemory,
	SharedMemoryPool,
	DmaFeedback,
	Callback,
}

impl Display for WaylandObjectKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl WaylandObjectKind {
	pub(crate) fn as_str(&self) -> &'static str {
		match self {
			WaylandObjectKind::Display => "wl_display",
			WaylandObjectKind::Registry => "wl_registry",
			WaylandObjectKind::Compositor => "wl_compositor",
			WaylandObjectKind::Surface => "wl_surface",
			WaylandObjectKind::Buffer => "wl_buffer",
			WaylandObjectKind::XdgWmBase => "xdg_wm_base",
			WaylandObjectKind::XdgTopLevel => "xdg_toplevel",
			WaylandObjectKind::XdgSurface => "xdg_surface",
			WaylandObjectKind::DmaBuf => "zwp_linux_dmabuf_v1",
			WaylandObjectKind::SharedMemory => "wl_shm",
			WaylandObjectKind::SharedMemoryPool => "wl_shm_poll",
			WaylandObjectKind::DmaFeedback => "zwp_linux_dmabuf_feedback_v1",
			WaylandObjectKind::Callback => "wl_callback",
		}
	}
}

pub(crate) type Wlto = Rl<dyn WaylandObject>;

#[derive(Default)]
pub(crate) struct IdentManager {
	pub(crate) idmap: HashMap<usize, Wlto>,
	pub(crate) free: VecDeque<Id>,
	pub(crate) top_id: usize,
	pub(crate) current_sync_id: Option<Id>,
}

impl IdentManager {
	pub(crate) fn new_id(&mut self) -> Id {
		self.top_id += 1;
		wlog!(DebugLevel::Trivial, "wlim", format!("new id picked: {}", self.top_id), YELLOW, NONE);
		Id(self.top_id as u32)
	}

	pub(crate) fn new_id_registered(&mut self, obj: Wlto) -> Id {
		wlog!(
			DebugLevel::Trivial,
			"wlim",
			format!("picking new id for {}", obj.borrow().kind_str()),
			YELLOW,
			NONE
		);
		let id = if let Some(id) = self.free.pop_front() {
			wlog!(
				DebugLevel::Trivial,
				"wlim",
				format!("new id picked from free pool: {}", self.top_id),
				YELLOW,
				NONE
			);
			id
		} else {
			self.new_id()
		};
		self.idmap.insert(id.raw() as usize, obj);
		id
	}

	pub(crate) fn free_id(&mut self, id: Id) -> Result<(), WaylandError> {
		let registered =
			self.idmap.iter().find(|(k, _)| **k == id.raw() as usize).map(|(k, _)| k).copied();
		if let Some(r) = registered {
			self.idmap.remove(&r).ok_or(WaylandError::IdMapRemovalFail)?;
		}
		self.free.push_back(id);
		wlog!(
			DebugLevel::Trivial,
			"wlim",
			format!("freeing id {id} | all: {:?}", self.free),
			YELLOW,
			NONE
		);
		Ok(())
	}

	// ugh
	pub(crate) fn find_obj_by_id(&self, id: Id) -> Result<&Wlto, WaylandError> {
		self.idmap
			.iter()
			.find(|(k, _)| **k == id.raw() as usize)
			.map(|(_, v)| v)
			.ok_or(WaylandError::ObjectNonExistent)
	}
}

impl Drop for IdentManager {
	fn drop(&mut self) {
		let len = self.idmap.len();
		self.idmap.clear();
		wlog!(
			DebugLevel::Important,
			"wlim",
			format!("destroying self, cleared {len} objects from the map"),
			YELLOW,
			CYAN
		);
	}
}

pub(crate) trait Boxed: Sized {
	fn boxed(self) -> Box<Self> {
		Box::new(self)
	}
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum PixelFormat {
	#[default]
	Argb888,
	Xrgb888,
}

impl PixelFormat {
	pub(crate) fn from_u32(processee: u32) -> Result<PixelFormat, WaylandError> {
		match processee {
			0 => Ok(PixelFormat::Argb888),
			1 => Ok(PixelFormat::Xrgb888),
			_ => Err(WaylandError::InvalidPixelFormat),
		}
	}

	pub const fn width(&self) -> u32 {
		match self {
			Self::Argb888 => 4,
			Self::Xrgb888 => 4,
		}
	}

	// pub const fn to_fourcc(self) -> u32 {
	// 	match self {
	// 		PixelFormat::Argb888 => fourcc_code(b'X', b'R', b'2', b'4'),
	// 		PixelFormat::Xrgb888 => fourcc_code(b'X', b'R', b'2', b'4'),
	// 	}
	// }

	// pub const fn bpp(&self) -> u32 {
	// 	match self {
	// 		PixelFormat::Argb888 => 32,
	// 		PixelFormat::Xrgb888 => 32,
	// 	}
	// }
}

#[derive(Default)]
pub(crate) struct God {
	pub(crate) wlim: IdentManager,
	pub(crate) wlmm: MessageManager,
}

impl God {
	pub fn handle_events(&mut self) -> Result<(), WaylandError> {
		wlog!(DebugLevel::Trivial, "event handler", "called", CYAN, NONE);
		let mut retries = 0;
		let fds = loop {
			let (len, fds) = self.wlmm.get_events()?;
			if len > 0 || retries > 9999 {
				break fds;
			}
			retries += 1;
		};
		let mut conseq: VecDeque<Consequence> = VecDeque::new();
		while let Some(action) = self.wlmm.q.pop_front() {
			match action {
				Action::RequestRequest(ev) => {
					conseq.push_back(Consequence::Trace(
						DebugLevel::Trivial,
						"event handler",
						format!("going to handle {:?} ({})", ev.kind, ev.sender_id),
						CYAN,
						NONE,
					));
					conseq.push_back(Consequence::Request(ev));
				}
				Action::Sync(id) => {
					self.wlim.current_sync_id = Some(id);
				}
				Action::CallbackDone(id, data) => {
					wlog!(
						DebugLevel::Trivial,
						"event handler",
						format!("callback {} done with data {}", id, data),
						CYAN,
						NONE
					);
					if let Some(sid) = self.wlim.current_sync_id
						&& sid == id
					{
						wlog!(
							DebugLevel::Trivial,
							"event handler",
							"sync callback done",
							CYAN,
							NONE
						);
						self.wlim.current_sync_id = None;
						break;
					}
				}
				// this needs structs
				Action::Error(id, id_, opcode, x) => {
					conseq.push_back(Consequence::Trace(
						DebugLevel::Error,
						"error trace",
						format!("{id} {id_} {opcode} {x}"),
						RED,
						RED,
					));
				}
				Action::Trace(debug_level, kind, msg) => {
					conseq.push_back(Consequence::Trace(debug_level, kind, msg, WHITE, NONE));
				}
				Action::EventResponse(raw) => {
					let obj = self.wlim.find_obj_by_id(raw.recv_id)?;
					let actions_new = obj.borrow_mut().handle(&raw.payload, raw.opcode, &fds)?;
					self.wlmm.q.extend_front(actions_new);
					self.wlmm.q.push_front(Action::Trace(
						DebugLevel::Verbose,
						obj.borrow().kind_str(),
						format!("handling self (id {})", raw.recv_id.raw()),
					));
				}
				Action::IdDeletion(id) => {
					conseq.push_back(Consequence::IdDeletion(id));
				}
			}
		}
		while let Some(c) = conseq.pop_front() {
			match c {
				Consequence::Request(mut msg) => {
					// self.wlmm.send_request_logged(&mut msg, Some(id), Some(kind), None)?;
					self.wlmm.send_request_logged(&mut msg)?;
				}
				Consequence::IdDeletion(id) => {
					wlog!(
						DebugLevel::Trivial,
						"event handler",
						format!("id {id} deleted internally"),
						CYAN,
						NONE
					);
					self.wlim.free_id(id)?;
				}
				Consequence::Trace(dl, kind, msg, bg, fg) => {
					wlog!(dl, kind, msg, bg, fg)
				} // Consequence::Resize(w, h, xdgs) => {
				  // 	let xdgs = xdgs.upgrade().to_wl_err()?;
				  // 	let xdgs = xdgs.borrow_mut();
				  // 	let att_buf =
				  // 		xdgs.wl_surface.upgrade().to_wl_err()?.borrow().attached_buf.clone();

				  // 	if let Some(buf_) = att_buf {
				  // 		let mut buf = buf_.borrow_mut();
				  // 		wlog!(
				  // 			DebugLevel::Important,
				  // 			"event handler",
				  // 			format!("calling resize, w: {}, h: {}", w, h),
				  // 			CYAN,
				  // 			NONE
				  // 		);
				  // 		let new_buf_id =
				  // 			self.wlim.new_id_registered(WaylandObjectKind::Buffer, buf_.clone());
				  // 		let acts = buf.get_resize_actions(new_buf_id, (w, h))?;
				  // 		conseq.extend_front(acts);
				  // 	} else {
				  // 		wlog!(
				  // 			DebugLevel::Important,
				  // 			"event handler",
				  // 			"buf not present at resize",
				  // 			CYAN,
				  // 			YELLOW
				  // 		);
				  // 	}

				  // 	let surf = xdgs.wl_surface.upgrade().to_wl_err()?;
				  // 	let mut surf = surf.borrow_mut();
				  // 	surf.w = w;
				  // 	surf.h = h;
				  // }
				  // Consequence::DropObject(id) => {
				  // 	self.wlim.idmap.remove(&id);
				  // }
			};
		}
		Ok(())
	}
}
