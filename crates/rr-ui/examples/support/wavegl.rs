//! Prototype: a 3D "joy division" wavetable view drawn with raw OpenGL inside an egui rect
//! through `egui::PaintCallback` + `egui_glow::CallbackFn`. Shared by the rr-ui `motion_gl`
//! example (eframe) and the gts `plugin_loop` example (egui-baseview, the plugin's exact backend).
//!
//! Techniques shown (all GL 3.2 core / GLSL 150, the context nih_plug_egui creates):
//!  * thick, analytically anti-aliased lines: each segment is a quad expanded in screen space in
//!    the vertex shader, coverage from distance-to-centre in the fragment shader (no MSAA needed)
//!  * depth fade (far layers dimmer), hidden-line removal with opaque fills drawn back to front
//!  * glow: a second, wider, soft ribbon pass with additive blending (no FBO needed)
//!  * the same dither egui applies, so dark gradients do not band
//!  * an optional GPU timer query (GL_ARB_timer_query) for the callback's own cost
//!
//! GL objects are created lazily on first paint, inside the painter's context. They are never
//! deleted explicitly: the context dies with the window. Keep them in per-editor state (not a
//! `static`): a reopened editor gets a NEW context and old handles would be invalid.

use eframe::egui::{self, Rect};
use egui_glow::glow::{self, HasContext as _};
use std::sync::{Arc, Mutex};

pub const LAYERS: usize = 32;
pub const POINTS: usize = 256;

#[derive(Clone, Copy)]
pub struct View {
    pub yaw: f32,
    pub pitch: f32,
    /// Ribbon width in physical px.
    pub width_px: f32,
    /// 0..1 position along the stack that is lit (playhead).
    pub focus: f32,
    /// Glow pass strength, 0 disables the pass.
    pub glow: f32,
    pub color: [f32; 3],
    pub bg: [f32; 3],
    /// Rebuild + upload all vertex data every frame (worst case: morphing waves).
    pub reupload: bool,
}

impl Default for View {
    fn default() -> Self {
        Self { yaw: 0.0, pitch: 0.55, width_px: 1.6, focus: 0.5, glow: 0.35, color: [0.35, 0.9, 1.0], bg: [0.03, 0.05, 0.07], reupload: false }
    }
}

pub struct Shared {
    /// LAYERS x POINTS samples in -1..1, layer 0 = far.
    pub waves: Vec<f32>,
    pub version: u64,
    pub view: View,
    gpu: Option<Gpu>,
    uploaded_version: u64,
    /// GPU ms of every completed timer query (empty = unsupported).
    pub gpu_samples: Vec<f32>,
    /// CPU ms spent inside the callback (vertex build + upload + draw submission).
    pub cpu_samples: Vec<f32>,
    pub gl_info: String,
}

struct Gpu {
    line: glow::Program,
    fill: glow::Program,
    line_vao: glow::VertexArray,
    line_vbo: glow::Buffer,
    #[allow(dead_code)] // kept alive: the index buffer is part of line_vao state
    line_ibo: glow::Buffer,
    fill_vao: glow::VertexArray,
    fill_vbo: glow::Buffer,
    query: Option<glow::Query>,
    pending: bool,
}

#[derive(Clone)]
pub struct WaveGl(pub Arc<Mutex<Shared>>);

impl WaveGl {
    pub fn new(waves: Vec<f32>) -> Self {
        assert_eq!(waves.len(), LAYERS * POINTS);
        Self(Arc::new(Mutex::new(Shared {
            waves,
            version: 1,
            view: View::default(),
            gpu: None,
            uploaded_version: 0,
            gpu_samples: vec![],
            cpu_samples: vec![],
            gl_info: String::new(),
        })))
    }

    /// Adds the callback shape to `painter`, covering `rect`.
    pub fn show(&self, painter: &egui::Painter, rect: Rect) {
        let shared = self.0.clone();
        let cb = egui_glow::CallbackFn::new(move |info, p| {
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            s.paint(&info, p.gl());
        });
        painter.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
    }
}

