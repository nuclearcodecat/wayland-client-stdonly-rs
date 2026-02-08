use crate::{
	Rl,
	abstraction::app::{App, Presenter},
	wayland::{buffer::BufferBackend, shm::ShmBackend, surface::Surface},
};

pub struct TopLevelWindowWizard<'a, P: Presenter<B>, B: BufferBackend = ShmBackend> {
	pub(crate) app_id: Option<String>,
	pub(crate) title: Option<String>,
	pub(crate) width: Option<i32>,
	pub(crate) height: Option<i32>,
	pub(crate) sur: Option<Rl<Surface>>,
	pub(crate) parent: &'a mut App<B, P>,
	pub(crate) close_cb: Option<Box<dyn FnMut() -> bool>>,
	pub(crate) backend: Option<B>,
}

impl<P: Presenter<B>, B: BufferBackend> TopLevelWindowWizard<'_, P, B> {
	pub fn with_app_id(mut self, app_id: &str) -> Self {
		self.app_id = Some(String::from(app_id));
		self
	}

	pub fn with_title(mut self, title: &str) -> Self {
		self.title = Some(String::from(title));
		self
	}

	pub fn with_width(mut self, width: i32) -> Self {
		self.width = Some(width);
		self
	}

	pub fn with_height(mut self, height: i32) -> Self {
		self.height = Some(height);
		self
	}

	pub fn with_close_callback<F>(mut self, cb: F) -> Self
	where
		F: FnMut() -> bool + 'static,
	{
		self.close_cb = Some(Box::new(cb));
		self
	}
}
