#![allow(dead_code)]
#![allow(unused)]

use std::{collections::HashMap, error::Error, marker::PhantomData};

use crate::{
	Rl,
	abstraction::{presenter::TopLevelWindow, wizard::TopLevelWindowWizard},
	init_logger,
	wayland::{
		God, IdentManager, WaylandError, buffer::BufferBackend, compositor::Compositor,
		display::Display, registry::Registry, shm::ShmBackend, surface::Surface,
		wire::MessageManager,
	},
};

pub enum Presenter {
	TopLevelWindow(TopLevelWindow),
}

#[derive(Default)]
pub(crate) struct PresenterMap {
	pub(crate) last_id: usize,
	pub(crate) inner: HashMap<usize, Presenter>,
}

impl PresenterMap {
	pub(crate) fn push(&mut self, to_push: Presenter) {
		self.inner.insert(self.last_id, to_push);
		self.last_id += 1;
	}
}

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
		let registry = Registry::new_registered(&mut god);
		wait_for_sync!();
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

	pub fn push_presenter(&mut self, presenter: Presenter) {
		self.presenters.push(presenter);
	}
}
