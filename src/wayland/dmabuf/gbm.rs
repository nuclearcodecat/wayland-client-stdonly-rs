use std::os::fd::RawFd;

use crate::{dbug, wayland::WaytinierError};

// the defs are in /gbm directory that i copied from the mesa src directory
// it is .gitignored though
pub(crate) struct LibGbmFunctions {
	// pass the render node fd here and get a gbm_device pointer
	pub(crate) gbm_create_device: fn(RawFd) -> *mut GbmDevice,
	// pass the gbm device here to get a buffer pointer
	// the only flag in wayland is the scanout flag
	pub(crate) gbm_bo_create: fn(*mut GbmDevice, u32, u32, u32, u32) -> *mut GbmBuffer,
	pub(crate) gbm_bo_destroy: fn(*mut GbmBuffer),
	pub(crate) gbm_device_destroy: fn(*mut GbmDevice),
}

pub(crate) struct LibGbm {
	pub(crate) lib: libloading::Library,
	pub(crate) fns: LibGbmFunctions,
}

impl LibGbm {
	pub(crate) fn new_loaded() -> Result<Self, WaytinierError> {
		let lib = unsafe { libloading::Library::new("libgbm.so")? };
		let new = Self {
			fns: LibGbmFunctions {
				gbm_create_device: unsafe { *lib.get("gbm_create_device")? },
				gbm_bo_create: unsafe { *lib.get("gbm_bo_create")? },
				gbm_bo_destroy: unsafe { *lib.get("gbm_bo_destroy")? },
				gbm_device_destroy: unsafe { *lib.get("gbm_device_destroy")? },
			},
			lib,
		};
		dbug!("loaded!!");
		Ok(new)
	}
}

pub(crate) struct GbmDevice;
pub(crate) struct GbmBuffer;
