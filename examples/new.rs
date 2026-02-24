use waytinier::{App, BufferAccessor, DmaBackend, TopLevelWindowWizard, wayland::WaytinierError};

struct AppState {}

fn main() -> Result<(), WaytinierError> {
	let mut app = App::new()?;
	let backend = DmaBackend::new()?;
	let window = TopLevelWindowWizard::new(&mut app).with_backend(&backend).spawn()?;
	app.push_presenter(window);

	let mut state = AppState {};

	while !app.finished {
		app.work(&mut state, |_state, ss| match ss.buf {
			BufferAccessor::ShmSlice(slice) => {
				let slice = unsafe { &mut **slice };
				slice.chunks_mut(4).for_each(|chunk| {
					chunk[0] = 255;
					chunk[1] = 255;
					chunk[2] = 255;
					chunk[3] = 255;
				});
			}
			BufferAccessor::DmaBufFd(_fd) => {
				// panic!()
			}
		})?;
	}
	Ok(())
}
