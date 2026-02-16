use crate::{Rl, wayland::WaylandObject};

pub(crate) struct DmaBuf {
	pub(crate) fbs: Vec<Rl<DmaFeedback>>,
}
pub(crate) struct DmaFeedback;

impl WaylandObject for DmaBuf {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: crate::wayland::OpCode,
		_fds: &[std::os::unix::prelude::OwnedFd],
	) -> Result<Vec<crate::wayland::wire::Action>, crate::wayland::WaylandError> {
		todo!()
	}

	fn kind(&self) -> crate::wayland::WaylandObjectKind {
		todo!()
	}
}

impl WaylandObject for DmaFeedback {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: crate::wayland::OpCode,
		_fds: &[std::os::unix::prelude::OwnedFd],
	) -> Result<Vec<crate::wayland::wire::Action>, crate::wayland::WaylandError> {
		todo!()
	}

	fn kind(&self) -> crate::wayland::WaylandObjectKind {
		todo!()
	}
}
