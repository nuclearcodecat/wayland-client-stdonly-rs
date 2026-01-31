use crate::wayland::{
	EventAction, ExpectRc, God, OpCode, RcCell, RecvError, WaylandError, WaylandObject,
	WaylandObjectKind, WeRcGod,
	callback::Callback,
	registry::Registry,
	wire::{FromWirePayload, Id, QueueEntry, WireArgument, WireRequest},
};
use std::{cell::RefCell, error::Error, os::fd::RawFd, rc::Rc};

pub struct Display {
	pub id: Id,
	god: WeRcGod,
}

impl Display {
	pub fn new(god: RcCell<God>) -> Result<RcCell<Self>, Box<dyn Error>> {
		let display = Rc::new(RefCell::new(Self {
			id: 0,
			god: Rc::downgrade(&god),
		}));
		let id =
			god.borrow_mut().wlim.new_id_registered(WaylandObjectKind::Display, display.clone());
		display.borrow_mut().id = id;
		Ok(display)
	}

	pub fn make_registry(&mut self) -> Result<RcCell<Registry>, Box<dyn Error>> {
		let reg = Rc::new(RefCell::new(Registry::new_empty(0, self.god.clone())));
		let id = self
			.god
			.upgrade()
			.to_wl_err()?
			.borrow_mut()
			.wlim
			.new_id_registered(WaylandObjectKind::Registry, reg.clone());
		reg.borrow_mut().id = id;
		self.queue_request(self.wl_get_registry(id))?;
		Ok(reg)
	}

	pub(crate) fn wl_get_registry(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			opcode: 1,
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn wl_sync(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			opcode: 0,
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn sync(&self) -> Result<RcCell<Callback>, Box<dyn Error>> {
		let cb = Callback::new(self.god.clone())?;
		let id = cb.borrow().id;
		let god = self.god.upgrade().to_wl_err()?;
		let mut god = god.borrow_mut();
		god.wlmm.q.push_back(QueueEntry::Request((self.wl_sync(id), self.kind())));
		god.wlmm.q.push_back(QueueEntry::Sync(id));
		Ok(cb)
	}
}

impl WaylandObject for Display {
	fn id(&self) -> Id {
		self.id
	}

	fn god(&self) -> WeRcGod {
		self.god.clone()
	}

	fn handle(
		&mut self,
		opcode: OpCode,
		payload: &[u8],
		_fds: &[RawFd],
	) -> Result<Vec<EventAction>, Box<dyn Error>> {
		let p = payload;
		let mut pending = vec![];
		match opcode {
			0 => {
				let obj_id = u32::from_wire(p)?;
				let code = u32::from_wire(&p[4..])?;
				let message = String::from_wire(&p[8..])?;
				// maybe add some sort of error manager
				eprintln!("======== ERROR {} FIRED in wl_display\nfor object\n{:?}", code, message);
				pending.push(EventAction::Error(
					RecvError {
						id: obj_id,
						code,
						msg: message,
					}
					.boxed(),
				));
			}
			1 => {
				let deleted_id = u32::from_wire(payload)?;
				// println!(
				// 	"==================== ID {:?} GOT DELETED (unimpl)",
				// 	deleted_id
				// );
				// self.god.upgrade().to_wl_err()?.borrow_mut().wlim.free_id(deleted_id)?;
				pending.push(EventAction::IdDeletion(deleted_id));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(inv, self.kind_as_str()).boxed());
			}
		}
		Ok(pending)
	}

	#[inline]
	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::Display
	}

	#[inline]
	fn kind_as_str(&self) -> &'static str {
		self.kind().as_str()
	}
}

// both should be RcCell
#[macro_export]
macro_rules! wait_for_sync {
	($display:expr, $god: expr) => {
		let cb = $display.borrow().sync()?;
		while !cb.borrow().done {
			$god.borrow_mut().handle_events()?;
		}
	};
}
