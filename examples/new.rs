use std::error::Error;

use waytinier::{
	abstraction::{app::App, wizard::TopLevelWindowWizard},
	wayland::shm::ShmBackend,
};

fn main() -> Result<(), Box<dyn Error>> {
	let mut app = App::new()?;
	let window = TopLevelWindowWizard::new(&mut app).with_backend(ShmBackend {}).spawn()?;
	app.push_presenter(window);
	Ok(())
}
