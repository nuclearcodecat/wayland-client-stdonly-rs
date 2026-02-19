use std::{
	collections::HashSet,
	ffi::{CString, c_void},
	os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
	ptr,
};

use libc::{
	MAP_FAILED, MAP_SHARED, O_CREAT, O_RDWR, PROT_READ, PROT_WRITE, ftruncate, mmap, munmap,
	shm_open, shm_unlink,
};

use crate::{
	CYAN, DebugLevel, NONE, Rl, WHITE,
	abstraction::app::App,
	dbug, handle_log, qpush, rl,
	wayland::{
		Boxed, ExpectRc, God, Id, IdentManager, OpCode, PixelFormat, Raw, WaylandObject,
		WaylandObjectKind, WaytinierError,
		buffer::{Buffer, BufferBackend},
		registry::Registry,
		surface::Surface,
		wire::{Action, FromWirePayload, MessageManager, WireArgument, WireRequest},
	},
	wlog,
};

pub struct ShmBackend {
	pub(crate) shm: Rl<SharedMemory>,
	pub(crate) pool: Rl<SharedMemoryPool>,
}

impl BufferBackend for ShmBackend {
	fn make_buffer(
		&mut self,
		god: &mut God,
		w: u32,
		h: u32,
		surface: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
		_registry: &Rl<Registry>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		let mut pool = self.pool.borrow_mut();
		let buffer = pool.make_buffer(god, (0, w, h), surface, backend)?;
		Ok(buffer)
	}

	fn resize(
		&mut self,
		wlmm: &mut MessageManager,
		wlim: &mut IdentManager,
		buf: &Rl<Buffer>,
		w: u32,
		h: u32,
	) -> Result<(), WaytinierError> {
		let id = wlim.new_id_registered(buf.clone());
		let mut buffer = buf.borrow_mut();
		wlmm.queue_request(buffer.wl_destroy());

		buffer.w = w;
		buffer.h = h;

		let mut pool = self.pool.borrow_mut();
		let format = buffer.master.upgrade().to_wl_err()?.borrow().pf;
		let shm_actions = pool.get_resize_actions_if_larger((w * h * format.width()) as i32)?;
		buffer.slice = pool.slice;
		wlmm.q.extend(shm_actions);

		buffer.id = id;

		wlmm.queue_request(pool.wl_create_buffer(
			buffer.id,
			(
				buffer.offset as i32,
				buffer.w as i32,
				buffer.h as i32,
				(buffer.w * format.width()) as i32,
			),
			format,
		));

		Ok(())
	}
}

impl ShmBackend {
	#[allow(clippy::new_ret_no_self)]
	pub fn new(app: &mut App) -> Result<Rl<Box<dyn BufferBackend>>, WaytinierError> {
		let shm = SharedMemory::new_registered_bound(&mut app.god, &app.registry)?;
		let pool = SharedMemoryPool::new_registered_allocated(&mut app.god, &shm, 8)?;
		Ok(rl!(ShmBackend {
			shm,
			pool,
		}
		.boxed() as Box<dyn BufferBackend>))
	}
}

pub struct SharedMemory {
	id: Id,
	valid_pix_formats: HashSet<PixelFormat>,
}

impl SharedMemory {
	pub(crate) fn new(id: Id) -> Rl<Self> {
		rl!(Self {
			id,
			valid_pix_formats: HashSet::new(),
		})
	}

	pub(crate) fn new_registered(god: &mut God) -> Rl<Self> {
		let new = Self::new(Id(0));
		let id = god.wlim.new_id_registered(new.clone());
		new.borrow_mut().id = id;
		new
	}

	pub(crate) fn new_registered_bound(
		god: &mut God,
		registry: &Rl<Registry>,
	) -> Result<Rl<Self>, WaytinierError> {
		let new = Self::new_registered(god);
		registry.borrow_mut().bind(god, new.borrow().id, WaylandObjectKind::SharedMemory, 1)?;
		Ok(new)
	}

	fn push_pix_format(&mut self, pf: PixelFormat) {
		self.valid_pix_formats.insert(pf);
	}

