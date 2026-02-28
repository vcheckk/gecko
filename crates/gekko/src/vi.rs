pub mod regs;

use crate::mmio::Mmio;

pub fn xfb_addr(mmio: &Mmio) -> u32 {
    let top = mmio.read_register::<regs::TopFieldBase>();
    let addr = (top.xfb_addr() << 9) | ((top.page_offset() as u32) << 24);
    addr
}

pub const XFB_WIDTH: usize = 640;
pub const XFB_HEIGHT: usize = 574;

#[rustfmt::skip]
pub fn render_xfb(mmio: &mut Mmio) -> Vec<u32> {
    let mut pixels = vec![0u32; XFB_WIDTH * XFB_HEIGHT];
    let xfb_addr = xfb_addr(mmio);

    // XFB is YUY2 (YCbCr 4:2:2): each 32-bit word = [Y0][Cb][Y1][Cr] (big-endian)
    // One word -> two adjacent pixels sharing Cb and Cr.
    let ycbcr_to_rgb = |y: f32, cb: f32, cr: f32| -> u32 {
        let r = (1.164 * y + 1.596 * cr).clamp(0.0, 255.0) as u8;
        let g = (1.164 * y - 0.813 * cr - 0.391 * cb).clamp(0.0, 255.0) as u8;
        let b = (1.164 * y + 2.018 * cb).clamp(0.0, 255.0) as u8;
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    };

    for i in 0..(XFB_WIDTH * XFB_HEIGHT / 2) {
        let word = mmio.phys_read_u32(xfb_addr + (i as u32) * 4);
        let y0 = ((word >> 24) & 0xFF) as f32 - 16.0;
        let cb = ((word >> 16) & 0xFF) as f32 - 128.0;
        let y1 = ((word >>  8) & 0xFF) as f32 - 16.0;
        let cr = ( word        & 0xFF) as f32 - 128.0;
        pixels[i * 2]     = ycbcr_to_rgb(y0, cb, cr);
        pixels[i * 2 + 1] = ycbcr_to_rgb(y1, cb, cr);
    }
    pixels
}