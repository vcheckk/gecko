use super::draw::{TextureDescriptor, TextureFormat};

/// Decode a GX-format texture from RAM into RGBA8 pixels.
pub fn decode_to_rgba(ram: &[u8], desc: &TextureDescriptor) -> Vec<u8> {
    let w = desc.width as usize;
    let h = desc.height as usize;

    let mut rgba = vec![0u8; w * h * 4];
    match desc.format {
        TextureFormat::I4 => decode_i4(ram, desc, &mut rgba, w, h),
        TextureFormat::I8 => decode_i8(ram, desc, &mut rgba, w, h),
        TextureFormat::IA4 => decode_ia4(ram, desc, &mut rgba, w, h),
        TextureFormat::IA8 => decode_ia8(ram, desc, &mut rgba, w, h),
        TextureFormat::RGB565 => decode_rgb565(ram, desc, &mut rgba, w, h),
        TextureFormat::RGB5A3 => decode_rgb5a3(ram, desc, &mut rgba, w, h),
        TextureFormat::RGBA8 => decode_rgba8(ram, desc, &mut rgba, w, h),
        TextureFormat::CMPR => decode_cmpr(ram, desc, &mut rgba, w, h),
        _ => tracing::error!(?desc.format, "unsupported texture format"),
    }

    rgba
}

/// Compute the number of raw bytes a GX texture occupies in RAM.
pub fn raw_data_size(width: u32, height: u32, format: TextureFormat) -> usize {
    let (block_w, block_h, block_bytes): (u32, u32, u32) = match format {
        TextureFormat::I4 => (8, 8, 32),
        TextureFormat::I8 => (8, 4, 32),
        TextureFormat::IA4 => (8, 4, 32),
        TextureFormat::IA8 => (4, 4, 32),
        TextureFormat::RGB565 => (4, 4, 32),
        TextureFormat::RGB5A3 => (4, 4, 32),
        TextureFormat::RGBA8 => (4, 4, 64),
        TextureFormat::CMPR => (8, 8, 32),
        TextureFormat::CI4 => (8, 8, 32),
        TextureFormat::CI8 => (8, 4, 32),
        TextureFormat::CI14 => (4, 4, 32),
    };
    let blocks_x = width.div_ceil(block_w);
    let blocks_y = height.div_ceil(block_h);
    (blocks_x * blocks_y * block_bytes) as usize
}

fn decode_i4(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 8;
    const BH: usize = 8;
    const BB: usize = 32;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);
    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_i4: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let byte = read_u8_unchecked(src, blk + (ty * BW + tx) / 2);
                        let nibble = if tx & 1 == 0 { byte >> 4 } else { byte & 0x0F };
                        let i = expand_4_to_8(nibble);
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, [i, i, i, i]);
                    }
                }
            }
        }
    }
}

fn decode_i8(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 8;
    const BH: usize = 4;
    const BB: usize = 32;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);
    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_i8: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let i = read_u8_unchecked(src, blk + ty * BW + tx);
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, [i, i, i, i]);
                    }
                }
            }
        }
    }
}

fn decode_ia4(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 8;
    const BH: usize = 4;
    const BB: usize = 32;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);
    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_ia4: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let packed = read_u8_unchecked(src, blk + ty * BW + tx);
                        let a = expand_4_to_8(packed >> 4);
                        let i = expand_4_to_8(packed & 0x0F);
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, [i, i, i, a]);
                    }
                }
            }
        }
    }
}

fn decode_ia8(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 4;
    const BH: usize = 4;
    const BB: usize = 32;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);

    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_ia8: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let off = blk + (ty * BW + tx) * 2;
                        let a = read_u8_unchecked(src, off);
                        let i = read_u8_unchecked(src, off + 1);
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, [i, i, i, a]);
                    }
                }
            }
        }
    }
}

fn decode_rgb565(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 4;
    const BH: usize = 4;
    const BB: usize = 32;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);
    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_rgb565: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let packed = read_be_u16_unchecked(src, blk + (ty * BW + tx) * 2);
                        let pixel = self::rgb565_to_rgba(packed);
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, pixel);
                    }
                }
            }
        }
    }
}

fn decode_rgb5a3(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 4;
    const BH: usize = 4;
    const BB: usize = 32;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);
    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_rgb5a3: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let packed = read_be_u16_unchecked(src, blk + (ty * BW + tx) * 2);
                        let pixel = if packed & 0x8000 != 0 {
                            let r = expand_5_to_8(((packed >> 10) & 0x1F) as u8);
                            let g = expand_5_to_8(((packed >> 5) & 0x1F) as u8);
                            let b = expand_5_to_8((packed & 0x1F) as u8);
                            [r, g, b, 255]
                        } else {
                            let a = expand_3_to_8(((packed >> 12) & 0x7) as u8);
                            let r = expand_4_to_8(((packed >> 8) & 0xF) as u8);
                            let g = expand_4_to_8(((packed >> 4) & 0xF) as u8);
                            let b = expand_4_to_8((packed & 0xF) as u8);
                            [r, g, b, a]
                        };
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, pixel);
                    }
                }
            }
        }
    }
}