const LINE_VS: &str = r#"#version 150
in vec3 a_p0; in vec3 a_p1; in vec2 a_corner;
uniform mat4 u_mvp; uniform vec2 u_vp; uniform float u_halfw; uniform float u_layer_t;
out float v_d; out float v_t;
void main() {
    vec4 c0 = u_mvp * vec4(a_p0, 1.0);
    vec4 c1 = u_mvp * vec4(a_p1, 1.0);
    vec2 s0 = c0.xy / c0.w * u_vp * 0.5;
    vec2 s1 = c1.xy / c1.w * u_vp * 0.5;
    vec2 dir = s1 - s0;
    dir = dot(dir, dir) > 1e-8 ? normalize(dir) : vec2(1.0, 0.0);
    vec2 nrm = vec2(-dir.y, dir.x);
    float reach = u_halfw + 1.0;                       // +1 px AA fringe
    vec4 c = mix(c0, c1, a_corner.x);
    vec2 off = nrm * a_corner.y * reach + dir * (a_corner.x * 2.0 - 1.0) * reach * 0.5;
    gl_Position = c + vec4(off / (u_vp * 0.5) * c.w, 0.0, 0.0);
    v_d = a_corner.y * reach;
    v_t = u_layer_t;
}"#;

const LINE_FS: &str = r#"#version 150
in float v_d; in float v_t;
uniform vec3 u_color; uniform float u_halfw; uniform float u_focus; uniform float u_soft; uniform float u_glow;
out vec4 f;
float ign(vec2 n){ return fract(52.9829189 * fract(0.06711056 * n.x + 0.00583715 * n.y)); }
void main() {
    float ad = abs(v_d);
    float a;
    if (u_soft > 0.5) { float s = u_halfw * 0.5; a = exp(-(ad * ad) / (s * s)) * 0.35 * u_glow; }
    else              { a = clamp(u_halfw + 0.5 - ad, 0.0, 1.0); }
    float fade = mix(0.18, 1.0, v_t);                      // far layers dimmer
    float lit = exp(-pow((v_t - u_focus) / 0.06, 2.0));    // playhead highlight
    vec3 col = u_color * fade * (0.75 + 1.1 * lit) + vec3(0.5) * lit * 0.35;
    float alpha = a * (u_soft > 0.5 ? 1.0 : mix(0.35, 1.0, v_t));
    vec3 rgb = col * alpha;                                 // premultiplied, gamma space like egui
    rgb += (ign(gl_FragCoord.xy) - 0.5) * 0.95 / 255.0 * alpha;
    f = vec4(rgb, u_soft > 0.5 ? 0.0 : alpha);
}"#;

const FILL_VS: &str = r#"#version 150
in vec3 a_pos; in float a_kind;
uniform mat4 u_mvp; uniform float u_layer_t;
out float v_kind; out float v_t;
void main(){ gl_Position = u_mvp * vec4(a_pos, 1.0); v_kind = a_kind; v_t = u_layer_t; }"#;

const FILL_FS: &str = r#"#version 150
in float v_kind; in float v_t;
uniform vec3 u_bg; uniform vec3 u_color;
out vec4 f;
float ign(vec2 n){ return fract(52.9829189 * fract(0.06711056 * n.x + 0.00583715 * n.y)); }
void main(){
    float fade = mix(0.18, 1.0, v_t);
    vec3 rgb = u_bg + u_color * 0.07 * fade * (1.0 - v_kind);   // faint tint under the curve
    rgb += (ign(gl_FragCoord.xy) - 0.5) * 0.95 / 255.0;
    f = vec4(rgb, 1.0);                                          // opaque: hides layers behind
}"#;

fn compile(gl: &glow::Context, vs: &str, fs: &str, attribs: &[(&str, u32)]) -> glow::Program {
    unsafe {
        let p = gl.create_program().expect("program");
        for (kind, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
            let s = gl.create_shader(kind).expect("shader");
            gl.shader_source(s, src);
            gl.compile_shader(s);
            assert!(gl.get_shader_compile_status(s), "shader error: {}", gl.get_shader_info_log(s));
            gl.attach_shader(p, s);
        }
        for (n, loc) in attribs {
            gl.bind_attrib_location(p, *loc, n);
        }
        gl.link_program(p);
        assert!(gl.get_program_link_status(p), "link error: {}", gl.get_program_info_log(p));
        p
    }
}

type M4 = [f32; 16];

/// Column-major 4x4 product `a * b`.
fn mul(a: &M4, b: &M4) -> M4 {
    let mut r = [0.0; 16];
    for c in 0..4 {
        for row in 0..4 {
            r[c * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[c * 4 + k]).sum();
        }
    }
    r
}

fn perspective(fovy: f32, aspect: f32, n: f32, f: f32) -> M4 {
    let t = 1.0 / (fovy / 2.0).tan();
    let mut m = [0.0; 16];
    m[0] = t / aspect;
    m[5] = t;
    m[10] = (f + n) / (n - f);
    m[11] = -1.0;
    m[14] = 2.0 * f * n / (n - f);
    m
}

