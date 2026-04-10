use crate::BindGroupCacheKey;
use crate::pipeline::PipelineKey;
use crate::triangulate::{self, GpuVertex, align_up};
use crate::{DrawUniforms, FrameUniforms, GxRenderer, helpers, texture};
use encase::{ShaderType as _, UniformBuffer};
use gecko::flipper::gx::draw::{DrawCommands, GxCommand};
use gecko::flipper::gx::regs::{MagFilter, MinFilter, WrapMode};
use glam::{Mat4, UVec4, Vec4};

impl GxRenderer {
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        commands: &DrawCommands,
        ram: &mut [u8],
        target: &wgpu::TextureView,
    ) {
        if commands.commands.is_empty() {
            return;
        }

        self.prepare_resources(device, queue, commands, ram);

        // Render all draws into EFB in one pass, then process copies (readback to RAM + clear).
        // The caller displays from the XFB data written to RAM, not from the EFB directly.
        let (frame_uniform_bytes, draw_call_indices) = self.aggregate_draw_data(device, commands);

        if !self.scratch_draws.is_empty() {
            self.upload_buffers(device, queue, &frame_uniform_bytes);
            let num_draws = self.scratch_draws.len();
            self.execute_render_pass_range(device, queue, commands, &draw_call_indices, 0, num_draws);
        }

        // Blit EFB to screen
        self.blit_efb_to_target(device, queue, target);

        // Process copies (readback EFB to RAM as YUV422 + clear EFB for next frame)
        for cmd in &commands.commands {
            if let GxCommand::CopyEfb(copy) = cmd {
                self.execute_efb_copy(device, queue, copy, ram);
            }
        }
    }

    fn prepare_resources(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, commands: &DrawCommands, ram: &[u8]) {
        for cmd in &commands.commands {
            let GxCommand::Draw(dc) = cmd else { continue };
            for desc in dc.textures.iter().flatten() {
                let key = (desc.ram_addr, desc.width, desc.height, desc.format);
                self.texture_cache.entry(key).or_insert_with(|| {
                    let (tex, view) = texture::upload_texture(device, queue, ram, desc);
                    (tex, view)
                });
                let sampler_key = (desc.wrap_s, desc.wrap_t, desc.mag_filter, desc.min_filter);
                self.sampler_cache.entry(sampler_key).or_insert_with(|| {
                    device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some("gx_sampler"),
                        address_mode_u: helpers::map_wrap_mode(sampler_key.0),
                        address_mode_v: helpers::map_wrap_mode(sampler_key.1),
                        mag_filter: helpers::map_mag_filter(sampler_key.2),
                        min_filter: helpers::map_min_filter(sampler_key.3),
                        ..Default::default()
                    })
                });
            }
            let pipeline_key = PipelineKey::from_draw_call(dc);
            if !self.pipeline_cache.contains_key(&pipeline_key) {
                let pipeline = self.create_pipeline(device, &pipeline_key);
                self.pipeline_cache.insert(pipeline_key, pipeline);
            }
        }

        let fallback_sampler_key = (WrapMode::Clamp, WrapMode::Clamp, MagFilter::Linear, MinFilter::Linear);
        self.sampler_cache.entry(fallback_sampler_key).or_insert_with(|| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("gx_sampler_fallback"),
                ..Default::default()
            })
        });
    }

    fn aggregate_draw_data(&mut self, device: &wgpu::Device, commands: &DrawCommands) -> (Vec<u8>, Vec<usize>) {
        self.scratch_vertices.clear();
        self.scratch_draws.clear();
        self.scratch_uniform_bytes.clear();

        let draw_stride = self.draw_uniform_stride as usize;
        let draw_encase_size = DrawUniforms::min_size().get() as usize;
        let frame_stride = align_up(
            FrameUniforms::min_size().get(),
            device.limits().min_uniform_buffer_offset_alignment as u64,
        ) as usize;
        let frame_encase_size = FrameUniforms::min_size().get() as usize;
        let mut frame_uniform_bytes: Vec<u8> = Vec::new();
        let mut draw_call_indices: Vec<usize> = Vec::new();

        for (dc_idx, cmd) in commands.commands.iter().enumerate() {
            let GxCommand::Draw(dc) = cmd else { continue };
            let prev_len = self.scratch_vertices.len();
            triangulate::triangulate_into(dc, &mut self.scratch_vertices);
            let added = self.scratch_vertices.len() - prev_len;
            if added == 0 {
                continue;
            }

            let mvp = commands.projection * dc.modelview;
            let draw_uniform = DrawUniforms {
                mvp: Mat4::from_cols_array_2d(&mvp.0),
            };

            let start = self.scratch_draws.len() * draw_stride;
            self.scratch_uniform_bytes.resize(start + draw_stride, 0);
            let mut draw_buf = UniformBuffer::new(&mut self.scratch_uniform_bytes[start..start + draw_encase_size]);
            draw_buf.write(&draw_uniform).unwrap();

            let alpha_cmp = dc.bp_alpha_compare;
            let frame_uniform = FrameUniforms {
                tev_color_regs: dc.tev_color_regs.map(Vec4::from),
                tev_konst_colors: dc.tev_konst_colors.map(Vec4::from),
                tev_color_env: pack_u32x16_to_uvec4x4(&dc.tev_color_env.map(|e| e.raw())),
                tev_alpha_env: pack_u32x16_to_uvec4x4(&dc.tev_alpha_env.map(|e| e.raw())),
                tev_orders: pack_u32x16_to_uvec4x4(&dc.tev_orders.map(|o| o.raw())),
                num_tev_stages: dc.num_tev_stages as u32,
                alpha_ref0: alpha_cmp.ref0() as f32 / 255.0,
                alpha_ref1: alpha_cmp.ref1() as f32 / 255.0,
                alpha_comp0: alpha_cmp.comp0() as u32,
                alpha_comp1: alpha_cmp.comp1() as u32,
                alpha_op: alpha_cmp.op() as u32,
                light_colors: dc.light_colors.map(Vec4::from),
                light_cosatt: dc.light_cosatt.map(Vec4::from),
                light_distatt: dc.light_distatt.map(Vec4::from),
                light_pos: dc.light_pos.map(Vec4::from),
                light_dir: dc.light_dir.map(Vec4::from),
                color_ctrl: dc.color_ctrl.raw(),
                alpha_ctrl: dc.alpha_ctrl.raw(),
                ambient_color: Vec4::from(dc.ambient_color),
                material_color: Vec4::from(dc.material_color),
            };

            let fstart = self.scratch_draws.len() * frame_stride;
            frame_uniform_bytes.resize(fstart + frame_stride, 0);
            let mut frame_buf = UniformBuffer::new(&mut frame_uniform_bytes[fstart..fstart + frame_encase_size]);
            frame_buf.write(&frame_uniform).unwrap();

            self.scratch_draws.push((prev_len as u32, added as u32));
            draw_call_indices.push(dc_idx);
        }

        (frame_uniform_bytes, draw_call_indices)
    }

    fn upload_buffers(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, frame_uniform_bytes: &[u8]) {
        let num_draws = self.scratch_draws.len();
        self.ensure_draw_capacity(device, num_draws);

        if self.scratch_vertices.len() > self.vertex_capacity {
            self.vertex_capacity = self.scratch_vertices.len().next_power_of_two();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gx_vertices"),
                size: (self.vertex_capacity * std::mem::size_of::<GpuVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        let frame_stride = align_up(
            FrameUniforms::min_size().get(),
            device.limits().min_uniform_buffer_offset_alignment as u64,
        ) as usize;
        let needed_frame_size = (num_draws * frame_stride) as u64;
        if needed_frame_size > self.frame_uniform_buffer.size() {
            self.frame_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gx_frame_uniforms"),
                size: needed_frame_size,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.bind_group_cache.clear();
        }

        queue.write_buffer(&self.frame_uniform_buffer, 0, frame_uniform_bytes);
        queue.write_buffer(&self.draw_uniform_buffer, 0, &self.scratch_uniform_bytes);
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.scratch_vertices));
    }

    fn execute_render_pass_range(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        commands: &DrawCommands,
        draw_call_indices: &[usize],
        scratch_start: usize,
        scratch_end: usize,
    ) {
        let target_width = crate::EFB_WIDTH;
        let target_height = crate::EFB_HEIGHT;
        let fallback_sampler_key = (WrapMode::Clamp, WrapMode::Clamp, MagFilter::Linear, MinFilter::Linear);
        let frame_stride = align_up(
            FrameUniforms::min_size().get(),
            device.limits().min_uniform_buffer_offset_alignment as u64,
        ) as usize;

        // Pre-build bind groups for each unique texture/sampler configuration.
        for &dc_idx in draw_call_indices {
            let GxCommand::Draw(dc) = &commands.commands[dc_idx] else { continue };
            let mut tex_keys: [Option<_>; 8] = [None; 8];
            let mut sampler_keys: [Option<_>; 8] = [None; 8];

            for slot in 0..8 {
                if let Some(desc) = &dc.textures[slot] {
                    tex_keys[slot] = Some((desc.ram_addr, desc.width, desc.height, desc.format));
                    sampler_keys[slot] = Some((desc.wrap_s, desc.wrap_t, desc.mag_filter, desc.min_filter));
                }
            }

            let bg_key = BindGroupCacheKey { tex_keys, sampler_keys };
            self.bind_group_cache.entry(bg_key).or_insert_with(|| {
                let mut tex_views: [&wgpu::TextureView; 8] = [&self.fallback_view; 8];
                let mut tex_samplers: [&wgpu::Sampler; 8] = [&self.sampler_cache[&fallback_sampler_key]; 8];

                for slot in 0..8 {
                    if let Some(tk) = &tex_keys[slot] {
                        tex_views[slot] = &self.texture_cache[tk].1;
                        tex_samplers[slot] = &self.sampler_cache[&sampler_keys[slot].unwrap()];
                    }
                }

                let entries: [wgpu::BindGroupEntry; 18] = std::array::from_fn(|i| match i {
                    0 => wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.frame_uniform_buffer,
                            offset: 0,
                            size: Some(FrameUniforms::min_size()),
                        }),
                    },
                    1 => wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.draw_uniform_buffer,
                            offset: 0,
                            size: Some(DrawUniforms::min_size()),
                        }),
                    },
                    2..=9 => wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: wgpu::BindingResource::TextureView(tex_views[i - 2]),
                    },
                    _ => wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: wgpu::BindingResource::Sampler(tex_samplers[i - 10]),
                    },
                });

                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.bind_group_layout,
                    entries: &entries,
                })
            });
        }

        let mut encoder = device.create_command_encoder(&Default::default());
        {
            // TODO: lol?
            let needs_initial_clear = self.efb_needs_clear;
            self.efb_needs_clear = false;
            let color_load = if needs_initial_clear {
                wgpu::LoadOp::Clear(wgpu::Color::BLACK)
            } else {
                wgpu::LoadOp::Load
            };
            let depth_load = if needs_initial_clear {
                wgpu::LoadOp::Clear(1.0)
            } else {
                wgpu::LoadOp::Load
            };

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gx_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.efb_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.efb_depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: depth_load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            for (index, (first_vertex, vertex_count)) in self.scratch_draws.iter().copied().enumerate().skip(scratch_start).take(scratch_end - scratch_start) {
                let GxCommand::Draw(dc) = &commands.commands[draw_call_indices[index]] else {
                    continue;
                };
                let pipeline_key = PipelineKey::from_draw_call(dc);
                let pipeline = &self.pipeline_cache[&pipeline_key];
                rpass.set_pipeline(pipeline);

                let mut tex_keys: [Option<_>; 8] = [None; 8];
                let mut sampler_keys: [Option<_>; 8] = [None; 8];
                for slot in 0..8 {
                    if let Some(desc) = &dc.textures[slot] {
                        tex_keys[slot] = Some((desc.ram_addr, desc.width, desc.height, desc.format));
                        sampler_keys[slot] = Some((desc.wrap_s, desc.wrap_t, desc.mag_filter, desc.min_filter));
                    }
                }

                let bg_key = BindGroupCacheKey { tex_keys, sampler_keys };
                let bind_group = &self.bind_group_cache[&bg_key];

                let frame_offset = (index * frame_stride) as u32;
                let draw_offset = (index as u64 * self.draw_uniform_stride) as u32;
                rpass.set_bind_group(0, bind_group, &[frame_offset, draw_offset]);

                // Apply per-draw viewport and scissor, clamped to render target bounds
                let vp = &dc.viewport;
                let vp_x = vp.x.max(0.0);
                let vp_y = vp.y.max(0.0);
                let vp_w = vp.w.max(1.0).min(target_width as f32 - vp_x);
                let vp_h = vp.h.max(1.0).min(target_height as f32 - vp_y);
                rpass.set_viewport(vp_x, vp_y, vp_w, vp_h, vp.min_depth, vp.max_depth);

                let sc = &dc.scissor;
                let sc_w = sc.w.max(1).min(target_width.saturating_sub(sc.x));
                let sc_h = sc.h.max(1).min(target_height.saturating_sub(sc.y));
                rpass.set_scissor_rect(sc.x, sc.y, sc_w, sc_h);

                rpass.draw(first_vertex..first_vertex + vertex_count, 0..1);
            }
        }

        queue.submit([encoder.finish()]);
    }

    fn execute_efb_copy(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        copy: &gecko::flipper::gx::draw::EfbCopyCmd,
        ram: &mut [u8],
    ) {
        let x = copy.src_x.min(crate::EFB_WIDTH);
        let y = copy.src_y.min(crate::EFB_HEIGHT);
        let w = copy.src_w.min(crate::EFB_WIDTH.saturating_sub(x));
        let h = copy.src_h.min(crate::EFB_HEIGHT.saturating_sub(y));

        if w > 0 && h > 0 {
            let bytes_per_row = align_up(w as u64 * 4, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64) as u32;
            let buffer_size = bytes_per_row as u64 * h as u64;

            let staging = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("efb_copy_staging"),
                size: buffer_size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            let mut encoder = device.create_command_encoder(&Default::default());
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.efb_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::default(),
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &staging,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(h),
                    },
                },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            queue.submit([encoder.finish()]);

            let slice = staging.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            let _ = device.poll(wgpu::PollType::wait_indefinitely());

            {
                let data = slice.get_mapped_range();
                if copy.copy_to_xfb {
                    let is_bgra = matches!(
                        self.surface_format,
                        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
                    );
                    self::encode_xfb_yuv422(&data, bytes_per_row, w, h, ram, copy.dest_addr as usize, is_bgra);
                }
                // TODO: tiled texture encode for CopyTex
                drop(data);
            }
            staging.unmap();
        }

        if copy.clear {
            // Clear the entire EFB with the game's clear color.
            // TODO: scoped clear (only the copied region) for proper multi-pass support.
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("efb_clear_after_copy"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.efb_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: copy.clear_color[0] as f64,
                                g: copy.clear_color[1] as f64,
                                b: copy.clear_color[2] as f64,
                                a: copy.clear_color[3] as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.efb_depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(copy.clear_z),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });
            }
            queue.submit([encoder.finish()]);
        }
    }

    fn blit_efb_to_target(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
    ) {
        let mut encoder = _device.create_command_encoder(&Default::default());
        let target_size = target.texture().size();
        let copy_w = crate::EFB_WIDTH.min(target_size.width);
        let copy_h = crate::EFB_HEIGHT.min(target_size.height);

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.efb_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            wgpu::TexelCopyTextureInfo {
                texture: target.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::default(),
            },
            wgpu::Extent3d {
                width: copy_w,
                height: copy_h,
                depth_or_array_layers: 1,
            },
        );

        queue.submit([encoder.finish()]);
    }

    fn ensure_draw_capacity(&mut self, device: &wgpu::Device, count: usize) {
        if count <= self.draw_uniform_capacity {
            return;
        }

        self.draw_uniform_capacity = count.next_power_of_two();
        self.draw_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gx_draw_uniforms"),
            size: self.draw_uniform_stride * self.draw_uniform_capacity as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.bind_group_cache.clear();
    }
}

