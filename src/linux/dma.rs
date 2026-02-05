use std::os::fd::RawFd;

use libc::{O_CLOEXEC, O_RDWR};

use crate::{
	dbug,
	linux::ioctl::{DMA_HEAP_IOCTL_ALLOC, DRM_IOCTL_MODE_CREATE_DUMB},
};

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

// /**
//  * struct drm_mode_create_dumb - Create a KMS dumb buffer for scanout.
//  * @height: buffer height in pixels
//  * @width: buffer width in pixels
//  * @bpp: bits per pixel
//  * @flags: must be zero
//  * @handle: buffer object handle
//  * @pitch: number of bytes between two consecutive lines
//  * @size: size of the whole buffer in bytes
//  *
//  * User-space fills @height, @width, @bpp and @flags. If the IOCTL succeeds,
//  * the kernel fills @handle, @pitch and @size.
//  */
// struct drm_mode_create_dumb {
// 	__u32 height;
// 	__u32 width;
// 	__u32 bpp;
// 	__u32 flags;

// 	__u32 handle;
// 	__u32 pitch;
// 	__u64 size;
// };
#[repr(C)]
#[derive(Default, Debug)]
pub(crate) struct ModeCreateDumb {
	pub(crate) height: u32,
	pub(crate) width: u32,
	pub(crate) bpp: u32,
	pub(crate) flags: u32,
	pub(crate) handle: u32,
	pub(crate) pitch: u32,
	pub(crate) size: u64,
}

pub(crate) fn make_dumb_buffer(
	fd: RawFd,
	width: u32,
	height: u32,
	bpp: u32,
) -> Result<ModeCreateDumb, std::io::Error> {
	dbug!(format!("making dumb buffer // fd: {fd}, w: {width}, h: {height}, bpp: {bpp}"));
	let mut mode_create_dumb = ModeCreateDumb {
		width,
		height,
		bpp,
		flags: 0,
		..Default::default()
	};
	dbug!(format!("modecreatedumb\n{:#?}", mode_create_dumb));

	let ret = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB as u64, &mut mode_create_dumb) };
	if ret == -1 {
		return Err(std::io::Error::last_os_error());
	}
	Ok(mode_create_dumb)
}

// /**
//  * struct dma_heap_allocation_data - metadata passed from userspace for
//  *                                      allocations
//  * @len:		size of the allocation
//  * @fd:			will be populated with a fd which provides the
//  *			handle to the allocated dma-buf
//  * @fd_flags:		file descriptor flags used when allocating
//  * @heap_flags:		flags passed to heap
//  *
//  * Provided by userspace as an argument to the ioctl
//  */
// struct dma_heap_allocation_data {
//   __u64 len;
//   __u32 fd;
//   __u32 fd_flags;
//   __u64 heap_flags;
// };
#[repr(C)]
#[derive(Debug)]
pub(crate) struct DmaHeapAllocationData {
	len: u64,
	fd: u32,
	fd_flags: u32,
	heap_flags: u64,
}

pub(crate) fn make_dma_heap(
	fd: RawFd,
	width: u32,
	height: u32,
	bpp: u32,
) -> Result<DmaHeapAllocationData, std::io::Error> {
	dbug!(format!("making dumb buffer // fd: {fd}, w: {width}, h: {height}, bpp: {bpp}"));
	let mut dma_heap_alloc_data = DmaHeapAllocationData {
		len: (width * height * bpp) as u64,
		fd: 0,
		fd_flags: (O_RDWR | O_CLOEXEC) as u32,
		heap_flags: 0,
	};
	dbug!(format!("dma_heap_allocation_data\n{:#?}", dma_heap_alloc_data));

	let ret = unsafe { libc::ioctl(fd, DMA_HEAP_IOCTL_ALLOC as u64, &mut dma_heap_alloc_data) };
	if ret == -1 {
		return Err(std::io::Error::last_os_error());
	}
	Ok(dma_heap_alloc_data)
}
