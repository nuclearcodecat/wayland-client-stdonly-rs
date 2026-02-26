use std::{
	ffi::CString,
	os::fd::{AsRawFd, FromRawFd, OwnedFd},
	str::FromStr,
};

use libc::{O_CLOEXEC, O_RDWR};

use crate::{
	BufferAccessor, Rl, dbug, rl,
	wayland::{
		God, IdentManager, WaytinierError,
		buffer::{Buffer, BufferBackend},
		dmabuf::{
			gbm::{GbmBuffer, GbmDevice, LibGbm},
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
	pub(crate) bo_ptr: Option<*mut GbmBuffer>,
	pub(crate) dev_ptr: Option<*mut GbmDevice>,
}

// todo right now the buffer backend can only hold one buffer.
//  the buffer backend is supposed to supply as many buffers
//  as requested. extend this
impl DmaBackend {
	pub(crate) fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
		backend: &Rl<BufferBackend>,
		registry: &Rl<Registry>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		dbug!("making buffer object");
		let dmabuf = DmaBuf::new_registered_bound(god, registry, surface)?;
		let feedback = DmaFeedback::new_registered_gotten(god, &dmabuf);
		let _feedback = feedback.borrow();
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
		let fd = (self.libgbm.fns.gbm_bo_get_fd)(bo_ptr);
		dbug!(format!("dmabuf fd: {fd}"));
		let fd = unsafe { OwnedFd::from_raw_fd(fd) };
		// i need to call these two \/
		(self.libgbm.fns.gbm_bo_destroy)(bo_ptr);
		(self.libgbm.fns.gbm_device_destroy)(dev_ptr);

		dbug!("buffer object made");

		Ok(Buffer::new_registered(
			god,
			(0, 0, 0),
			surface,
			backend,
			Some(BufferAccessor::DmaBufFd(fd)),
		)?)
	}

	pub(crate) fn resize(
		&mut self,
		wlmm: &mut MessageManager,
		wlim: &mut IdentManager,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaytinierError> {
		self.destroy();
		let mut buf_b = buf.borrow_mut();
		buf_b.w = w;
		buf_b.h = h;
		let id = wlim.new_id_registered(buf.clone());
		buf_b.id = id;
		Ok(())
	}

	pub(crate) fn destroy(&mut self) {
		(self.libgbm.fns.gbm_bo_destroy)(self.bo_ptr.unwrap());
		(self.libgbm.fns.gbm_device_destroy)(self.dev_ptr.unwrap());
	}

	#[allow(clippy::new_ret_no_self)]
	pub fn new() -> Result<Rl<BufferBackend>, WaytinierError> {
		Ok(rl!(BufferBackend::Dma(DmaBackend {
			dmabuf: None,
			libgbm: LibGbm::new_loaded()?,
			rendernode_fd: None,
			bo_ptr: None,
			dev_ptr: None
		})))
	}
}