fn look_at(eye: [f32; 3], at: [f32; 3]) -> M4 {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let f = norm(sub(at, eye));
    let s = norm(cross(f, [0.0, 1.0, 0.0]));
    let u = cross(s, f);
    [s[0], u[0], -f[0], 0.0, s[1], u[1], -f[1], 0.0, s[2], u[2], -f[2], 0.0, -dot(s, eye), -dot(u, eye), dot(f, eye), 1.0]
}

/// World: x in -1..1, y = wave * AMP, z in -1..1 (layer 0 far at -1).
pub const AMP: f32 = 0.32;
pub const BASE: f32 = -0.45;

pub fn z_of(l: usize) -> f32 {
    (l as f32 / (LAYERS - 1) as f32) * 2.0 - 1.0
}

/// The matrix both the GL view and the CPU fallback use.
pub fn camera(v: &View, aspect: f32) -> [f32; 16] {
    let proj = perspective(0.75, aspect, 0.1, 20.0);
    let (sy, cy) = v.yaw.sin_cos();
    let (sp, cp) = v.pitch.sin_cos();
    let d = 3.1;
    let eye = [d * cp * sy, d * sp, d * cp * cy];
    mul(&proj, &look_at(eye, [0.0, -0.12, 0.0]))
}

/// Projects a world point to NDC (x, y, w).
pub fn project(m: &[f32; 16], p: [f32; 3]) -> (f32, f32, f32) {
    let x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
    let y = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
    let w = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
    (x / w, y / w, w)
}

