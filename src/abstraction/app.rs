#![allow(dead_code)]
#![allow(unused)]

use std::{collections::HashMap, error::Error, marker::PhantomData};

use crate::{
	Rl,
	wayland::{
		IdentManager, buffer::BufferBackend, compositor::Compositor, display::Display,
		registry::Registry, surface::Surface, wire::MessageManager,
	},
};

pub trait Presenter<B: BufferBackend> {
	fn backend(&self) -> B;
	fn surface(&self) -> Rl<Surface>;
	fn try_close(&mut self) -> bool {
		true
	}
}

pub(crate) struct PresenterMap<B: BufferBackend, P: Presenter<B>> {
	pub(crate) last_id: usize,
	pub(crate) inner: HashMap<usize, P>,
	// todo remove this when i finally use the backend
	pub(crate) _marker: PhantomData<B>,
}

pub(crate) struct App<B: BufferBackend, P: Presenter<B>> {
	pub(crate) presenters: PresenterMap<B, P>,
	pub(crate) compositor: Rl<Compositor>,
	pub(crate) registry: Rl<Registry>,
	pub(crate) display: Rl<Display>,
	pub finished: bool,
	pub(crate) wlmm: MessageManager,
	pub(crate) wlim: IdentManager,
}

impl<B: BufferBackend, P: Presenter<B>> App<B, P> {
	pub fn new() -> Result<Self, Box<dyn Error>> {
		let mut wlim = IdentManager::default();
		let display = Display::new_registered(&mut wlim);
		let registry = Registry::new_registered(&mut wlim);
		todo!()
	}
}
