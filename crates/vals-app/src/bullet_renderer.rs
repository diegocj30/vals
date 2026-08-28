//! Render instanciado de balas: **un solo draw call** para todas.
//!
//! Es el hito de rendimiento del proyecto, y lo dictaron los numeros de H2, no
//! la intuicion: a 20.000 balas la simulacion costaba 0,245 ms y emitir los
//! draw calls de macroquad 17,11 ms. El render era 70 veces mas caro que todo
//! lo demas junto.
//!
//! La idea: en vez de pedirle a macroquad dos circulos por bala —cada uno
//! teselado en CPU y metido en un batch— se sube **una tabla de instancias** a
//! la GPU y se dibuja el mismo quad N veces. La CPU pasa de generar geometria
//! a copiar 32 bytes por bala.
//!
//! Y la bala no es una textura: la dibuja el fragment shader a partir de la
//! distancia al centro. Cero memoria de texturas, nitidez perfecta a cualquier
//! escala, y el halo y el anillo de las parryables salen de la misma formula.

use macroquad::miniquad::{
    Bindings, BlendFactor, BlendState, BlendValue, BufferId, BufferLayout, BufferSource,
    BufferType, BufferUsage, Equation, Pipeline, PipelineParams, ShaderMeta, ShaderSource,
    UniformBlockLayout, UniformDesc, UniformType, UniformsSource, VertexAttribute, VertexFormat,
    VertexStep,
};
use macroquad::prelude::*;
use macroquad::window::{InternalGlContext, get_internal_gl};
use vals_core::bullets::{BULLET_KINDS, Bullets};

use crate::draw::Layout;

/// Cuanto mas grande es el quad que el radio de la bala.
///
/// El sobrante es donde vive el halo. Tiene que coincidir con la constante del
/// shader, y por eso esta comentado en los dos sitios.
const HALO: f32 = 2.2;

/// Una bala, tal y como la ve la GPU.
///
/// `#[repr(C)]` no es decorativo: este layout es literalmente el que leen los
/// atributos de vertice declarados en la pipeline.
#[repr(C)]
#[derive(Clone, Copy)]
struct Instance {
    pos: [f32; 2],
    radius: f32,
    /// 1.0 si lleva anillo de parryable. Va por instancia porque cambia bala a
    /// bala; el latido, que es global, viaja como uniform.
    ring: f32,
    color: [f32; 4],
}

/// Uniforms del frame.
///
/// El orden importa: el backend de OpenGL los lee empaquetados sin relleno, en
/// el mismo orden en que se declaran en el `ShaderMeta`.
#[repr(C)]
struct Uniforms {
    origin: [f32; 2],
    screen: [f32; 2],
    scale: f32,
    pulse: f32,
}

pub struct BulletRenderer {
    pipeline: Pipeline,
    bindings: Bindings,
    instance_buffer: BufferId,
    capacity: usize,
    instances: Vec<Instance>,
    /// Instancias enviadas en el ultimo frame.
    pub drawn: usize,
    /// Balas descartadas por estar fuera de pantalla.
    pub culled: usize,
}

