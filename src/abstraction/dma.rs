pub(crate) const fn fourcc_code(a: u8, b: u8, c: u8, d: u8) -> u32 {
	let a = a as u32;
	let b = b as u32;
	let c = c as u32;
	let d = d as u32;
	(a | b << 8) | (c << 16) | (d << 24)
}

// https://github.com/torvalds/linux/blob/master/include/uapi/drm/drm_fourcc.h line 467
#[repr(u64)]
pub(crate) enum DrmFormatModVendor {
	None = 0,
}

pub(crate) const fn fourcc_mod_code(vendor: DrmFormatModVendor, val: u64) -> u64 {
	(vendor as u64) << 56 | val & 0x00ffffffffffffff
}

pub(crate) const DRM_FORMAT_MOD_LINEAR: u64 = fourcc_mod_code(DrmFormatModVendor::None, 0);
