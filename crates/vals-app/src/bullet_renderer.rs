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
    /// Hacia donde apunta la bala, unitario. Solo importa en las agujas.
    eje: [f32; 2],
    /// Cuanto se estira a lo largo y a lo ancho de `eje`. (1, 1) es un circulo.
    estira: [f32; 2],
}

/// El eje y el estiramiento de una bala.
///
/// Las agujas eran circulos, y una aguja redonda no dice hacia donde va, que es
/// lo unico que importa de ella. Ahora se alargan en su direccion y se afinan a
/// lo ancho: el area queda parecida y la silueta pasa a ser una flecha. Una
/// aguja parada no tiene direccion, y en vez de dividir entre cero se queda
/// tumbada.
pub(crate) fn estiramiento(kind: u8, vel_x: f32, vel_y: f32) -> ([f32; 2], [f32; 2]) {
    if kind != KIND_AGUJA {
        return ([1.0, 0.0], [1.0, 1.0]);
    }
    let largo = (vel_x * vel_x + vel_y * vel_y).sqrt();
    let eje = if largo > 1e-3 {
        [vel_x / largo, vel_y / largo]
    } else {
        [1.0, 0.0]
    };
    (eje, [AGUJA.0, AGUJA.1])
}

/// Cuanto le queda a una bala por apagarse, en `[0, 1]`: 1 entera, 0 a punto
/// de morir.
///
/// Las balas de vida corta (las patadas del cancan) desaparecian de golpe a
/// media sala, y eso se leia como un fallo. Ahora en su ultimo `APAGADO` de
/// segundo se encogen y se aclaran. Solo es pintura: la bala sigue matando
/// con su radio hasta el ultimo tick, y por eso no se encoge por debajo de el.
pub(crate) fn apagado(ttl: f32) -> f32 {
    (ttl / APAGADO).clamp(0.0, 1.0)
}

/// Segundos que dura el apagado de una bala que muere por `ttl`.
const APAGADO: f32 = 0.25;
/// Opacidad con la que llega al ultimo tick: no a cero, que una bala
/// invisible que aun mata es peor que una que se esfuma.
pub(crate) const OPACIDAD_FINAL: f32 = 0.35;

/// Radio y opacidad con que se pinta una bala, ya contado su apagado.
pub(crate) fn pintura(kind: u8, ttl: f32) -> (f32, f32) {
    let def = &BULLET_KINDS[kind as usize];
    let f = apagado(ttl);
    (
        def.radius + (def.draw_radius - def.radius) * f,
        OPACIDAD_FINAL + (1.0 - OPACIDAD_FINAL) * f,
    )
}

