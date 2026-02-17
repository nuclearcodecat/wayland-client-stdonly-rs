use std::{
	collections::VecDeque,
	env,
	error::Error,
	fmt::{self, Display},
	io::{IoSlice, IoSliceMut},
	os::{
		fd::{FromRawFd, OwnedFd, RawFd},
		unix::net::{AncillaryData, SocketAncillary, UnixStream},
	},
	path::PathBuf,
};

use crate::{
	CYAN, DebugLevel, GREEN, NONE, RED, Rl,
	wayland::{Boxed, Id, OpCode, Raw, WaylandError, WaylandObjectKind, surface::Surface},
	wlog,
};

pub(crate) struct WireRequest {
	pub(crate) sender_id: Id,
	pub(crate) kind: WaylandObjectKind,
	pub(crate) opcode: OpCode,
	pub(crate) opname: &'static str,
	pub(crate) args: Vec<WireArgument>,
}

pub(crate) struct WireEventRaw {
	pub(crate) recv_id: Id,
	pub(crate) opcode: OpCode,
	pub(crate) payload: Vec<u8>,
}

#[derive(Debug)]
pub(crate) enum WireArgument {
	Int(i32),
	UnInt(u32),
	// add actual type and helper funs
	FixedPrecision(u32),
	String(String),
	Obj(Id),
	NewId(Id),
	NewIdSpecific(&'static str, u32, Id),
	Arr(Vec<u8>),
	// u32?
	FileDescriptor(RawFd),
}

pub(crate) enum Action {
	RequestRequest(WireRequest),
	EventResponse(WireEventRaw),
	CallbackDone(Id, u32),
	Sync(Id),
	Error(RecvError),
	Trace(DebugLevel, &'static str, String),
	IdDeletion(Id),
	Resize(u32, u32, Rl<Surface>),
}

pub(crate) enum Consequence {
	Request(WireRequest),
	IdDeletion(Id),
	Trace(DebugLevel, &'static str, String, &'static str, &'static str),
}

pub(crate) struct MessageManager {
	pub(crate) sock: UnixStream,
	pub(crate) q: VecDeque<Action>,
}

impl Drop for MessageManager {
	fn drop(&mut self) {
		wlog!(DebugLevel::Important, "wlmm", "destroying self", GREEN, CYAN);
		if let Err(er) = self.discon() {
			wlog!(DebugLevel::Error, "wlmm", format!("failed to discon: {er}"), GREEN, RED);
		} else {
			wlog!(DebugLevel::Error, "wlmm", "discon was successful", GREEN, CYAN);
		}
	}
}

impl Display for WireRequest {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"sending {} ({}°) for object {} ({}) with args {:?}",
			self.opname, self.opcode, self.kind, self.sender_id, self.args
		)
	}
}

impl Default for MessageManager {
	fn default() -> Self {
		Self::from_defualt_env().unwrap()
	}
}

impl MessageManager {
	pub(crate) fn new(sockname: &str) -> Result<Self, WaylandError> {
		let base = env::var("XDG_RUNTIME_DIR")?;
		let mut base = PathBuf::from(base);
		base.push(sockname);
		let sock = UnixStream::connect(base)?;
		sock.set_nonblocking(true)?;
		let wlmm = Self {
			sock,
			q: VecDeque::new(),
		};

		Ok(wlmm)
	}

	pub(crate) fn from_defualt_env() -> Result<Self, WaylandError> {
		let env = env::var("WAYLAND_DISPLAY");
		match env {
			Ok(x) => Ok(Self::new(&x)?),
			Err(er) => match er {
				std::env::VarError::NotPresent => Err(WaylandError::NoWaylandDisplay),
				_ => Err(WaylandError::Env(er)),
			},
		}
	}

	pub(crate) fn discon(&self) -> Result<(), WaylandError> {
		Ok(self.sock.shutdown(std::net::Shutdown::Both)?)
	}

	pub(crate) fn send_request_logged(&self, msg: &mut WireRequest) -> Result<(), WaylandError> {
		wlog!(DebugLevel::Trivial, "wlmm", format!("{msg}"), GREEN, NONE);
		self.send_request(msg)
	}

