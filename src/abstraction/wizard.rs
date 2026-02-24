use crate::{
	Rl,
	abstraction::{
		app::App,
		presenter::{PresenterObject, TopLevelWindow},
	},
	wait_for_sync,
	wayland::{
		PixelFormat, WaytinierError,
		buffer::BufferBackend,
		surface::Surface,
		xdg_shell::{surface::XdgSurface, toplevel::XdgTopLevel, wm_base::XdgWmBase},
	},
};

pub struct TopLevelWindowWizard<'a> {
	pub(crate) app_id: Option<String>,
	pub(crate) title: Option<String>,
	pub(crate) width: Option<u32>,
	pub(crate) height: Option<u32>,
	pub(crate) parent: &'a mut App,
	pub(crate) close_cb: Option<Box<dyn FnMut() -> bool>>,
	pub(crate) backend: Option<Rl<BufferBackend>>,
	pub(crate) pf: Option<PixelFormat>,
	pub(crate) xdg_wm_base: Option<Rl<XdgWmBase>>,
}

impl<'a> TopLevelWindowWizard<'a> {
	pub fn new(parent: &'a mut App) -> Self {
		Self {
			app_id: None,
			title: None,
			width: None,
			height: None,
			parent,
			close_cb: None,
			backend: None,
			pf: None,
			xdg_wm_base: None,
		}
	}

	pub fn with_app_id(mut self, app_id: &str) -> Self {
		self.app_id = Some(String::from(app_id));
		self
	}

	pub fn with_title(mut self, title: &str) -> Self {
		self.title = Some(String::from(title));
		self
	}

	pub fn with_width(mut self, width: u32) -> Self {
		self.width = Some(width);
		self
	}

	pub fn with_height(mut self, height: u32) -> Self {
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

	pub fn with_pixel_format(mut self, pf: PixelFormat) -> Self {
		self.pf = Some(pf);
		self
	}

	pub fn with_existing_xdg_wm_base(mut self, xdg_wm_base: Rl<XdgWmBase>) -> Self {
		self.xdg_wm_base = Some(xdg_wm_base);
		self
	}

	pub fn with_backend(mut self, backend: &Rl<BufferBackend>) -> Self {
		self.backend = Some(backend.clone());
		self
	}

	pub fn spawn(self) -> Result<Box<dyn PresenterObject>, WaytinierError> {
		let mut god = &mut self.parent.god;
		let registry = &self.parent.registry;
		let compositor = &self.parent.compositor;
		let pf = self.pf.unwrap_or_default();
		let w = self.width.unwrap_or(800);
		let h = self.height.unwrap_or(600);
		let surface = Surface::new_registered_made(god, compositor, w, h, pf);
		let _xdg_wm_base =
			self.xdg_wm_base.unwrap_or(XdgWmBase::new_registered_bound(registry, god)?);
		let xdg_surface = XdgSurface::new_registered(god, &_xdg_wm_base, &surface);
		let xdg_toplevel = XdgTopLevel::new_registered_gotten(god, &xdg_surface);
		if let Some(title) = self.title {
			xdg_toplevel.borrow_mut().set_title(god, &title);
		};
		if let Some(appid) = self.app_id {
			xdg_toplevel.borrow_mut().set_app_id(god, &appid);
		};
		let backend = self.backend.ok_or(WaytinierError::ExpectedSomeValue(
			"attach a BufferBackend trait object with ::with_backend()",
		))?;
		surface.borrow().commit(god);
		wait_for_sync!(&self.parent.display, &mut god);
		let tlw = TopLevelWindow {
			_xdg_wm_base,

			xdg_toplevel,
			xdg_surface,
			backend,
			surface,
			close_cb: Box::new(|| true),
			frame: 0,
			frame_cb: None,
			finished: false,
		};
		Ok(Box::new(tlw))
	}
}
