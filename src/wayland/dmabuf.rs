// 1° kernel info (see userspace iface notes section and also has dmabuf-specific ioctl info)
// https://docs.kernel.org/driver-api/dma-buf.html
//
// 2° pixel format stuff
// https://docs.kernel.org/userspace-api/dma-buf-alloc-exchange.html
//
// 3° fourcc codes for the modifiers and formats
// https://github.com/torvalds/linux/blob/master/include/uapi/drm/drm_fourcc.h
//
// 4° FINALLY FOUND RENDER NODE INFO (MENTIONED IN WL DOCS), READ THIS LATER
// https://www.kernel.org/doc/html/v4.8/gpu/drm-uapi.html
//
// 5° PRIME
// https://www.kernel.org/doc/html/v4.13/gpu/drm-mm.html#prime-buffer-sharing
//
// 6° vk dma-buf stuff????
// https://docs.vulkan.org/refpages/latest/refpages/source/VK_EXT_external_memory_dma_buf.html
//
// 7° rust doc for the vk extension
// https://docs.rs/ash/latest/ash/ext/external_memory_dma_buf/index.html
//
// 8° read this shit
// https://blaztinn.gitlab.io/post/dmabuf-texture-sharing/
//
// 9° looks similar to the vk stuff from 6° but opengl
// https://registry.khronos.org/EGL/extensions/EXT/EGL_EXT_image_dma_buf_import.txt
//
// 10° important info from vulkan docs about formats and egl history
// https://docs.vulkan.org/refpages/latest/refpages/source/VK_EXT_image_drm_format_modifier.html
//
// 11° some gem stuff and dumb buffers info which seem simple? although
// https://man.archlinux.org/man/drm-memory.7.en
//
// 12° for dev_t dissection
// https://man.archlinux.org/man/stat.2.en
//
// === log / todo ===
// - i should probably use a render node? from 4° i couldn't figure out what the deal is with
//   primary nodes, but they seem to push render nodes as the primary system. i have a card1
//   and renderD128 (high number for some reason) device in /dev/dri and looking at
//   /dev/dri/by-path, they reference the same pci device.
// - use PRIME (?) ioctls to communicate with the render node (4° - »No ioctls except PRIME-related
//   ioctls will be allowed on this node«)
// - 5° - »To userspace PRIME buffers are dma-buf based file descriptors.«
// - i need to start using google instead of ddg face with bags under eyes emoji
// - the basic vulkan example in ash was insanely fucking long so i'm abandoning that idea
// - PFNEGLEXPORTDMABUFIMAGEQUERYMESAPROC what the fuck is that
// - 6° and 9° extensions were mentioned in 2° holy shit
// - »Many APIs in Linux use modifiers to negotiate and specify the memory layout of shared images.
//   For example, a Wayland compositor and Wayland client may, by relaying modifiers over the
//   Wayland protocol zwp_linux_dmabuf_v1, negotiate a vendor-specific tiling format for a shared
//   wl_buffer. The client may allocate the underlying memory for the wl_buffer with GBM, providing
//   the chosen modifier to gbm_bo_create_with_modifiers. The client may then import the wl_buffer
//   into Vulkan for producing image content, providing the resource’s dma_buf to
//   VkImportMemoryFdInfoKHR and its modifier to VkImageDrmFormatModifierExplicitCreateInfoEXT.
//   The compositor may then import the wl_buffer into OpenGL for sampling, providing the resource’s
//   dma_buf and modifier to eglCreateImage. The compositor may also bypass OpenGL and submit the
//   wl_buffer directly to the kernel’s display API, providing the dma_buf and modifier through
//   drm_mode_fb_cmd2«
// - the dumb buffer from 11° seems simple. it says that gpus can't access them though. i think this
//   just means that they can't write to them, so cpu stuff only. that's fine for now i guess
//

use std::{
	cell::RefCell,
	error::Error,
	ffi::CString,
	os::fd::{AsRawFd, OwnedFd},
	ptr::null_mut,
	rc::Rc,
	str::FromStr,
};

use libc::{MAP_FAILED, MAP_PRIVATE, PROT_READ};

