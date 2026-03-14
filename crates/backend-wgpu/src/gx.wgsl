struct Uniforms {
    mvp: mat4x4<f32>,
    tev_color_source: u32,  // 0 = vertex color, 1 = texture
    alpha_ref0: f32,
    alpha_ref1: f32,
    alpha_comp0: u32,       // CompareFunc (0=Never..7=Always)
    alpha_comp1: u32,
    alpha_op: u32,          // AlphaOp (0=AND, 1=OR, 2=XOR, 3=XNOR)
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var tex: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex0: vec2<f32>,
};

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex0: vec2<f32>,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip_pos = uniforms.mvp * vec4<f32>(in.position, 1.0);
    // Remap depth: GameCube/OpenGL uses [-1,1], wgpu uses [0,1]
    out.clip_pos.z = out.clip_pos.z * 0.5 + out.clip_pos.w * 0.5;
    out.color = in.color;
    out.tex0 = in.tex0;
    return out;
}

fn alpha_compare(a: f32, ref_val: f32, func: u32) -> bool {
    switch func {
        case 0u: { return false; }             // Never
        case 1u: { return a < ref_val; }       // Less
        case 2u: { return a == ref_val; }      // Equal
        case 3u: { return a <= ref_val; }      // LessEqual
        case 4u: { return a > ref_val; }       // Greater
        case 5u: { return a != ref_val; }      // NotEqual
        case 6u: { return a >= ref_val; }      // GreaterEqual
        case 7u: { return true; }              // Always
        default: { return true; }
    }
}

fn alpha_combine(a: bool, b: bool, op: u32) -> bool {
    switch op {
        case 0u: { return a && b; }            // AND
        case 1u: { return a || b; }            // OR
        case 2u: { return a != b; }            // XOR
        case 3u: { return a == b; }            // XNOR
        default: { return true; }
    }
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var color: vec4<f32>;
    if uniforms.tev_color_source == 1u {
        color = textureSample(tex, tex_sampler, in.tex0);
    } else {
        color = in.color;
    }

    let c0 = alpha_compare(color.a, uniforms.alpha_ref0, uniforms.alpha_comp0);
    let c1 = alpha_compare(color.a, uniforms.alpha_ref1, uniforms.alpha_comp1);
    if !alpha_combine(c0, c1, uniforms.alpha_op) {
        discard;
    }

    return color;
}