fn decode_rgba8(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const BW: usize = 4;
    const BH: usize = 4;
    const BB: usize = 64;

    let bcx = w.div_ceil(BW);
    let bcy = h.div_ceil(BH);
    if desc.ram_addr + bcx * bcy * BB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_rgba8: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let base_y = by * BH;
        let th = BH.min(h - base_y);
        for bx in 0..bcx {
            let base_x = bx * BW;
            let tw = BW.min(w - base_x);
            let blk = desc.ram_addr + (by * bcx + bx) * BB;

            for ty in 0..th {
                for tx in 0..tw {
                    unsafe {
                        let ti = ty * BW + tx;
                        let ar = blk + ti * 2;
                        let gb = blk + 32 + ti * 2;
                        let a = read_u8_unchecked(src, ar);
                        let r = read_u8_unchecked(src, ar + 1);
                        let g = read_u8_unchecked(src, gb);
                        let b = read_u8_unchecked(src, gb + 1);
                        write_pixel(dst, ((base_y + ty) * w + base_x + tx) * 4, [r, g, b, a]);
                    }
                }
            }
        }
    }
}

fn decode_cmpr(ram: &[u8], desc: &TextureDescriptor, rgba: &mut [u8], w: usize, h: usize) {
    const MW: usize = 8;
    const MH: usize = 8;
    const MB: usize = 32;
    const SB: usize = 8;

    let bcx = w.div_ceil(MW);
    let bcy = h.div_ceil(MH);
    if desc.ram_addr + bcx * bcy * MB > ram.len() {
        tracing::warn!(addr = desc.ram_addr, w, h, "decode_cmpr: texture OOB, skipping");
        return;
    }

    let src = ram.as_ptr();
    let dst = rgba.as_mut_ptr();

    for by in 0..bcy {
        let macro_y = by * MH;
        for bx in 0..bcx {
            let macro_x = bx * MW;
            let macro_off = desc.ram_addr + (by * bcx + bx) * MB;

            for si in 0..4usize {
                let sub_off = macro_off + si * SB;
                let sub_x = macro_x + (si & 1) * 4;
                let sub_y = macro_y + (si >> 1) * 4;
                if sub_x >= w || sub_y >= h {
                    continue;
                }
                let tw = 4usize.min(w - sub_x);
                let th = 4usize.min(h - sub_y);

                unsafe {
                    let c0 = read_be_u16_unchecked(src, sub_off);
                    let c1 = read_be_u16_unchecked(src, sub_off + 2);
                    let palette = self::build_dxt1_palette(c0, c1);

                    for row in 0..th {
                        let idx = read_u8_unchecked(src, sub_off + 4 + row);
                        for col in 0..tw {
                            let pi = ((idx >> ((3 - col) * 2)) & 0x03) as usize;
                            write_pixel(dst, ((sub_y + row) * w + sub_x + col) * 4, palette[pi]);
                        }
                    }
                }
            }
        }
    }
}

#[inline(always)]
fn build_dxt1_palette(c0: u16, c1: u16) -> [[u8; 4]; 4] {
    let [r0, g0, b0, _] = self::rgb565_to_rgba(c0);
    let [r1, g1, b1, _] = self::rgb565_to_rgba(c1);

    let mut p = [[0u8; 4]; 4];
    p[0] = [r0, g0, b0, 255];
    p[1] = [r1, g1, b1, 255];

    if c0 > c1 {
        p[2] = [
            ((r0 as u16 * 2 + r1 as u16) / 3) as u8,
            ((g0 as u16 * 2 + g1 as u16) / 3) as u8,
            ((b0 as u16 * 2 + b1 as u16) / 3) as u8,
            255,
        ];
        p[3] = [
            ((r0 as u16 + r1 as u16 * 2) / 3) as u8,
            ((g0 as u16 + g1 as u16 * 2) / 3) as u8,
            ((b0 as u16 + b1 as u16 * 2) / 3) as u8,
            255,
        ];
    } else {
        let avg_r = ((r0 as u16 + r1 as u16) / 2) as u8;
        let avg_g = ((g0 as u16 + g1 as u16) / 2) as u8;
        let avg_b = ((b0 as u16 + b1 as u16) / 2) as u8;
        p[2] = [avg_r, avg_g, avg_b, 255];
        p[3] = [avg_r, avg_g, avg_b, 0];
    }

    p
}

#[inline(always)]
fn expand_3_to_8(v: u8) -> u8 {
    (v << 5) | (v << 2) | (v >> 1)
}

#[inline(always)]
fn expand_4_to_8(v: u8) -> u8 {
    (v << 4) | v
}

#[inline(always)]
fn expand_5_to_8(v: u8) -> u8 {
    (v << 3) | (v >> 2)
}

#[inline(always)]
fn expand_6_to_8(v: u8) -> u8 {
    (v << 2) | (v >> 4)
}

#[inline(always)]
fn rgb565_to_rgba(packed: u16) -> [u8; 4] {
    let r = ((packed >> 11) & 0x1F) as u8;
    let g = ((packed >> 5) & 0x3F) as u8;
    let b = (packed & 0x1F) as u8;
    [expand_5_to_8(r), expand_6_to_8(g), expand_5_to_8(b), 255]
}

#[inline(always)]
unsafe fn write_pixel(dst: *mut u8, offset: usize, pixel: [u8; 4]) {
    unsafe { std::ptr::write(dst.add(offset).cast::<[u8; 4]>(), pixel) };
}

#[inline(always)]
unsafe fn read_u8_unchecked(src: *const u8, offset: usize) -> u8 {
    unsafe { *src.add(offset) }
}

#[inline(always)]
unsafe fn read_be_u16_unchecked(src: *const u8, offset: usize) -> u16 {
    unsafe { u16::from_be_bytes([*src.add(offset), *src.add(offset + 1)]) }
}
