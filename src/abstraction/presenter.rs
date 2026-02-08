use crate::{
	Rl,
	wayland::{
		buffer::BufferBackend,
		surface::Surface,
		xdg_shell::{surface::XdgSurface, toplevel::XdgTopLevel, wm_base::XdgWmBase},
	},
};

pub struct TopLevelWindow {
	pub(crate) xdg_toplevel: Rl<XdgTopLevel>,
	pub(crate) xdg_surface: Rl<XdgSurface>,
	pub(crate) xdg_wm_base: Rl<XdgWmBase>,
	pub(crate) backend: Box<dyn BufferBackend>,
	pub(crate) surface: Rl<Surface>,
	pub(crate) app_id: Option<String>,
	pub(crate) title: Option<String>,
}
