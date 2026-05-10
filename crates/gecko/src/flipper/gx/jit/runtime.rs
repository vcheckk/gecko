use crate::flipper::gx::GraphicsProcessor;
use std::ffi::c_void;

pub const SYM_APPLY_TEXGENS: &str = "gecko_gx_jit_apply_texgens";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gecko_gx_jit_apply_texgens(
    gp: *const c_void,
    position: *const [f32; 3],
    normal: *const [f32; 3],
    raw_tex: *const [[f32; 2]; 8],
    present_mask: u32,
    tex_mtx: *const [u8; 8],
    out_texcoords: *mut [[f32; 3]; 8],
) {
    let gp = unsafe { &*(gp as *const GraphicsProcessor) };
    let position = unsafe { *position };
    let normal = unsafe { *normal };
    let raw = unsafe { *raw_tex };
    let tex_mtx = unsafe { *tex_mtx };
    let out = unsafe { &mut *out_texcoords };

    let raw_options: [Option<[f32; 2]>; 8] = std::array::from_fn(|i| ((present_mask >> i) & 1 == 1).then(|| raw[i]));

    gp.apply_all_texgens(position, normal, &raw_options, &tex_mtx, out);
}