/// El tipo de bala que es una aguja, en `BULLET_KINDS`.
const KIND_AGUJA: u8 = 3;
/// A lo largo y a lo ancho de una aguja, respecto a su radio.
pub(crate) const AGUJA: (f32, f32) = (1.6, 0.8);

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
                VertexAttribute::with_buffer("in_inst_eje", VertexFormat::Float2, 1),
                VertexAttribute::with_buffer("in_inst_estira", VertexFormat::Float2, 1),
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
            let m = r * HALO * AGUJA.0;
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
            let (eje, estira) = estiramiento(b.kind, b.vel.x, b.vel.y);
            let (radius, opacidad) = pintura(b.kind, b.ttl);
            self.instances.push(Instance {
                pos: [b.pos.x, b.pos.y],
                radius,
                ring: if ring { 1.0 } else { 0.0 },
                color: [c.r, c.g, c.b, c.a * opacidad],
                eje,
                estira,
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
attribute vec2 in_inst_eje;
attribute vec2 in_inst_estira;

uniform vec2 u_origin;
uniform vec2 u_screen;
uniform float u_scale;

varying lowp vec2 v_uv;
varying lowp vec4 v_color;
varying lowp float v_ring;

void main() {
    // 2.2 = HALO en el codigo Rust. El quad se agranda para dejar sitio al
    // halo, asi que el borde solido de la bala cae en 1/2.2 del quad.
    // Se estira en el marco de la bala y luego se gira hacia su eje. El
    // fragment sigue viendo el quad sin estirar, asi que el circulo que dibuja
    // sale en pantalla como una elipse alargada en la direccion de la bala.
    vec2 local = in_pos * in_inst_estira;
    vec2 d = in_inst_eje;
    vec2 girado = vec2(local.x * d.x - local.y * d.y, local.x * d.y + local.y * d.x);
    vec2 logico = in_inst_pos + girado * in_inst_radius * 2.2;
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

    // La silueta mide lo mismo que la bala: nada de engordarla, que una bala
    // que se ve mas gorda que lo que te mata es tramposa.
    float silueta = smoothstep(borde, borde * 0.86, d);
    // El nucleo de color ocupa casi toda la silueta, y la tinta es solo un
    // filo en el ultimo 10 %. Hubo un intento con un contorno grueso (un tercio
    // del radio) y fue a peor: sobre la tarima oscura la tinta no se ve, asi
    // que solo encogia la parte brillante y apagaba la bala. Se notaba sin
    // saber que habia cambiado. El filo fino si sirve: separa la bala de los
    // decorados claros, como el sol dorado del club.
    float nucleo = smoothstep(borde * 0.93, borde * 0.84, d);
    // Halo que cae rapido: al cubo para que no manche la pantalla cuando hay
    // miles de balas encima.
    float halo = pow(max(0.0, 1.0 - d), 3.0) * (1.0 - silueta);

    // Anillo de las parryables, latiendo. Va en el shader y no como un draw
    // aparte, que es justo lo que hacia lento el render anterior.
    float anillo = 0.0;
    if (v_ring > 0.5) {
        float rr = borde * (1.42 + 0.30 * u_pulse);
        anillo = smoothstep(0.055, 0.0, abs(d - rr)) * (0.35 + 0.45 * u_pulse);
    }

    float a = clamp(silueta + halo * 0.28 + anillo, 0.0, 1.0) * v_color.a;

    // El realce se queda en un punto blanco pequeno en el centro, y no se
    // aplica al conjunto. Realzar con el halo entero —que es maximo justo en
    // el centro— lavaba la bala a blanco y borraba su color, que con la
    // pantalla llena es la unica pista de que tipo viene.
    float centro = smoothstep(borde * 0.5, 0.0, d) * 0.6;
    vec3 tinta = vec3(0.10, 0.07, 0.08);
    vec3 dentro = mix(tinta, v_color.rgb + vec3(centro), nucleo);
    // Fuera de la silueta solo queda el halo, y el halo es del color de la
    // bala, no de la tinta.
    vec3 rgb = clamp(mix(v_color.rgb, dentro, silueta) + vec3(anillo * 0.5), 0.0, 1.0);

    gl_FragColor = vec4(rgb * a, a);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_las_agujas_se_estiran_y_apuntan_a_donde_van() {
        // Una bala redonda no tiene direccion que ensenar.
        assert_eq!(estiramiento(0, 3.0, 4.0), ([1.0, 0.0], [1.0, 1.0]));
        // Una aguja que baja apunta hacia abajo.
        let (eje, estira) = estiramiento(KIND_AGUJA, 0.0, 250.0);
        assert!((eje[0]).abs() < 1e-6 && (eje[1] - 1.0).abs() < 1e-6);
        assert_eq!(estira, [AGUJA.0, AGUJA.1]);
        // Y una parada no divide entre cero.
        let (eje, _) = estiramiento(KIND_AGUJA, 0.0, 0.0);
        assert!(eje[0].is_finite() && eje[1].is_finite());
    }

    #[test]
    fn la_que_se_muere_se_apaga_sin_quedar_mas_pequena_que_lo_que_mata() {
        for (k, def) in BULLET_KINDS.iter().enumerate() {
            // Con vida de sobra, se pinta como siempre.
            assert_eq!(pintura(k as u8, 30.0), (def.draw_radius, 1.0));
            // En el ultimo tick, al radio de colision y casi transparente.
            let (r, a) = pintura(k as u8, 0.0);
            assert_eq!(r, def.radius);
            assert_eq!(a, OPACIDAD_FINAL);
        }
        // Y a medio apagar, a medias.
        let (r, a) = pintura(0, APAGADO * 0.5);
        assert!(r < BULLET_KINDS[0].draw_radius && r > BULLET_KINDS[0].radius);
        assert!(a < 1.0 && a > OPACIDAD_FINAL);
    }
}
