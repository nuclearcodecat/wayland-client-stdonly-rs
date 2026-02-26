use std::{
	collections::{HashMap, VecDeque},
	error::Error,
	fmt::Display,
	os::fd::OwnedFd,
	rc::Rc,
};

use crate::{
	CYAN, DebugLevel, NONE, RED, Rl, WHITE, YELLOW, dbug, get_dbug,
	wayland::wire::{Action, Consequence, MessageManager},
	wlog,
};

pub(crate) mod buffer;
pub(crate) mod callback;
pub(crate) mod compositor;
pub(crate) mod display;
pub(crate) mod dmabuf;
pub(crate) mod registry;
pub mod shm;
pub(crate) mod surface;
pub(crate) mod wire;
pub(crate) mod xdg_shell;

#[derive(Clone, Copy, Debug)]
pub struct OpCode(pub(crate) u32);
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Id(pub(crate) u32);

impl Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.raw())
	}
}

impl Display for OpCode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.raw())
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
	) -> Result<Vec<Action>, WaytinierError>;
	fn kind(&self) -> WaylandObjectKind;
	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}

#[derive(Debug)]
pub enum WaytinierError {
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
	ExpectedSomeValue(&'static str),
	ExoticOrInvalidPixelFormat,
	Dylib(libloading::Error),
	FdExpected,
	NullPtr(&'static str),
}

pub trait ExpectRc<T> {
	fn to_wl_err(self) -> Result<Rc<T>, WaytinierError>;
}

impl<T> ExpectRc<T> for Option<Rc<T>> {
	fn to_wl_err(self) -> Result<Rc<T>, WaytinierError> {
		match self {
			Some(x) => Ok(x),
			None => Err(WaytinierError::ExpectedSomeValue("Weak was empty")),
		}
	}
}

impl From<std::io::Error> for WaytinierError {
	fn from(er: std::io::Error) -> Self {
		WaytinierError::Io(er)
	}
}

impl From<std::env::VarError> for WaytinierError {
	fn from(er: std::env::VarError) -> Self {
		WaytinierError::Env(er)
	}
}

impl From<std::string::FromUtf8Error> for WaytinierError {
	fn from(er: std::string::FromUtf8Error) -> Self {
		WaytinierError::Utf8(er)
	}
}

impl From<std::ffi::NulError> for WaytinierError {
	fn from(er: std::ffi::NulError) -> Self {
		WaytinierError::Nul(er)
	}
}

impl From<libloading::Error> for WaytinierError {
	fn from(er: libloading::Error) -> Self {
		WaytinierError::Dylib(er)
	}
}

impl Error for WaytinierError {}

impl Display for WaytinierError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			WaytinierError::EmptyFromWirePayload => write!(f, "payload from wire was empty"),
			WaytinierError::RecvLenBad => write!(f, "received len of payload was bad"),
			WaytinierError::NoWaylandDisplay => {
				write!(f, "provided wayland display identifier doesn't exist")
			}
			WaytinierError::InvalidOpCode(code, name) => {
				write!(f, "invalid opcode {code} encountered for {name}")
			}
			WaytinierError::ObjectNonExistent => write!(f, "object does not exist in the map"),
			WaytinierError::IdMapRemovalFail => write!(f, "failed to remove object from idmap"),
			WaytinierError::NotInRegistry(kind) => {
				write!(f, "object of kind {kind} not found in registry")
			}
			WaytinierError::InvalidEnumVariant(kind) => {
				write!(f, "an invalid {kind} enum variant has been received")
			}
			WaytinierError::Io(er) => write!(f, "std::io::Error received: {:?}", er),
			WaytinierError::Env(er) => write!(f, "std::env::VarError received: {:?}", er),
			WaytinierError::Utf8(er) => write!(f, "std::string::FromUtf8Error received: {:?}", er),
			WaytinierError::Nul(er) => write!(f, "std::ffi::NulError received: {:?}", er),
			WaytinierError::ExpectedSomeValue(er) => write!(f, "expected a Some value: {er}"),
			WaytinierError::ExoticOrInvalidPixelFormat => {
				write!(f, "invalid pixel format encountered")
			}
			WaytinierError::Dylib(er) => write!(f, "libloading error occured: {er}"),
			WaytinierError::FdExpected => write!(f, "expected fd"),
			WaytinierError::NullPtr(er) => write!(f, "null pointer at {er}"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum WaylandObjectKind {
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
	DmaParams,
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
			WaylandObjectKind::SharedMemoryPool => "wl_shm_pool",
			WaylandObjectKind::DmaFeedback => "zwp_linux_dmabuf_feedback_v1",
			WaylandObjectKind::Callback => "wl_callback",
			WaylandObjectKind::DmaParams => "zwp_linux_buffer_params_v1",
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
				format!("new id picked from free pool: {}", id),
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

	pub(crate) fn free_id(&mut self, id: Id) -> Result<(), WaytinierError> {
		let registered =
			self.idmap.iter().find(|(k, _)| **k == id.raw() as usize).map(|(k, _)| k).copied();
		if let Some(r) = registered {
			self.idmap.remove(&r).ok_or(WaytinierError::IdMapRemovalFail)?;
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
	pub(crate) fn find_obj_by_id(&self, id: Id) -> Result<&Wlto, WaytinierError> {
		self.idmap
			.iter()
			.find(|(k, _)| **k == id.raw() as usize)
			.map(|(_, v)| v)
			.ok_or(WaytinierError::ObjectNonExistent)
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

// pub(crate) trait Boxed: Sized {
// 	fn boxed(self) -> Box<Self> {
// 		Box::new(self)
// 	}
// }

// impl<T> Boxed for T {}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum PixelFormat {
	#[default]
	Argb888,
	Xrgb888,
}

pub(crate) const fn fourcc_code(a: u8, b: u8, c: u8, d: u8) -> u32 {
	let a = a as u32;
	let b = b as u32;
	let c = c as u32;
	let d = d as u32;
	(a | b << 8) | (c << 16) | (d << 24)
}

impl PixelFormat {
	pub(crate) fn from_shm(processee: u32) -> Result<Self, WaytinierError> {
		match processee {
			0 => Ok(Self::Argb888),
			1 => Ok(Self::Xrgb888),
			_ => Self::from_u32(processee),
		}
	}

	const CC4_ARGB888: u32 = Self::Argb888.to_fourcc();
	const CC4_XRGB888: u32 = Self::Xrgb888.to_fourcc();
	pub(crate) const fn from_u32(processee: u32) -> Result<Self, WaytinierError> {
		match processee {
			x if x == Self::CC4_ARGB888 => Ok(Self::Argb888),
			x if x == Self::CC4_XRGB888 => Ok(Self::Xrgb888),
			_ => Err(WaytinierError::ExoticOrInvalidPixelFormat),
		}
	}

	pub(crate) const fn width(&self) -> u32 {
		match self {
			Self::Argb888 => 4,
			Self::Xrgb888 => 4,
		}
	}

	pub(crate) const fn to_fourcc(&self) -> u32 {
		match self {
			Self::Argb888 => fourcc_code(b'X', b'R', b'2', b'4'),
			Self::Xrgb888 => fourcc_code(b'X', b'R', b'2', b'4'),
		}
	}
}

#[derive(Default)]
pub(crate) struct God {
	pub(crate) wlim: IdentManager,
	pub(crate) wlmm: MessageManager,
}

impl God {
	pub fn handle_events(&mut self) -> Result<(), WaytinierError> {
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
		let mut last_responding_id: Option<Id> = None;
		while let Some(action) = self.wlmm.q.pop_front() {
			match action {
				Action::RequestRequest(ev) => {
					conseq.push_back(Consequence::Trace(
						DebugLevel::Trivial,
						"event handler",
						format!("going to handle {} ({})", ev.kind, ev.sender_id),
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
				Action::Error(er) => {
					conseq.push_back(Consequence::Trace(
						DebugLevel::Error,
						"error trace",
						format!("{er}"),
						RED,
						RED,
					));
				}
				Action::Trace(debug_level, kind, msg) => match debug_level {
					DebugLevel::Error => {
						conseq.push_back(Consequence::Trace(debug_level, kind, msg, WHITE, RED));
					}
					_ => {
						conseq.push_back(Consequence::Trace(debug_level, kind, msg, WHITE, NONE));
					}
				},
				Action::EventResponse(raw) => {
					let obj = self.wlim.find_obj_by_id(raw.recv_id)?;
					let actions_new = obj.borrow_mut().handle(&raw.payload, raw.opcode, &fds)?;
					self.wlmm.q.extend_front(actions_new);
					if get_dbug() > DebugLevel::Important as isize
						|| last_responding_id.is_none()
						|| last_responding_id.unwrap() != raw.recv_id
					{
						self.wlmm.q.push_front(Action::Trace(
							DebugLevel::Verbose,
							obj.borrow().kind_str(),
							format!("handling self (id {})", raw.recv_id.raw()),
						));
					}
					last_responding_id = Some(raw.recv_id);
				}
				Action::IdDeletion(id) => {
					conseq.push_back(Consequence::IdDeletion(id));
				}
				Action::Resize(w, h, surf) => {
					dbug!(format!("RESIZING {w} {h}"));
					let buf = {
						let mut surface = surf.borrow_mut();
						surface.w = w;
						surface.h = h;
						surface.attached_buf.clone()
					};
					if let Some(rcbuf) = &buf {
						let backend = {
							let buf = rcbuf.borrow();
							buf.backend.clone()
						};
						let mut backend = backend.borrow_mut();
						backend.resize(self, rcbuf, w, h)?
					} else {
						conseq.push_back(Consequence::Trace(
							DebugLevel::Important,
							"event handler",
							String::from("buffer object not attached to resized surface"),
							CYAN,
							NONE,
						));
					}
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
				}
			};
		}
		Ok(())
	}
}