impl Shared {
    fn build_line(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(LAYERS * (POINTS - 1) * 4 * 8);
        for l in 0..LAYERS {
            let z = z_of(l);
            for i in 0..POINTS - 1 {
                let p0 = [i as f32 / (POINTS - 1) as f32 * 2.0 - 1.0, self.waves[l * POINTS + i] * AMP, z];
                let p1 = [(i + 1) as f32 / (POINTS - 1) as f32 * 2.0 - 1.0, self.waves[l * POINTS + i + 1] * AMP, z];
                for (cx, cy) in [(0.0, -1.0), (0.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                    v.extend_from_slice(&p0);
                    v.extend_from_slice(&p1);
                    v.extend_from_slice(&[cx, cy]);
                }
            }
        }
        v
    }

    fn build_fill(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(LAYERS * POINTS * 2 * 4);
        for l in 0..LAYERS {
            let z = z_of(l);
            for i in 0..POINTS {
                let x = i as f32 / (POINTS - 1) as f32 * 2.0 - 1.0;
                v.extend_from_slice(&[x, self.waves[l * POINTS + i] * AMP, z, 0.0]);
                v.extend_from_slice(&[x, BASE, z, 1.0]);
            }
        }
        v
    }

    fn init(&mut self, gl: &glow::Context) {
        unsafe {
            let line = compile(gl, LINE_VS, LINE_FS, &[("a_p0", 0), ("a_p1", 1), ("a_corner", 2)]);
            let fill = compile(gl, FILL_VS, FILL_FS, &[("a_pos", 0), ("a_kind", 1)]);
            let line_vao = gl.create_vertex_array().unwrap();
            let line_vbo = gl.create_buffer().unwrap();
            let line_ibo = gl.create_buffer().unwrap();
            gl.bind_vertex_array(Some(line_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(line_vbo));
            for (loc, size, off) in [(0, 3, 0), (1, 3, 12), (2, 2, 24)] {
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, size, glow::FLOAT, false, 32, off);
            }
            // Static index buffer (part of the VAO state): two triangles per segment quad.
            let mut idx: Vec<u16> = Vec::with_capacity(LAYERS * (POINTS - 1) * 6);
            for k in 0..(LAYERS * (POINTS - 1)) as u16 {
                let b = k * 4;
                idx.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 1, b + 3]);
            }
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(line_ibo));
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, bytes(&idx), glow::STATIC_DRAW);

            let fill_vao = gl.create_vertex_array().unwrap();
            let fill_vbo = gl.create_buffer().unwrap();
            gl.bind_vertex_array(Some(fill_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(fill_vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 1, glow::FLOAT, false, 16, 12);
            gl.bind_vertex_array(None);

            let has_q = gl.supported_extensions().contains("GL_ARB_timer_query") || {
                let v = gl.version();
                v.major > 3 || (v.major == 3 && v.minor >= 3)
            };
            let query = if has_q { gl.create_query().ok() } else { None };
            self.gl_info = format!(
                "{} | {} | GL {} | timer_query {}",
                gl.get_parameter_string(glow::RENDERER),
                gl.get_parameter_string(glow::VENDOR),
                gl.get_parameter_string(glow::VERSION),
                query.is_some()
            );
            self.gpu = Some(Gpu { line, fill, line_vao, line_vbo, line_ibo, fill_vao, fill_vbo, query, pending: false });
            self.uploaded_version = 0;
        }
    }

    fn upload(&mut self, gl: &glow::Context) {
        let line = self.build_line();
        let fill = self.build_fill();
        let g = self.gpu.as_ref().unwrap();
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.line_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&line), glow::DYNAMIC_DRAW);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.fill_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes(&fill), glow::DYNAMIC_DRAW);
        }
        self.uploaded_version = self.version;
    }

    fn paint(&mut self, info: &egui::PaintCallbackInfo, gl: &Arc<glow::Context>) {
        let t0 = std::time::Instant::now();
        let gl: &glow::Context = gl;
        if self.gpu.is_none() {
            self.init(gl);
        }
        if self.uploaded_version != self.version || self.view.reupload {
            self.upload(gl);
        }
        let v = self.view;
        let vp = info.viewport_in_pixels();
        let (w, h) = (vp.width_px as f32, vp.height_px as f32);
        let mvp = camera(&v, w / h.max(1.0));
        let mut g = self.gpu.take().unwrap();
        unsafe {
            // GPU timer: one query in flight; harvest its result once available.
            let mut started = false;
            if let Some(q) = g.query {
                if g.pending && gl.get_query_parameter_u32(q, glow::QUERY_RESULT_AVAILABLE) != 0 {
                    self.gpu_samples.push(gl.get_query_parameter_u32(q, glow::QUERY_RESULT) as f32 / 1e6);
                    g.pending = false;
                }
                if !g.pending {
                    gl.begin_query(glow::TIME_ELAPSED, q);
                    started = true;
                }
            }
            gl.use_program(Some(g.fill));
            let u = |p: glow::Program, n: &str| gl.get_uniform_location(p, n);
            gl.uniform_matrix_4_f32_slice(u(g.fill, "u_mvp").as_ref(), false, &mvp);
            gl.uniform_3_f32(u(g.fill, "u_color").as_ref(), v.color[0], v.color[1], v.color[2]);
            gl.uniform_3_f32(u(g.fill, "u_bg").as_ref(), v.bg[0], v.bg[1], v.bg[2]);
            let u_t_fill = u(g.fill, "u_layer_t");
            gl.use_program(Some(g.line));
            gl.uniform_matrix_4_f32_slice(u(g.line, "u_mvp").as_ref(), false, &mvp);
            gl.uniform_2_f32(u(g.line, "u_vp").as_ref(), w, h);
            gl.uniform_3_f32(u(g.line, "u_color").as_ref(), v.color[0], v.color[1], v.color[2]);
            gl.uniform_1_f32(u(g.line, "u_focus").as_ref(), v.focus);
            let (u_t_line, u_halfw, u_soft, u_glow) = (u(g.line, "u_layer_t"), u(g.line, "u_halfw"), u(g.line, "u_soft"), u(g.line, "u_glow"));
            let per_layer = ((POINTS - 1) * 6) as i32;
            for l in 0..LAYERS {
                let t = l as f32 / (LAYERS - 1) as f32;
                // 1. opaque fill hides what is behind
                gl.use_program(Some(g.fill));
                gl.uniform_1_f32(u_t_fill.as_ref(), t);
                gl.bind_vertex_array(Some(g.fill_vao));
                gl.draw_arrays(glow::TRIANGLE_STRIP, (l * POINTS * 2) as i32, (POINTS * 2) as i32);
                // 2. glow (additive soft ribbon), then the crisp line
                gl.use_program(Some(g.line));
                gl.uniform_1_f32(u_t_line.as_ref(), t);
                gl.bind_vertex_array(Some(g.line_vao));
                if v.glow > 0.0 {
                    gl.blend_func(glow::ONE, glow::ONE);
                    gl.uniform_1_f32(u_soft.as_ref(), 1.0);
                    gl.uniform_1_f32(u_glow.as_ref(), v.glow);
                    gl.uniform_1_f32(u_halfw.as_ref(), 7.0);
                    gl.draw_elements(glow::TRIANGLES, per_layer, glow::UNSIGNED_SHORT, l as i32 * per_layer * 2);
                    gl.blend_func_separate(glow::ONE, glow::ONE_MINUS_SRC_ALPHA, glow::ONE_MINUS_DST_ALPHA, glow::ONE);
                }
                gl.uniform_1_f32(u_soft.as_ref(), 0.0);
                gl.uniform_1_f32(u_halfw.as_ref(), v.width_px * 0.5);
                gl.draw_elements(glow::TRIANGLES, per_layer, glow::UNSIGNED_SHORT, l as i32 * per_layer * 2);
            }
            gl.bind_vertex_array(None);
            gl.use_program(None);
            if started {
                gl.end_query(glow::TIME_ELAPSED);
                g.pending = true;
            }
        }
        self.gpu = Some(g);
        self.cpu_samples.push(t0.elapsed().as_secs_f32() * 1e3);
    }
}

