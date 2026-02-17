use waytinier::{App, DmaBackend, ShmBackend, TopLevelWindowWizard, wayland::WaylandError};

struct AppState {}

fn main() -> Result<(), WaylandError> {
	let mut app = App::new()?;
	let backend = DmaBackend::new(&mut app)?;
	let window = TopLevelWindowWizard::new(&mut app).with_backend(&backend).spawn()?;
	app.push_presenter(window);

	let mut state = AppState {};

	while !app.finished {
		app.work(&mut state, |_state, ss| {
			ss.buf.chunks_mut(4).for_each(|chunk| {
				chunk[0] = 0;
				chunk[1] = 0;
				chunk[2] = 0;
				chunk[3] = 0;
			});
		})?;
	}
	Ok(())
}
