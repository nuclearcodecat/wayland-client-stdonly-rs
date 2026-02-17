#![allow(dead_code)]
#![allow(unused)]

use std::{collections::HashMap, error::Error, marker::PhantomData};

use crate::{
	Rl,
	abstraction::{
		presenter::{Presenter, PresenterMap, PresenterObject, TopLevelWindow},
		wizard::TopLevelWindowWizard,
	},
	dbug, init_logger, rl, wait_for_sync,
	wayland::{
		Boxed, God, IdentManager, PixelFormat, WaylandError, buffer::BufferBackend,
		compositor::Compositor, display::Display, registry::Registry, shm::ShmBackend,
		surface::Surface, wire::MessageManager,
	},
};

pub struct App {
	pub(crate) presenters: PresenterMap,
	pub(crate) compositor: Rl<Compositor>,
	pub(crate) registry: Rl<Registry>,
	pub(crate) display: Rl<Display>,
	pub finished: bool,
	pub(crate) god: God,
}

impl App {
	pub fn new() -> Result<Self, WaylandError> {
		init_logger();

		let mut god = God::default();
		let display = Display::new_registered(&mut god);
		let registry = Registry::new_registered_made(&mut god, &display);
		wait_for_sync!(display, &mut god);
		let compositor = Compositor::new_registered_bound(&mut god, &registry)?;
		Ok(Self {
			presenters: PresenterMap::default(),
			compositor,
			registry,
			display,
			finished: false,
			god,
		})
	}

	pub fn push_presenter(&mut self, presenter: Box<dyn PresenterObject>) {
		self.presenters.push(presenter);
	}

	pub(crate) fn make_surface(&mut self, w: u32, h: u32, pf: PixelFormat) -> Rl<Surface> {
		Surface::new_registered_made(&mut self.god, &self.compositor, w, h, pf)
	}

	pub fn work<F, S>(&mut self, state: &mut S, mut render_fun: F) -> Result<bool, WaylandError>
	where
		F: FnMut(&mut S, Snapshot),
	{
		for (id, presenter) in &mut self.presenters.inner {
			// only tlw for now
			let window = match presenter.any().downcast_mut::<TopLevelWindow>() {
				Some(w) => w,
				None => continue,
			};
			let cb = &mut window.frame_cb;
			let frame = &mut window.frame;

			self.god.handle_events();

			// check if user wants to close window - the cb might not be a good idea
			if window.xdg_toplevel.borrow().close_requested && (window.close_cb)() {
				// make a trait object with a set_finished() method
				window.finished = true;
				continue;
			};
			if window.xdg_surface.borrow().is_configured {
				let ready = match &cb.clone() {
					Some(cb) => cb.borrow().done,
					None => true,
				};

				let mut surf = window.surface.borrow_mut();
				let surf_w = surf.w;
				let surf_h = surf.h;
				if surf.attached_buf.is_none() {
					dbug!("no buf");
					drop(surf);
					let buf = window.backend.borrow_mut().as_mut().make_buffer(
						&mut self.god,
						surf_w,
						surf_h,
						&window.surface,
						&window.backend,
						&self.registry,
					)?;
					let mut surf = window.surface.borrow_mut();
					surf.attach_buffer_obj(&mut self.god, buf)?;
					surf.commit(&mut self.god);
					drop(surf);
					self.god.handle_events()?;
					continue;
				}

				if ready {
					let new_cb = surf.frame(&mut self.god)?;
					*cb = Some(new_cb);
					*frame = frame.wrapping_add(1);

					unsafe {
						let slice = &mut *surf.get_buffer_slice()?;
						let buf = surf.attached_buf.clone().ok_or("no buffer");

						let ss = Snapshot {
							buf: slice,
							w: surf_w,
							h: surf_h,
							pf: surf.pf,
							frame: *frame,
							presenter_id: *id,
						};

						render_fun(state, ss);
					}
					surf.attach_buffer(&mut self.god)?;
					surf.repaint(&mut self.god)?;
					surf.commit(&mut self.god);
				}
			}
		}
		self.presenters.inner.retain(|_, pres| !pres.is_finished());
		if self.presenters.inner.iter().all(|(_, p)| p.is_finished()) {
			self.finished = true;
		};
		Ok(self.finished)
	}
}

pub struct Snapshot<'a> {
	pub buf: &'a mut [u8],
	pub w: u32,
	pub h: u32,
	pub pf: PixelFormat,
	pub frame: usize,
	pub presenter_id: usize,
}
