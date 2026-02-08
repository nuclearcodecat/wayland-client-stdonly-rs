use std::{
	collections::{HashMap, VecDeque},
	error::Error,
	fmt::Display,
	os::fd::OwnedFd,
};

use crate::{
	CYAN, DebugLevel, NONE, RED, Rl, YELLOW,
	wayland::wire::{MessageManager, QueueEntry},
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
		god: &mut God,
		payload: &[u8],
		opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<(), Box<dyn Error>>;
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
	InvalidOpCode(OpCode, &'static str),
	ObjectNonExistent,
	IdMapRemovalFail,
	NotInRegistry(WaylandObjectKind),
	InvalidEnumVariant(&'static str),
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
			WaylandError::ObjectNonExistent => {
				write!(f, "object does not exist in the map")
			}
			WaylandError::IdMapRemovalFail => {
				write!(f, "failed to remove object from idmap")
			}
			WaylandError::NotInRegistry(kind) => {
				write!(f, "object of kind {kind} not found in registry")
			}
			WaylandError::InvalidEnumVariant(kind) => {
				write!(f, "an invalid {kind} enum variant has been received")
			}
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

	pub(crate) fn free_id(&mut self, id: Id) -> Result<(), Box<dyn Error>> {
		let registered =
			self.idmap.iter().find(|(k, _)| **k == id.raw() as usize).map(|(k, _)| k).copied();
		if let Some(r) = registered {
			self.idmap.remove(&r).ok_or(WaylandError::IdMapRemovalFail.boxed())?;
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
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PixelFormat {
	Argb888,
	Xrgb888,
}

impl Default for PixelFormat {
	fn default() -> Self {
		Self::Argb888
	}
}

#[derive(Default)]
pub(crate) struct God {
	pub(crate) wlim: IdentManager,
	pub(crate) wlmm: MessageManager,
}

impl God {}
