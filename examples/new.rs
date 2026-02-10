use std::error::Error;

use waytinier::{
	abstraction::{app::App, wizard::TopLevelWindowWizard},
	wayland::shm::ShmBackend,
};

struct AppState {}

fn main() -> Result<(), Box<dyn Error>> {
	let mut app = App::new()?;
	let backend = ShmBackend::new(&mut app)?;
	let window = TopLevelWindowWizard::new(&mut app).with_backend(backend).spawn()?;
	app.push_presenter(window);

	let mut state = AppState {};

	while !app.finished {
		app.work(&mut state, |state, ss| {});
	}
	Ok(())
}
