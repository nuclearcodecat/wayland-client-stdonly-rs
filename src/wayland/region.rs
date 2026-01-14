pub struct Region {
	pub(crate) x: i32,
	pub(crate) y: i32,
	pub(crate) w: i32,
	pub(crate) h: i32,
}

impl Region {
	pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
		Self {
			x,
			y,
			w,
			h,
		}
	}
}