use crate::{
	DebugLevel, NONE, WHITE,
	abstraction::dma::{DRM_FORMAT_MOD_LINEAR, fourcc_mod_code},
	dbug,
	wayland::{
		EventAction, ExpectRc, God, OpCode, RcCell, WaylandError, WaylandObject, WaylandObjectKind,
		WeRcGod, WeakCell,
		registry::Registry,
		shm::PixelFormat,
		wire::{FromWirePayload, FromWireSingle, Id, WireArgument, WireRequest},
	},
	wlog,
};

pub(crate) struct DmaBuf {
	pub(crate) id: Id,
	pub(crate) god: WeakCell<God>,
	pub(crate) preferred_format: PixelFormat,
}

impl DmaBuf {
	pub(crate) fn new(god: RcCell<God>, pf: PixelFormat) -> Self {
		Self {
			id: 0,
			god: Rc::downgrade(&god),
			preferred_format: pf,
		}
	}

	pub(crate) fn new_bound(
		registry: RcCell<Registry>,
		god: RcCell<God>,
		pf: PixelFormat,
	) -> Result<RcCell<Self>, Box<dyn Error>> {
		let me = Rc::new(RefCell::new(Self::new(god.clone(), pf)));
		let id = god.borrow_mut().wlim.new_id_registered(WaylandObjectKind::DmaBuf, me.clone());
		me.borrow_mut().id = id;
		registry.borrow_mut().bind(id, me.borrow().kind(), 5)?;
		Ok(me)
	}

	pub(crate) fn wl_get_default_feedback(&self, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			opcode: 2,
			args: vec![WireArgument::NewId(id)],
		}
	}

	pub(crate) fn get_default_feedback(&mut self) -> Result<RcCell<DmaFeedback>, Box<dyn Error>> {
		let fb = Rc::new(RefCell::new(DmaFeedback::new(self.preferred_format)));
		let id = self
			.god
			.upgrade()
			.to_wl_err()?
			.borrow_mut()
			.wlim
			.new_id_registered(fb.borrow().kind(), fb.clone());
		fb.borrow_mut().id = id;
		dbug!(format!("{}", id));
		self.queue_request(self.wl_get_default_feedback(id))?;
		Ok(fb)
	}
}

impl WaylandObject for DmaBuf {
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
		_fds: &[OwnedFd],
	) -> Result<Vec<EventAction>, Box<dyn Error>> {
		let mut pending = vec![];
		match opcode {
			// format
			0 => {
				pending.push(EventAction::DebugMessage(
					crate::DebugLevel::Important,
					format!("format for dmabuf: {:?}", payload),
				));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(inv as OpCode, self.kind_as_str()).boxed());
			}
		};
		Ok(pending)
	}

	fn kind_as_str(&self) -> &'static str {
		self.kind().as_str()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaBuf
	}
}

#[allow(dead_code)]
pub(crate) struct DmaFeedback {
	pub(crate) id: Id,
	pub(crate) done: bool,
	pub(crate) format_table: Vec<(u32, u64)>,
	pub(crate) format_indices: Vec<u16>,
	pub(crate) flags: Vec<TrancheFlags>,
	pub(crate) pf: PixelFormat,
	pub(crate) target_device: Option<u32>,
}

impl DmaFeedback {
	pub(crate) fn new(pf: PixelFormat) -> Self {
		Self {
			id: 0,
			done: false,
			format_table: vec![],
			format_indices: vec![],
			flags: vec![],
			pf,
			target_device: None,
		}
	}

	fn parse_format_table(&mut self, slice: &[u8]) -> Result<(), Box<dyn Error>> {
		for chunk in slice.chunks(16) {
			let format = u32::from_wire_element(chunk)?;
			let _padding = u32::from_wire_element(&chunk[4..])?;
			let modifier = u64::from_wire_element(&chunk[8..])?;
			self.format_table.push((format, modifier));
		}
		wlog!(
			DebugLevel::SuperVerbose,
			self.kind_as_str(),
			format!("parsed {} format table: {:?}", self.kind_as_str(), self.format_table),
			WHITE,
			NONE
		);
		Ok(())
	}
}

#[repr(u32)]
#[derive(Debug)]
pub(crate) enum TrancheFlags {
	Scanout = 1 << 0,
}

impl WaylandObject for DmaFeedback {
	fn id(&self) -> Id {
		self.id
	}

	fn god(&self) -> WeRcGod {
		panic!("god is dead")
	}

