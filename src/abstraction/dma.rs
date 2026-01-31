const fn fourcc_code(a: u8, b: u8, c: u8, d: u8) -> u32 {
	let a = a as u32;
	let b = b as u32;
	let c = c as u32;
	let d = d as u32;
	(a | b << 8) | (c << 16) | (d << 24)
}

pub(crate) const DRM_FORMAT_ARGB8888: u32 = fourcc_code(b'A', b'R', b'2', b'4');