	fn wl_create_pool(&self, size: i32, fd: RawFd, id: Id) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "create_pool",
			args: vec![
				// WireArgument::NewIdSpecific(WaylandObjectKind::SharedMemoryPool.as_str(), 1, id),
				WireArgument::NewId(id),
				WireArgument::FileDescriptor(fd),
				WireArgument::Int(size),
			],
		}
	}
}

pub(crate) struct SharedMemoryPool {
	pub(crate) id: Id,
	pub(crate) name: CString,
	pub(crate) size: i32,
	pub(crate) fd: OwnedFd,
	pub(crate) slice: Option<*mut [u8]>,
	pub(crate) ptr: Option<*mut c_void>,
}

impl SharedMemoryPool {
	pub fn new(id: Id, name: CString, size: i32, fd: OwnedFd) -> Rl<Self> {
		rl!(Self {
			id,
			name,
			size,
			fd,
			slice: None,
			ptr: None,
		})
	}

	fn make_unique_pool_name() -> Result<CString, WaytinierError> {
		let mut vec = vec![];
		while vec.len() < 16 {
			let random: u8 = std::random::random(..);
			if random > b'a' && random < b'z' {
				vec.push(random);
			}
		}
		let suffix = String::from_utf8_lossy(&vec);
		let name = format!("wl-shm-{}", suffix);
		dbug!(name);
		Ok(CString::new(name)?)
	}

	pub(crate) fn new_registered(god: &mut God, name: CString, size: i32, fd: OwnedFd) -> Rl<Self> {
		let new = Self::new(Id(0), name, size, fd);
		let id = god.wlim.new_id_registered(new.clone());
		new.borrow_mut().id = id;
		new
	}

	pub(crate) fn new_registered_allocated(
		god: &mut God,
		shm: &Rl<SharedMemory>,
		size: i32,
	) -> Result<Rl<SharedMemoryPool>, WaytinierError> {
		let name = Self::make_unique_pool_name()?;
		let raw_fd = unsafe { shm_open(name.as_ptr(), O_RDWR | O_CREAT, 0) };
		if raw_fd == -1 {
			return Err(std::io::Error::last_os_error().into());
		}
		if unsafe { ftruncate(raw_fd, size.into()) } == -1 {
			return Err(std::io::Error::last_os_error().into());
		}
		let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

		let pool = Self::new_registered(god, name, size, fd);
		{
			let mut pool = pool.borrow_mut();
			wlog!(
				DebugLevel::Important,
				pool.kind_str(),
				format!("new pool fd: {}", raw_fd),
				WHITE,
				NONE
			);
			pool.update_ptr()?;
			god.wlmm.queue_request(shm.borrow().wl_create_pool(size, raw_fd, pool.id));
		}
		Ok(pool)
	}

