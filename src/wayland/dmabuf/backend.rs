use std::{
	ffi::CString,
	os::fd::{AsRawFd, FromRawFd, OwnedFd},
	str::FromStr,
};

use libc::{O_CLOEXEC, O_RDWR};

use crate::{
	App, Rl, dbug, rl,
	wayland::{
		Boxed, God, Id, IdentManager, WaytinierError,
		buffer::{Buffer, BufferBackend},
		dmabuf::{
			gbm::LibGbm,
			objects::{DmaBuf, DmaFeedback},
		},
		registry::Registry,
		surface::Surface,
		wire::MessageManager,
	},
};

pub struct DmaBackend {
	pub(crate) dmabuf: Option<Rl<DmaBuf>>,
	pub(crate) libgbm: LibGbm,
	pub(crate) rendernode_fd: Option<OwnedFd>,
}

impl BufferBackend for DmaBackend {
	fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
		registry: &Rl<Registry>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		dbug!("making buffer object");
		let dmabuf = DmaBuf::new_registered_bound(god, registry, surface)?;
		let feedback = DmaFeedback::new_registered_gotten(god, &dmabuf);
		let feedback = feedback.borrow();
		self.dmabuf = Some(dmabuf);

		// assuming renderD128
		// feedback.target_device
		let cardpath = CString::from_str("/dev/dri/renderD128")?;
		let fd = unsafe { libc::open(cardpath.as_ptr(), O_CLOEXEC | O_RDWR) };
		if fd == -1 {
			return Err(std::io::Error::last_os_error().into());
		}
		let fd = unsafe { OwnedFd::from_raw_fd(fd) };
		self.rendernode_fd = Some(fd);
		let pf = surface.borrow().pf;
		let dev_ptr =
			(self.libgbm.fns.gbm_create_device)(self.rendernode_fd.as_ref().unwrap().as_raw_fd());
		if dev_ptr.is_null() {
			return Err(WaytinierError::NullPtr("gbm_create_device"));
		}
		let bo_ptr = (self.libgbm.fns.gbm_bo_create)(dev_ptr, w, h, pf.to_fourcc(), 0);
		if bo_ptr.is_null() {
			return Err(WaytinierError::NullPtr("gbm_bo_create"));
		}
		// get an fd to it... prime stuff iirc
		(self.libgbm.fns.gbm_bo_destroy)(bo_ptr);
		(self.libgbm.fns.gbm_device_destroy)(dev_ptr);

		dbug!("buffer object made");

		// https://xeechou.net/posts/drm-backend-ii/
		Ok(Buffer::new(Id(0), (0, 0, 0), surface, backend))
	}

	fn resize(
		&mut self,
		wlmm: &mut MessageManager,
		wlim: &mut IdentManager,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaytinierError> {
		todo!()
	}
}

impl DmaBackend {
	#[allow(clippy::new_ret_no_self)]
	pub fn new(app: &mut App) -> Result<Rl<Box<dyn BufferBackend>>, WaytinierError> {
		Ok(rl!(DmaBackend {
			dmabuf: None,
			libgbm: LibGbm::new_loaded()?,
			rendernode_fd: None,
		}
		.boxed() as Box<dyn BufferBackend>))
	}
}