	fn send_request(&self, msg: &mut WireRequest) -> Result<(), WaylandError> {
		let mut buf: Vec<u8> = vec![];
		buf.append(&mut Vec::from(msg.sender_id.raw().to_ne_bytes()));
		buf.append(&mut vec![0, 0, 0, 0]);
		let mut fds = vec![];
		for obj in msg.args.iter_mut() {
			match obj {
				WireArgument::Arr(x) => {
					let len = x.len() as u32;
					buf.append(&mut Vec::from(len.to_ne_bytes()));
					buf.append(x);
					buf.resize(x.len() - (x.len() % 4) - 4, 0);
				}
				WireArgument::FileDescriptor(x) => {
					fds.push(*x);
				}
				_ => buf.append(&mut obj.as_vec_u8()),
			}
		}
		let word2 = (buf.len() << 16) as u32 | (msg.opcode.raw() & 0x0000ffffu32);
		let word2 = word2.to_ne_bytes();
		for (en, ix) in (4..=7).enumerate() {
			buf[ix] = word2[en];
		}
		let mut ancillary_buf = [0; 128];
		let mut ancillary = SocketAncillary::new(&mut ancillary_buf);
		ancillary.add_fds(&fds);
		wlog!(DebugLevel::SuperVerbose, "wlmm", format!("buf: {buf:?}"), GREEN, NONE);
		self.sock.send_vectored_with_ancillary(&[IoSlice::new(&buf)], &mut ancillary)?;
		Ok(())
	}

	fn get_socket_data(&self, buf: &mut [u8]) -> Result<(usize, Vec<OwnedFd>), WaylandError> {
		let mut iov = [IoSliceMut::new(buf)];

		let mut aux_buf: [u8; 64] = [0; 64];
		let mut aux = SocketAncillary::new(&mut aux_buf);

		match self.sock.recv_vectored_with_ancillary(&mut iov, &mut aux) {
			Ok(l) => {
				let mut fds = vec![];
				for msg in aux.messages() {
					if let Ok(AncillaryData::ScmRights(scmr)) = msg {
						for fd in scmr {
							let fd = unsafe { OwnedFd::from_raw_fd(fd) };
							fds.push(fd);
						}
					}
				}
				Ok((l, fds))
			}
			Err(er) => match er.kind() {
				std::io::ErrorKind::WouldBlock => Ok((0, vec![])),
				_ => Err(WaylandError::Io(er)),
			},
		}
	}

	pub(crate) fn get_events(&mut self) -> Result<(usize, Vec<OwnedFd>), WaylandError> {
		let mut b = [0; 8192];
		let (len, fds) = self.get_socket_data(&mut b)?;
		if len == 0 {
			return Ok((0, vec![]));
		}

		let mut cursor = 0;
		let mut ctr = 0;
		while cursor < len {
			let sender_id =
				u32::from_ne_bytes([b[cursor], b[cursor + 1], b[cursor + 2], b[cursor + 3]]);
			let byte2 =
				u32::from_ne_bytes([b[cursor + 4], b[cursor + 5], b[cursor + 6], b[cursor + 7]]);

			let recv_len = byte2 >> 16;
			// println!("len: {}", recv_len);
			if recv_len < 8 {
				return Err(WaylandError::RecvLenBad);
			}
			let opcode = (byte2 & 0x0000ffff) as usize;

			let payload = Vec::from(&b[cursor + 8..cursor + recv_len as usize]);

			let event = WireEventRaw {
				recv_id: Id(sender_id),
				opcode: OpCode(opcode as u32),
				payload,
			};
			self.q.push_back(Action::EventResponse(event));
			ctr += 1;

			cursor += recv_len as usize;
		}
		Ok((ctr, fds))
	}

	pub(crate) fn queue_request(&mut self, req: WireRequest) {
		self.q.push_back(Action::RequestRequest(req));
	}

	pub(crate) fn queue(&mut self, entry: Action) {
		self.q.push_back(entry);
	}
}

impl WireArgument {
	// size in bytes
	pub(crate) fn size(&self) -> usize {
		match self {
			WireArgument::Int(_) => 4,
			WireArgument::UnInt(_) => 4,
			WireArgument::FixedPrecision(_) => 4,
			WireArgument::String(x) => x.len(),
			WireArgument::Obj(_) => 4,
			WireArgument::NewId(_) => 4,
			WireArgument::NewIdSpecific(x, _, _) => x.len() + 8,
			WireArgument::Arr(x) => x.len(),
			WireArgument::FileDescriptor(_) => 4,
		}
	}