fn pack_u32x16_to_uvec4x4(data: &[u32; 16]) -> [UVec4; 4] {
    [
        UVec4::new(data[0], data[1], data[2], data[3]),
        UVec4::new(data[4], data[5], data[6], data[7]),
        UVec4::new(data[8], data[9], data[10], data[11]),
        UVec4::new(data[12], data[13], data[14], data[15]),
    ]
}

/// Encode EFB readback pixels as YUV422 into guest RAM (XFB format).
/// Pixel data may be RGBA or BGRA depending on the surface format.
fn encode_xfb_yuv422(
    pixels: &[u8],
    bytes_per_row: u32,
    width: u32,
    height: u32,
    ram: &mut [u8],
    dest: usize,
    is_bgra: bool,
) {
    let row_bytes = width as usize * 2;
    for y in 0..height as usize {
        let src_row = &pixels[y * bytes_per_row as usize..];
        let dst_row_off = dest + y * row_bytes;
        for px_pair in 0..(width as usize / 2) {
            let x = px_pair * 2;
            let s0 = x * 4;
            let s1 = (x + 1) * 4;
            let (r1, g1, b1) = if is_bgra {
                (src_row[s0 + 2] as i32, src_row[s0 + 1] as i32, src_row[s0] as i32)
            } else {
                (src_row[s0] as i32, src_row[s0 + 1] as i32, src_row[s0 + 2] as i32)
            };
            let (r2, g2, b2) = if is_bgra {
                (src_row[s1 + 2] as i32, src_row[s1 + 1] as i32, src_row[s1] as i32)
            } else {
                (src_row[s1] as i32, src_row[s1 + 1] as i32, src_row[s1 + 2] as i32)
            };

            let y1 = ((77 * r1 + 150 * g1 + 29 * b1) / 256).clamp(0, 255) as u8;
            let y2 = ((77 * r2 + 150 * g2 + 29 * b2) / 256).clamp(0, 255) as u8;
            let cb = ((112 * (b1 + b2) - 74 * (g1 + g2) - 38 * (r1 + r2)) / 512 + 128).clamp(0, 255) as u8;
            let cr = ((112 * (r1 + r2) - 94 * (g1 + g2) - 18 * (b1 + b2)) / 512 + 128).clamp(0, 255) as u8;

            let off = dst_row_off + px_pair * 4;
            if off + 3 < ram.len() {
                ram[off] = y1;
                ram[off + 1] = cb;
                ram[off + 2] = y2;
                ram[off + 3] = cr;
            }
        }
    }
}
