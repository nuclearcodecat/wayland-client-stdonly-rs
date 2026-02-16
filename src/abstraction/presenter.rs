use std::{any::Any, collections::HashMap};

use crate::{
	Rl,
	wayland::{
		buffer::BufferBackend,
		callback::Callback,
		surface::Surface,
		xdg_shell::{surface::XdgSurface, toplevel::XdgTopLevel, wm_base::XdgWmBase},
	},
};

pub enum Presenter {
	TopLevelWindow(TopLevelWindow),
}

#[derive(Default)]
pub(crate) struct PresenterMap {
	pub(crate) last_id: usize,
	pub(crate) inner: HashMap<usize, Box<dyn PresenterObject>>,
}

impl PresenterMap {
	pub(crate) fn push(&mut self, to_push: Box<dyn PresenterObject>) {
		self.inner.insert(self.last_id, to_push);
		self.last_id += 1;
	}
}

pub trait PresenterObject {
	fn is_finished(&self) -> bool;
	fn set_finished(&mut self, finished: bool);
	fn any(&mut self) -> &mut dyn Any;
}

pub struct TopLevelWindow {
	pub(crate) xdg_toplevel: Rl<XdgTopLevel>,
	pub(crate) xdg_surface: Rl<XdgSurface>,
	pub(crate) xdg_wm_base: Rl<XdgWmBase>,
	pub(crate) backend: Rl<Box<dyn BufferBackend>>,
	pub(crate) surface: Rl<Surface>,
	pub(crate) app_id: Option<String>,
	pub(crate) title: Option<String>,
	pub(crate) close_cb: Box<dyn FnMut() -> bool>,
	pub(crate) frame: usize,
	pub(crate) frame_cb: Option<Rl<Callback>>,
	pub(crate) finished: bool,
}

impl PresenterObject for TopLevelWindow {
	fn is_finished(&self) -> bool {
		self.finished
	}

	fn set_finished(&mut self, finished: bool) {
		self.finished = finished
	}

	fn any(&mut self) -> &mut dyn Any {
		self
	}
}