	pub(crate) fn as_vec_u8(&self) -> Vec<u8> {
		match self {
			WireArgument::Int(x) => Vec::from(x.to_ne_bytes()),
			WireArgument::UnInt(x) => Vec::from(x.to_ne_bytes()),
			WireArgument::FixedPrecision(x) => Vec::from(x.to_ne_bytes()),
			WireArgument::String(x) => {
				let mut complete: Vec<u8> = vec![];
				// str len + 1 because of nul
				let len = &mut Vec::from(((x.len() + 1) as u32).to_ne_bytes());
				complete.append(len);
				complete.append(&mut Vec::from(x.as_str()));
				// nul
				complete.push(0);
				// padding
				complete.resize(complete.len() - (complete.len() % 4) + 4, 0);
				// println!("complete len rn: {}", complete.len());
				complete
			}
			WireArgument::Obj(x) => Vec::from(x.raw().to_ne_bytes()),
			WireArgument::NewId(x) => Vec::from(x.raw().to_ne_bytes()),
			WireArgument::NewIdSpecific(x, y, z) => {
				let mut complete: Vec<u8> = vec![];
				// str len
				let len = &mut Vec::from(((x.len() + 1) as u32).to_ne_bytes());
				complete.append(len);
				// println!("len: {}, complete: {:?}", complete.len(), complete);
				complete.append(&mut Vec::from(*x));
				// println!("len: {}, complete: {:?}", complete.len(), complete);
				complete.push(0);
				// println!("len: {}, complete: {:?}", complete.len(), complete);
				// pad str
				let clen = complete.len();
				complete.resize(clen - (clen % 4) + (4 * (clen % 4).clamp(0, 1)), 0);
				// println!("len: {}, complete: {:?}", complete.len(), complete);
				complete.append(&mut Vec::from(y.to_ne_bytes()));
				// println!("len: {}, complete: {:?}", complete.len(), complete);
				complete.append(&mut Vec::from(z.raw().to_ne_bytes()));
				// println!("len: {}, complete: {:?}", complete.len(), complete);
				// println!("complete len rn: {}", complete.len());
				complete
			}
			WireArgument::Arr(_) => panic!("debil"),
			WireArgument::FileDescriptor(x) => Vec::from(x.to_ne_bytes()),
		}
	}
}

pub(crate) trait FromWirePayload: Sized {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError>;
}

fn is_empty(payload: &[u8]) -> Result<(), WaylandError> {
	if payload.is_empty() {
		Err(WaylandError::EmptyFromWirePayload)
	} else {
		Ok(())
	}
}

impl FromWirePayload for String {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		let p = payload;
		let len = u32::from_wire(payload)? as usize;
		let ix = p[4..4 + len]
			.iter()
			.enumerate()
			.find(|(_, c)| **c == b'\0')
			.map(|(e, _)| e)
			.unwrap_or_default();
		Ok(String::from_utf8(p[4..4 + ix].to_vec())?)
	}
}

impl FromWirePayload for u16 {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		let p = payload;
		Ok(u16::from_ne_bytes([p[0], p[1]]))
	}
}

impl FromWirePayload for u32 {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		let p = payload;
		Ok(u32::from_ne_bytes([p[0], p[1], p[2], p[3]]))
	}
}

impl FromWirePayload for u64 {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		let p = payload;
		Ok(u64::from_ne_bytes([p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]]))
	}
}

impl FromWirePayload for i32 {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		let p = payload;
		Ok(i32::from_ne_bytes([p[0], p[1], p[2], p[3]]))
	}
}

impl FromWirePayload for Vec<u32> {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		payload[4..].chunks(4).map(u32::from_wire).collect()
	}
}

impl FromWirePayload for Vec<u16> {
	fn from_wire(payload: &[u8]) -> Result<Self, WaylandError> {
		is_empty(payload)?;
		payload[4..].chunks(2).map(u16::from_wire).collect()
	}
}

#[derive(Debug)]
pub(crate) struct RecvError {
	pub(crate) recv_id: Id,
	pub(crate) id: Id,
	pub(crate) code: OpCode,
	pub(crate) msg: String,
}

impl Error for RecvError {}

impl Display for RecvError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"recv error on id {} for id {}, code {}: {}",
			self.recv_id, self.id, self.code, self.msg
		)
	}
}
