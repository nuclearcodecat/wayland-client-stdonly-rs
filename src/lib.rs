#![feature(unix_socket_ancillary_data)]
#![feature(variant_count)]
#![feature(deque_extend_front)]

use std::{
	cell::RefCell,
	rc::{Rc, Weak},
	sync::OnceLock,
};

pub mod abstraction;
pub mod wayland;

pub const NONE: &str = "\x1b[0m";
pub const RED: &str = "\x1b[31m";
pub const CYAN: &str = "\x1b[36m";
pub const YELLOW: &str = "\x1b[33m";
pub const GREEN: &str = "\x1b[32m";
pub const WHITE: &str = "\x1b[37m";
pub const PURPLE: &str = "\x1b[35m";

pub(crate) static DEBUGLVL: OnceLock<isize> = OnceLock::new();

pub fn init_logger() {
	let dbug: isize =
		std::env::var("WAYTINIER_DEBUGLVL").unwrap_or(String::from("2")).parse().unwrap_or(2);
	let _ = DEBUGLVL.set(dbug);
}

#[cfg(not(feature = "nolog"))]
pub(crate) fn get_dbug() -> isize {
	*DEBUGLVL.get().unwrap_or(&0)
}

#[allow(dead_code)]
#[repr(isize)]
#[derive(PartialEq)]
pub(crate) enum DebugLevel {
	None = -1,
	Error,
	Important,
	Trivial,
	Verbose,
	SuperVerbose,
}

#[macro_export]
macro_rules! wlog {
	($lvl:expr, $header:expr, $msg:expr, $header_color:expr, $msg_color:expr) => {{
		#[cfg(not(feature = "nolog"))]
		if $crate::get_dbug() >= $lvl as isize {
			println!(
				"{}\x1b[7m! {} !\x1b[0m{} {}{}{}",
				$header_color,
				$header,
				$crate::NONE,
				$msg_color,
				$msg,
				$crate::NONE,
			)
		}
		#[cfg(feature = "nolog")]
		let _ = (&$lvl, &$header, &$msg, &$header_color, &$msg_color);
	}};
}

#[macro_export]
macro_rules! dbug {
	($msg:expr) => {
		$crate::wlog!($crate::DebugLevel::Important, "DEBUG", $msg, $crate::CYAN, $crate::CYAN);
	};
}

#[macro_export]
macro_rules! handle_log {
	($self:expr, $lvl:expr, $msg:expr) => {
		$crate::wlog!($lvl, $self.kind_str(), $msg, $crate::WHITE, $crate::NONE);
	};
}

pub(crate) type Rl<T> = Rc<RefCell<T>>;
pub(crate) type Wl<T> = Weak<RefCell<T>>;

#[macro_export]
macro_rules! rl {
	($x:expr) => {
		std::rc::Rc::new(std::cell::RefCell::new($x))
	};
}
