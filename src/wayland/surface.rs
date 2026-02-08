use crate::{
	Rl, rl,
	wayland::{Id, PixelFormat, WaylandObject},
};

pub(crate) struct Surface {
	pub(crate) id: Id,
	pub(crate) pf: PixelFormat,
}

impl Surface {
	pub(crate) fn new(id: Id, pf: PixelFormat) -> Rl<Self> {
		rl!(Self {
			id,
			pf,
		})
	}
}

impl WaylandObject for Surface {
	fn handle(
		&self,
		payload: &[u8],
		opcode: super::OpCode,
		_fds: Vec<std::os::unix::prelude::OwnedFd>,
	) -> Result<Vec<super::AppRequest>, Box<dyn std::error::Error>> {
		todo!()
	}

	fn kind(&self) -> super::WaylandObjectKind {
		todo!()
	}
}