impl BulletRenderer {
    pub fn new(capacity: usize) -> Self {
        let InternalGlContext {
            quad_context: ctx, ..
        } = unsafe { get_internal_gl() };

        // Un quad centrado en el origen, en [-1, 1]. Es la unica geometria del
        // sistema: todo lo demas son instancias sobre este mismo quad.
        let quad: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&quad),
        );
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );
        // `Stream`: se reescribe entero cada frame.
        let instance_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::empty::<Instance>(capacity),
        );

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                ShaderMeta {
                    images: vec![],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("u_origin", UniformType::Float2),
                            UniformDesc::new("u_screen", UniformType::Float2),
                            UniformDesc::new("u_scale", UniformType::Float1),
                            UniformDesc::new("u_pulse", UniformType::Float1),
                        ],
                    },
                },
            )
            .expect("el shader de balas deberia compilar");

        let pipeline = ctx.new_pipeline(
            &[
                // Buffer 0: el quad, un vertice por vertice.
                BufferLayout::default(),
                // Buffer 1: las instancias, una por bala.
                BufferLayout {
                    step_func: VertexStep::PerInstance,
                    ..Default::default()
                },
            ],
            &[
                VertexAttribute::with_buffer("in_pos", VertexFormat::Float2, 0),
                VertexAttribute::with_buffer("in_inst_pos", VertexFormat::Float2, 1),
                VertexAttribute::with_buffer("in_inst_radius", VertexFormat::Float1, 1),
                VertexAttribute::with_buffer("in_inst_ring", VertexFormat::Float1, 1),
                VertexAttribute::with_buffer("in_inst_color", VertexFormat::Float4, 1),
            ],
            shader,
            PipelineParams {
                // Alpha premultiplicado: el shader ya devuelve el color
                // multiplicado por su alfa, asi que el halo se suma sin
                // repintar el borde y no quedan orlas oscuras al solaparse.
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::One,
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );

        Self {
            pipeline,
            bindings: Bindings {
                vertex_buffers: vec![vertex_buffer, instance_buffer],
                index_buffer,
                images: vec![],
            },
            instance_buffer,
            capacity,
            instances: Vec::with_capacity(capacity),
            drawn: 0,
            culled: 0,
        }
    }

    /// Dibuja todas las balas vivas en una sola llamada.
    ///
    /// `color` decide el color y si lleva anillo a partir del tipo y los flags.
    /// Se pasa como funcion para que la paleta siga viviendo en `draw.rs`: este
    /// modulo sabe de GPU, no de estetica.
    pub fn draw(
        &mut self,
        bullets: &Bullets,
        layout: &Layout,
        pulse: f32,
        color: fn(u8, u8) -> (Color, bool),
    ) {
        self.build(bullets, layout, color);
        if self.instances.is_empty() {
            return;
        }

        let mut gl = unsafe { get_internal_gl() };
        // Sin esto, lo que macroquad tenga pendiente se dibujaria despues de
        // las balas y el orden de capas saldria del reves.
        gl.flush();

        gl.quad_context
            .buffer_update(self.instance_buffer, BufferSource::slice(&self.instances));

        gl.quad_context.apply_pipeline(&self.pipeline);
        gl.quad_context
            .begin_default_pass(macroquad::miniquad::PassAction::Nothing);
        gl.quad_context.apply_bindings(&self.bindings);
        gl.quad_context
            .apply_uniforms(UniformsSource::table(&Uniforms {
                origin: layout.origin_px().into(),
                screen: [screen_width(), screen_height()],
                scale: layout.scale(),
                pulse,
            }));
        // Seis indices (dos triangulos), N instancias. **Un solo draw call.**
        gl.quad_context.draw(0, 6, self.instances.len() as i32);
        gl.quad_context.end_render_pass();
    }

    /// Rellena la tabla de instancias, descartando lo que no se ve.
    fn build(&mut self, bullets: &Bullets, layout: &Layout, color: fn(u8, u8) -> (Color, bool)) {
        self.instances.clear();
        self.culled = 0;

        let (min, max) = layout.visible_bounds();

        for b in bullets.iter_live() {
            let r = BULLET_KINDS[b.kind as usize].draw_radius;
            // Culling contra la zona visible mas el radio del halo: subir a la
            // GPU balas que caen fuera es trabajo pagado a cambio de nada.
            let m = r * HALO;
            if b.pos.x + m < min.x
                || b.pos.x - m > max.x
                || b.pos.y + m < min.y
                || b.pos.y - m > max.y
            {
                self.culled += 1;
                continue;
            }
            if self.instances.len() >= self.capacity {
                break;
            }
            let (c, ring) = color(b.kind, b.flags);
            self.instances.push(Instance {
                pos: [b.pos.x, b.pos.y],
                radius: r,
                ring: if ring { 1.0 } else { 0.0 },
                color: [c.r, c.g, c.b, c.a],
            });
        }
        self.drawn = self.instances.len();
    }
}

const VERTEX: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec2 in_inst_pos;
attribute float in_inst_radius;
attribute float in_inst_ring;
attribute vec4 in_inst_color;

uniform vec2 u_origin;
uniform vec2 u_screen;
uniform float u_scale;

varying lowp vec2 v_uv;
varying lowp vec4 v_color;
varying lowp float v_ring;

void main() {
    // 2.2 = HALO en el codigo Rust. El quad se agranda para dejar sitio al
    // halo, asi que el borde solido de la bala cae en 1/2.2 del quad.
    vec2 logico = in_inst_pos + in_pos * in_inst_radius * 2.2;
    vec2 px = u_origin + logico * u_scale;
    gl_Position = vec4(px.x / u_screen.x * 2.0 - 1.0,
                       1.0 - px.y / u_screen.y * 2.0,
                       0.0, 1.0);
    v_uv = in_pos;
    v_color = in_inst_color;
    v_ring = in_inst_ring;
}
"#;

const FRAGMENT: &str = r#"#version 100
precision mediump float;

varying lowp vec2 v_uv;
varying lowp vec4 v_color;
varying lowp float v_ring;

uniform float u_pulse;

void main() {
    float d = length(v_uv);
    float borde = 1.0 / 2.2;

    // Nucleo solido, con el canto suavizado.
    float nucleo = smoothstep(borde, borde * 0.78, d);
    // Halo que cae rapido: al cubo para que no manche la pantalla cuando hay
    // miles de balas encima.
    float halo = pow(max(0.0, 1.0 - d), 3.0);

    // Anillo de las parryables, latiendo. Va en el shader y no como un draw
    // aparte, que es justo lo que hacia lento el render anterior.
    float anillo = 0.0;
    if (v_ring > 0.5) {
        float rr = borde * (1.42 + 0.30 * u_pulse);
        anillo = smoothstep(0.055, 0.0, abs(d - rr)) * (0.35 + 0.45 * u_pulse);
    }

    float a = clamp(nucleo + halo * 0.28 + anillo, 0.0, 1.0) * v_color.a;

    // El realce se queda en un punto blanco pequeno en el centro, y no se
    // aplica al conjunto. Realzar con el halo entero —que es maximo justo en
    // el centro— lavaba la bala a blanco y borraba su color, que con la
    // pantalla llena es la unica pista de que tipo viene.
    float centro = smoothstep(borde * 0.5, 0.0, d) * 0.6;
    vec3 rgb = clamp(v_color.rgb + vec3(centro + anillo * 0.5), 0.0, 1.0);

    gl_FragColor = vec4(rgb * a, a);
}
"#;