fn bytes<T>(v: &[T]) -> &[u8] {
    // SAFETY: plain f32 / u16 slices.
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

/// Test waves: layer l morphs from a sine toward a harmonic-rich saw-ish shape.
pub fn demo_waves() -> Vec<f32> {
    let mut w = vec![0.0; LAYERS * POINTS];
    for l in 0..LAYERS {
        let m = l as f32 / (LAYERS - 1) as f32;
        let mut peak = 1e-6f32;
        for i in 0..POINTS {
            let x = i as f32 / POINTS as f32;
            let mut s = 0.0;
            for h in 1..=(2 + (m * 14.0) as usize) {
                let hh = h as f32;
                s += (std::f32::consts::TAU * hh * x + m * hh * 0.7).sin() / hh.powf(1.0 + (1.0 - m));
            }
            w[l * POINTS + i] = s;
            peak = peak.max(s.abs());
        }
        for i in 0..POINTS {
            w[l * POINTS + i] /= peak;
        }
    }
    w
}

/// Same scene drawn with plain egui shapes (the fallback): per layer an opaque mesh strip for the
/// fill and a `Shape::line` polyline with feathered stroke. Returns nothing; shapes go to `painter`.
pub fn paint_cpu(painter: &egui::Painter, rect: Rect, waves: &[f32], v: &View, with_fill: bool) {
    use egui::epaint::{Mesh, Vertex, WHITE_UV};
    use egui::{pos2, Color32, Shape, Stroke};
    let m = camera(v, rect.width() / rect.height());
    let to_screen = |p: [f32; 3]| {
        let (x, y, _) = project(&m, p);
        pos2(rect.center().x + x * rect.width() * 0.5, rect.center().y - y * rect.height() * 0.5)
    };
    let col = |t: f32, k: f32| {
        let fade = 0.18 + 0.82 * t;
        let c = |c: f32| ((c * fade * k).clamp(0.0, 1.0) * 255.0) as u8;
        Color32::from_rgb(c(v.color[0]), c(v.color[1]), c(v.color[2]))
    };
    let bg = Color32::from_rgb((v.bg[0] * 255.0) as u8, (v.bg[1] * 255.0) as u8, (v.bg[2] * 255.0) as u8);
    for l in 0..LAYERS {
        let t = l as f32 / (LAYERS - 1) as f32;
        let z = z_of(l);
        let pts: Vec<_> = (0..POINTS).map(|i| to_screen([i as f32 / (POINTS - 1) as f32 * 2.0 - 1.0, waves[l * POINTS + i] * AMP, z])).collect();
        if with_fill {
            let mut mesh = Mesh::default();
            for (i, p) in pts.iter().enumerate() {
                let x = i as f32 / (POINTS - 1) as f32 * 2.0 - 1.0;
                mesh.vertices.push(Vertex { pos: *p, uv: WHITE_UV, color: bg });
                mesh.vertices.push(Vertex { pos: to_screen([x, BASE, z]), uv: WHITE_UV, color: bg });
                if i > 0 {
                    let b = (i as u32 - 1) * 2;
                    mesh.indices.extend_from_slice(&[b, b + 1, b + 2, b + 2, b + 1, b + 3]);
                }
            }
            painter.add(Shape::mesh(mesh));
        }
        let lit = (-((t - v.focus) / 0.06).powi(2)).exp();
        painter.add(Shape::line(pts, Stroke::new(1.3_f32, col(t, 0.75 + 1.1 * lit))));
    }
}
