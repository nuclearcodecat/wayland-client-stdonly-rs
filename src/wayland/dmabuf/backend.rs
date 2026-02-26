use std::{
	ffi::CString,
	os::fd::{AsRawFd, FromRawFd, OwnedFd},
	str::FromStr,
};

use libc::{O_CLOEXEC, O_RDWR};

use crate::{
	BufferAccessor, CYAN, DebugLevel, Rl, WHITE, dbug, rl,
	wayland::{
		God, WaytinierError,
		buffer::{Buffer, BufferBackend},
		dmabuf::{
			gbm::{GbmBuffer, GbmDevice, LibGbm},
			objects::{DmaBuf, DmaFeedback, DmaParams},
		},
		registry::Registry,
		surface::Surface,
		wire::Action,
	},
	wlog,
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
		let dmabuf = DmaBuf::new_registered_bound(god, registry, surface)?;
		let feedback = DmaFeedback::new_registered_gotten(god, &dmabuf);
		self.dmabuf = Some(dmabuf.clone());

		while !feedback.borrow().done {
			god.handle_events()?;
		}

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
		// i need to call these two \/
		(self.libgbm.fns.gbm_bo_destroy)(bo_ptr);
		(self.libgbm.fns.gbm_device_destroy)(dev_ptr);

		let params_rc = DmaParams::new_registered_gotten(god, &dmabuf);
		god.handle_events()?;
		let stride = w * pf.width();
		let modf = {
			let fb = feedback.borrow();
			fb.format_indices
				.iter()
				.map(|ix| fb.format_table[*ix as usize])
				.find(|(fmt, _)| *fmt == pf.to_fourcc())
				.map(|(_, modf)| modf)
		};

		let params = params_rc.borrow();
		if let Some(modf) = modf {
			params.add_fd(god, fd, stride, modf);
			god.wlmm.queue(Action::Trace(
				DebugLevel::Important,
				"dma backend",
				format!("found modifier {modf}"),
			));
		} else {
			return Err(WaytinierError::ExpectedSomeValue("format modifier not present"));
		}

		params.create(god, w, h, pf);
		drop(params);
		loop {
			if let Some(id) = params_rc.borrow().new_buf_id {
				let fd = unsafe { OwnedFd::from_raw_fd(fd) };
				return Ok(Buffer::new(
					id,
					(0, 0, 0),
					surface,
					backend,
					Some(BufferAccessor::DmaBufFd(fd)),
				));
			} else {
				god.handle_events()?;
			}
		}
	}

	pub(crate) fn resize(
		&mut self,
		god: &mut God,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaytinierError> {
		self.destroy()?;
		let mut buf_b = buf.borrow_mut();
		buf_b.w = w;
		buf_b.h = h;
		let id = god.wlim.new_id_registered(buf.clone());
		buf_b.id = id;
		Ok(())
	}

	pub(crate) fn destroy(&self) -> Result<(), WaytinierError> {
		let dev_ptr = match self.dev_ptr {
			Some(p) => p,
			None => return Err(WaytinierError::ExpectedSomeValue("buffer object pointer")),
		};
		let bo_ptr = match self.bo_ptr {
			Some(p) => p,
			None => return Err(WaytinierError::ExpectedSomeValue("buffer object pointer")),
		};
		(self.libgbm.fns.gbm_bo_destroy)(bo_ptr);
		(self.libgbm.fns.gbm_device_destroy)(dev_ptr);
		Ok(())
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

impl Drop for DmaBackend {
	fn drop(&mut self) {
		wlog!(DebugLevel::Important, "dma backend", "dropping self", WHITE, CYAN);
		if let Err(er) = self.destroy() {
			wlog!(
				DebugLevel::Error,
				"dma backend",
				format!("error while dropping: {er:?}"),
				WHITE,
				CYAN
			);
		};
	}
}