	fn handle(
		&mut self,
		opcode: OpCode,
		payload: &[u8],
		_fds: &[OwnedFd],
	) -> Result<Vec<EventAction>, Box<dyn Error>> {
		let mut pending = vec![];
		match opcode {
			// done
			0 => {
				self.done = true;
			}
			// format_table
			1 => {
				dbug!("format_table");
				let size = u32::from_wire_element(payload)? as usize;
				let fd = _fds.first().ok_or(WaylandError::FdExpected.boxed())?;
				let ptr = unsafe {
					libc::mmap(null_mut(), size, PROT_READ, MAP_PRIVATE, fd.as_raw_fd(), 0)
				};
				if ptr == MAP_FAILED {
					return Err(Box::new(std::io::Error::last_os_error()));
				}
				let slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr as *mut u8, size) };
				self.parse_format_table(slice)?;

				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("size: {size}, fd: {:?}", _fds),
				));
			}
			// main_device
			2 => {
				dbug!("main_device");
				let main_device: Vec<u32> = Vec::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("main_device: {:?}", main_device),
				));
			}
			// tranche_done
			3 => {
				dbug!("tranche_done");
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					String::from("tranche done"),
				));
			}
			// tranche_target_device
			4 => {
				dbug!("tranche_target_device");
				let target_device: Vec<u32> = Vec::from_wire(payload)?;
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("tranche target device: {:?}", target_device),
				));
				let renderd128_stat = unsafe {
					// for now assume i won't be installing a second gpu
					// i should check the dir on my laptop actually
					let mut stat_struct: libc::stat = std::mem::zeroed();
					let name_str = CString::from_str("/dev/dri/renderD128")?;
					let ret = libc::stat(name_str.as_ptr(), &mut stat_struct);
					// asuit this shit
					if ret != 0 {
						dbug!("EPIC FAIL");
					} else {
						dbug!("we good");
					}
					stat_struct
				};
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!("renderd128 stat: {:#?}", renderd128_stat),
				));
				pending.push(EventAction::DebugMessage(
					DebugLevel::Important,
					format!(
						"renderd128 stat dev: {}, td[0]: {}",
						renderd128_stat.st_rdev, target_device[0]
					),
				));
				// dev_t      st_rdev;     /* Device ID (if special file) */
				if renderd128_stat.st_rdev == target_device[0].into() {
					self.target_device = Some(target_device[0]);
					pending.push(EventAction::DebugMessage(
						DebugLevel::Important,
						String::from(
							"tranche target device matches the renderd128 stat dev number",
						),
					))
				}
			}
			// tranche_formats
			5 => {
				dbug!("tranche_formats");
				let indices: Vec<u16> = Vec::from_wire(payload)?;
				self.format_indices = indices;
				pending.push(EventAction::DebugMessage(
					DebugLevel::SuperVerbose,
					format!("tranche indices: {:?}", self.format_indices),
				));
				for ix in &self.format_indices {
					let entry = self.format_table[*ix as usize];
					pending.push(EventAction::DebugMessage(
						DebugLevel::Verbose,
						format!("tranche format {ix}: {:?}", entry),
					));
					if entry.0 == self.pf.to_fourcc() {
						pending.push(EventAction::DebugMessage(
							DebugLevel::Important,
							format!(
								"found desired pixelformat, {}: {:?}",
								DRM_FORMAT_MOD_LINEAR, entry
							),
						));
						if entry.1 == DRM_FORMAT_MOD_LINEAR {
							pending.push(EventAction::DebugMessage(
								DebugLevel::Important,
								format!("found linear modifier: {:?}", entry),
							));
						}
					}
				}
			}
			// tranche_flags
			6 => {
				dbug!("tranche_flags");
				let flags = u32::from_wire_element(payload)?;
				let mut v = vec![];
				if flags & TrancheFlags::Scanout as u32 != 0 {
					v.push(TrancheFlags::Scanout);
				};
				pending.push(EventAction::DebugMessage(
					DebugLevel::Trivial,
					format!("tranche flags: {:?}", v),
				));
			}
			inv => {
				return Err(WaylandError::InvalidOpCode(inv as OpCode, self.kind_as_str()).boxed());
			}
		}
		Ok(pending)
	}

	fn kind_as_str(&self) -> &'static str {
		self.kind().as_str()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::DmaFeedback
	}
}
