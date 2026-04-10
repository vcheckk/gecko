use super::constants::{
    DEPTH_24_BIT_MAX, XF_PROJECTION_BASE, XF_PROJECTION_END, XF_VIEWPORT_BASE, XF_VIEWPORT_END, XF_VIEWPORT_OFFSET_X,
    XF_VIEWPORT_OFFSET_Y, XF_VIEWPORT_OFFSET_Z, XF_VIEWPORT_SCALE_X, XF_VIEWPORT_SCALE_Y, XF_VIEWPORT_SCALE_Z,
};
use super::math::Vec3;
use super::{GraphicsProcessor, draw};

impl GraphicsProcessor {
    pub fn xf_transform_3x4(&self, base: usize, v: [f32; 3]) -> Vec3 {
        Vec3(
            f32::from_bits(self.xf_mem[base]) * v[0]
                + f32::from_bits(self.xf_mem[base + 1]) * v[1]
                + f32::from_bits(self.xf_mem[base + 2]) * v[2]
                + f32::from_bits(self.xf_mem[base + 3]),
            f32::from_bits(self.xf_mem[base + 4]) * v[0]
                + f32::from_bits(self.xf_mem[base + 5]) * v[1]
                + f32::from_bits(self.xf_mem[base + 6]) * v[2]
                + f32::from_bits(self.xf_mem[base + 7]),
            f32::from_bits(self.xf_mem[base + 8]) * v[0]
                + f32::from_bits(self.xf_mem[base + 9]) * v[1]
                + f32::from_bits(self.xf_mem[base + 10]) * v[2]
                + f32::from_bits(self.xf_mem[base + 11]),
        )
    }

    pub fn rebuild_viewport(&mut self) {
        let scale_x = f32::from_bits(self.xf_mem[XF_VIEWPORT_SCALE_X]);
        let scale_y = f32::from_bits(self.xf_mem[XF_VIEWPORT_SCALE_Y]);
        let scale_z = f32::from_bits(self.xf_mem[XF_VIEWPORT_SCALE_Z]);
        let offset_x = f32::from_bits(self.xf_mem[XF_VIEWPORT_OFFSET_X]);
        let offset_y = f32::from_bits(self.xf_mem[XF_VIEWPORT_OFFSET_Y]);
        let offset_z = f32::from_bits(self.xf_mem[XF_VIEWPORT_OFFSET_Z]);

        // Decode: scale_x = wd*0.5, scale_y = (-ht)*0.5
        // offset_x = (xOrig + wd*0.5) + 342, offset_y = (yOrig + ht*0.5) + 342
        let w = scale_x * 2.0;
        let h = scale_y * -2.0;
        let x = offset_x - 342.0 - scale_x;
        let y = offset_y - 342.0 + scale_y; // +scale_y because scale_y is negative

        let far = offset_z / DEPTH_24_BIT_MAX;
        let near = far - scale_z / DEPTH_24_BIT_MAX;

        self.cur_viewport = draw::Viewport {
            x,
            y,
            w,
            h,
            min_depth: near,
            max_depth: far,
        };
    }

    pub fn rebuild_projection(&mut self) {
        let pm1 = f32::from_bits(self.xf_mem[XF_PROJECTION_BASE + 0]);
        let pm2 = f32::from_bits(self.xf_mem[XF_PROJECTION_BASE + 1]);
        let pm3 = f32::from_bits(self.xf_mem[XF_PROJECTION_BASE + 2]);
        let pm4 = f32::from_bits(self.xf_mem[XF_PROJECTION_BASE + 3]);
        let pm5 = f32::from_bits(self.xf_mem[XF_PROJECTION_BASE + 4]);
        let pm6 = f32::from_bits(self.xf_mem[XF_PROJECTION_BASE + 5]);
        let proj_type = self.xf_mem[XF_PROJECTION_END];

        self.draw_commands.projection = if proj_type == 0 {
            // Perspective
            draw::Matrix4([
                [pm1, 0.0, 0.0, 0.0],
                [0.0, pm3, 0.0, 0.0],
                [pm2, pm4, pm5, -1.0],
                [0.0, 0.0, pm6, 0.0],
            ])
        } else {
            // Orthographic
            draw::Matrix4([
                [pm1, 0.0, 0.0, 0.0],
                [0.0, pm3, 0.0, 0.0],
                [0.0, 0.0, pm5, 0.0],
                [pm2, pm4, pm6, 1.0],
            ])
        };
    }

    pub fn load_cp(&mut self, data: &[u8]) {
        let idx = data[0] as usize;
        let val = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);
        self.cp_regs[idx] = val;

        tracing::debug!(
            reg_idx = format!("{idx:02X}"),
            value = format!("{val:08X}"),
            "CP register write"
        );
    }

    pub fn load_xf(&mut self, data: &[u8]) {
        let length = u16::from_be_bytes([data[0], data[1]]) as usize;
        let addr = u16::from_be_bytes([data[2], data[3]]) as usize;
        let n = length + 1;
        let end = addr + n;

        for i in 0..n {
            let offset = 4 + i * 4;
            let val = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
            let reg = addr + i;
            if reg < self.xf_mem.len() {
                self.xf_mem[reg] = val;
            }

            tracing::debug!(
                reg_idx = format!("{reg:04X}"),
                value = format!("{val:08X}"),
                "XF register write"
            );
        }

        // Rebuild projection if the write touched its address range
        // (modelview is resolved lazily at draw call time from the current position matrix slot)
        if addr <= XF_PROJECTION_END && end > XF_PROJECTION_BASE {
            self.rebuild_projection();
        }

        // Rebuild viewport if the write touched its address range
        if addr <= XF_VIEWPORT_END && end > XF_VIEWPORT_BASE {
            self.rebuild_viewport();
        }
    }
}
