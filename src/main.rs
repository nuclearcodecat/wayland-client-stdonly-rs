use std::{cell::RefCell, env, error::Error, rc::Rc};

use wayland_raw::wayland::{
	Context, IdentManager, RcCell,
	callback::Callback,
	compositor::Compositor,
	display::Display,
	region::Region,
	shm::{PixelFormat, SharedMemory},
	wire::MessageManager,
	xdgshell::{XdgTopLevel, XdgWmBase},
};

fn main() -> Result<(), Box<dyn Error>> {
	let wlim = IdentManager::default();
	let wlmm = MessageManager::new(&env::var("WAYLAND_DISPLAY")?)?;
	let ctx = Context::new(wlmm, wlim);
	let ctx = Rc::new(RefCell::new(ctx));

	let display = Display::new(ctx.clone());
	let registry = display.borrow_mut().make_registry()?;
	ctx.borrow_mut().handle_events()?;
	let compositor = Compositor::new_bound(&mut registry.borrow_mut(), ctx.clone())?;
	let surface = compositor.borrow_mut().make_surface()?;
	let shm = SharedMemory::new_bound_initialized(&mut registry.borrow_mut(), ctx.clone())?;
	let shm_pool = shm.borrow_mut().make_pool(500 * 500 * 4)?;
	unsafe {
		let x = &mut *shm_pool.borrow_mut().slice.unwrap();
		x.chunks_mut(4).for_each(|y| {
			y[0] = 255;
		});
	}
	ctx.borrow_mut().handle_events()?;
	let buf = shm_pool.borrow_mut().make_buffer((0, 500, 500, 500), PixelFormat::Xrgb888)?;
	let xdg_wm_base = XdgWmBase::new_bound(&mut registry.borrow_mut())?;
	let xdg_surface = xdg_wm_base.borrow_mut().make_xdg_surface(surface.clone())?;
	let xdg_toplevel = XdgTopLevel::new_from_xdg_surface(xdg_surface.clone())?;
	xdg_toplevel.borrow_mut().set_app_id(String::from("wayland-raw"))?;
	xdg_toplevel.borrow_mut().set_title(String::from("wayland-raw"))?;
	surface.borrow_mut().attach_buffer(buf.clone())?;
	surface.borrow_mut().commit()?;
	let mut frame: usize = 0;
	let mut cb: Option<RcCell<Callback>> = None;

	loop {
		ctx.borrow_mut().handle_events()?;

		if xdg_surface.borrow().is_configured {
			println!("looping");
			let ready = match &cb.clone() {
				Some(cb) => cb.borrow().done,
				None => true,
			};

			if ready {
				let new_cb = surface.borrow_mut().frame()?;
				cb = Some(new_cb);

				let (r, g, b) = hsv_to_rgb(frame as f64, 1.0, 1.0);

				unsafe {
					let slice = &mut *shm_pool.borrow_mut().slice.unwrap();
					frame = frame.wrapping_add(1);

					for pixel in slice.chunks_mut(4) {
						pixel[0] = r;
						pixel[1] = g;
						pixel[2] = b;
					}
				}
				surface.borrow_mut().attach_buffer(buf.clone())?;
				surface.borrow_mut().damage_buffer(Region::new(0, 0, 500, 500))?;
				surface.borrow_mut().commit()?;
			}
		}
	}
}

// stolen from hsv library
pub fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> (u8, u8, u8) {
	fn is_between(value: f64, min: f64, max: f64) -> bool {
		min <= value && value < max
	}

	// check_bounds(hue, saturation, value);

	let c = value * saturation;
	let h = hue / 60.0;
	let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
	let m = value - c;

	let (r, g, b): (f64, f64, f64) = if is_between(h, 0.0, 1.0) {
		(c, x, 0.0)
	} else if is_between(h, 1.0, 2.0) {
		(x, c, 0.0)
	} else if is_between(h, 2.0, 3.0) {
		(0.0, c, x)
	} else if is_between(h, 3.0, 4.0) {
		(0.0, x, c)
	} else if is_between(h, 4.0, 5.0) {
		(x, 0.0, c)
	} else {
		(c, 0.0, x)
	};

	(((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8)
}