	fn wl_create_buffer(
		&self,
		id: Id,
		(offset, width, height, stride): (i32, i32, i32, i32),
		format: PixelFormat,
	) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(0),
			opname: "create_buffer",
			args: vec![
				WireArgument::NewId(id),
				WireArgument::Int(offset),
				WireArgument::Int(width),
				WireArgument::Int(height),
				WireArgument::Int(stride),
				WireArgument::UnInt(format as u32),
			],
		}
	}

	fn wl_destroy(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(1),
			opname: "destroy",
			args: vec![],
		}
	}

	fn wl_resize(&self) -> WireRequest {
		WireRequest {
			sender_id: self.id,
			kind: self.kind(),
			opcode: OpCode(2),
			opname: "resize",
			args: vec![WireArgument::Int(self.size)],
		}
	}

	fn unmap(&self) -> Result<(), WaytinierError> {
		if let Some(ptr) = self.ptr {
			if unsafe { munmap(ptr, self.size as usize) } == 0 {
				Ok(())
			} else {
				Err(std::io::Error::last_os_error().into())
			}
		} else {
			Err(WaytinierError::ExpectedSomeValue("pointer to shm in unmap"))
		}
	}

	fn unlink(&self) -> Result<(), std::io::Error> {
		let r = unsafe { shm_unlink(self.name.as_ptr()) };
		if r == 0 {
			Ok(())
		} else {
			Err(std::io::Error::last_os_error())
		}
	}

	pub(crate) fn destroy(&self) -> Result<(), WaytinierError> {
		self.unmap()?;
		self.unlink()?;
		Ok(())
	}

	pub(crate) fn update_ptr(&mut self) -> Result<(), WaytinierError> {
		let ptr = unsafe {
			mmap(
				ptr::null_mut(),
				self.size as usize,
				PROT_READ | PROT_WRITE,
				MAP_SHARED,
				self.fd.as_raw_fd(),
				0,
			)
		};
		if ptr == MAP_FAILED {
			return Err(std::io::Error::last_os_error().into());
		}

		let x: *mut [u8] = ptr::slice_from_raw_parts_mut(ptr as *mut u8, self.size as usize);
		self.ptr = Some(ptr);
		self.slice = Some(x);
		Ok(())
	}

	pub(crate) fn get_resize_actions_if_larger(
		&mut self,
		size: i32,
	) -> Result<Vec<Action>, WaytinierError> {
		dbug!(format!("size: {size}"));
		let mut pending = vec![];
		if size < self.size {
			return Ok(pending);
		}
		handle_log!(
			pending,
			self,
			DebugLevel::Important,
			format!("{} | RESIZE size {size}", self.kind_str())
		);
		self.unmap()?;
		self.size = size;
		let r = unsafe { ftruncate(self.fd.as_raw_fd(), size.into()) };
		if r == 0 {
			Ok(())
		} else {
			Err(std::io::Error::last_os_error())
		}?;
		self.update_ptr()?;
		qpush!(pending, self.wl_resize());
		Ok(pending)
	}

	pub(crate) fn make_buffer(
		&mut self,
		god: &mut God,
		(offset, w, h): (u32, u32, u32),
		master: &Rl<Surface>,
		backend: &Rl<Box<dyn BufferBackend>>,
	) -> Result<Rl<Buffer>, WaytinierError> {
		let surface = master.borrow();
		let buf = Buffer::new_registered(god, (offset, w, h), master, backend)?;

		buf.borrow_mut().slice = self.slice;

		god.wlmm.queue_request(self.wl_create_buffer(
			buf.borrow().id,
			(offset as i32, w as i32, h as i32, (w * surface.pf.width()) as i32),
			surface.pf,
		));
		Ok(buf)
	}
}

impl WaylandObject for SharedMemory {
	fn handle(
		&mut self,
		payload: &[u8],
		opcode: super::OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		let mut pending = vec![];
		match opcode.raw() {
			0 => {
				let format = u32::from_wire(payload)?;
				if let Ok(pf) = PixelFormat::from_shm(format) {
					self.push_pix_format(pf);
					handle_log!(
						pending,
						self,
						DebugLevel::Trivial,
						format!("pushing pixel format {:?} (0x{:08x})", pf, format)
					);
				} else {
					handle_log!(
						pending,
						self,
						DebugLevel::Error,
						format!("found unrecognized pixel format 0x{:08x}", format)
					);
				}
			}
			_ => {
				return Err(WaytinierError::InvalidOpCode(opcode, self.kind()));
			}
		}
		Ok(pending)
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::SharedMemory
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}

impl WaylandObject for SharedMemoryPool {
	fn handle(
		&mut self,
		_payload: &[u8],
		_opcode: OpCode,
		_fds: &[OwnedFd],
	) -> Result<Vec<Action>, WaytinierError> {
		todo!()
	}

	fn kind(&self) -> WaylandObjectKind {
		WaylandObjectKind::SharedMemoryPool
	}

	fn kind_str(&self) -> &'static str {
		self.kind().as_str()
	}
}

impl Drop for SharedMemoryPool {
	fn drop(&mut self) {
		wlog!(DebugLevel::Important, self.kind_str(), "dropping self", WHITE, CYAN);
		if let Err(er) = self.destroy() {
			wlog!(
				DebugLevel::Error,
				self.kind_str(),
				format!("error while dropping: {er:?}"),
				WHITE,
				CYAN
			);
		};
	}
}
