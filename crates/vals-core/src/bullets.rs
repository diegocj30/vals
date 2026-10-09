//! El almacen de balas. Es el corazon del proyecto.
//!
//! Layout **SoA** (struct of arrays) en vez del AoS habitual: cada atributo
//! vive en su propio `Vec` contiguo. El bucle de movimiento solo toca posicion
//! y velocidad, asi que con SoA cada linea de cache que se trae de RAM viene
//! llena de datos que se van a usar. Con un `Vec<Bullet>` clasico, cada linea
//! arrastraria tambien el ttl, el tipo, los flags y el relleno.
//!
//! Todo se preasigna al construir: **no hay ni una allocation dentro del bucle
//! de juego**, que es justo lo que produce los tirones que arruinan un danmaku.

use glam::Vec2;

use crate::hash::Fnv1a;
use crate::math;
use crate::{ARENA_H, ARENA_W};

/// Capacidad del pool. Se reserva entera de golpe al arrancar.
pub const MAX_BULLETS: usize = 32_768;

/// Margen fuera de la arena antes de matar una bala.
///
/// No se matan justo al cruzar el borde: un patron puede sacar balas y
/// devolverlas, y matarlas en el borde exacto se veria como un parpadeo.
pub const CULL_MARGIN: f32 = 64.0;

/// Propiedades fijas de un tipo de bala.
pub struct BulletKindDef {
    /// Radio de colision.
    pub radius: f32,
    /// Radio con el que se dibuja. Mayor que el de colision a proposito: la
    /// bala se ve mas grande de lo que mata, lo que hace el juego legible sin
    /// hacerlo injusto.
    pub draw_radius: f32,
}

/// Tabla de tipos. `kind` es un indice a esto y no un enum gordo: cabe en un
/// byte y deja la puerta abierta a ordenar por tipo al renderizar.
pub const BULLET_KINDS: [BulletKindDef; 4] = [
    BulletKindDef {
        radius: 4.0,
        draw_radius: 5.5,
    },
    BulletKindDef {
        radius: 7.0,
        draw_radius: 9.0,
    },
    BulletKindDef {
        radius: 12.0,
        draw_radius: 15.0,
    },
    BulletKindDef {
        radius: 2.5,
        draw_radius: 4.0,
    },
];

pub const KIND_SMALL: u8 = 0;
pub const KIND_MEDIUM: u8 = 1;
pub const KIND_LARGE: u8 = 2;
pub const KIND_NEEDLE: u8 = 3;

/// Se puede neutralizar con el parry.
///
/// Estas son las balas que invierten el juego: en vez de esquivarlas te
/// conviene ir a por ellas. Es lo que separa esto de un "esquiva y ya".
pub const FLAG_PARRYABLE: u8 = 1 << 0;
/// Ya se rozo, para no cobrar el graze dos veces por la misma bala.
pub const FLAG_GRAZED: u8 = 1 << 1;

/// Parametros para crear una bala.
#[derive(Clone, Copy, Debug)]
pub struct Spawn {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Aceleracion a lo largo de la velocidad, en u/s^2. Negativa, frena.
    pub accel: f32,
    /// Giro de la velocidad, en rad/s. Es lo que curva los patrones.
    pub spin: f32,
    pub ttl: f32,
    pub kind: u8,
    pub flags: u8,
}

impl Default for Spawn {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            accel: 0.0,
            spin: 0.0,
            ttl: 30.0,
            kind: KIND_SMALL,
            flags: 0,
        }
    }
}

/// Una bala vista desde fuera. Solo para el render y los tests: dentro del
/// pool esta struct nunca se materializa.
#[derive(Clone, Copy, Debug)]
pub struct BulletView {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Segundos que le quedan. Solo se lee: el render apaga las que se van a
    /// morir para que no desaparezcan de golpe.
    pub ttl: f32,
    pub kind: u8,
    pub flags: u8,
}

/// Pool de balas en SoA.
#[derive(Clone, Debug)]
pub struct Bullets {
    pos_x: Vec<f32>,
    pos_y: Vec<f32>,
    vel_x: Vec<f32>,
    vel_y: Vec<f32>,
    accel: Vec<f32>,
    spin: Vec<f32>,
    ttl: Vec<f32>,
    kind: Vec<u8>,
    flags: Vec<u8>,
    alive: Vec<bool>,
    /// Huecos reutilizables.
    free: Vec<u32>,
    /// Un indice por encima del slot vivo mas alto. Los bucles paran aqui en
    /// vez de recorrer los 32k siempre: con 200 balas en pantalla, barrer el
    /// pool entero seria absurdo.
    high_water: usize,
    live: usize,
}

impl Bullets {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pos_x: vec![0.0; cap],
            pos_y: vec![0.0; cap],
            vel_x: vec![0.0; cap],
            vel_y: vec![0.0; cap],
            accel: vec![0.0; cap],
            spin: vec![0.0; cap],
            ttl: vec![0.0; cap],
            kind: vec![0; cap],
            flags: vec![0; cap],
            alive: vec![false; cap],
            // La free-list arranca vacia: mientras queden slots virgenes se
            // tira de `high_water`, que sale mas barato que rellenarla entera.
            free: Vec::with_capacity(cap),
            high_water: 0,
            live: 0,
        }
    }

    pub fn capacity(&self) -> usize {
        self.alive.len()
    }

    pub fn live_count(&self) -> usize {
        self.live
    }

    /// Slots que recorre el bucle de update. Util en el overlay: si se dispara
    /// muy por encima de `live_count`, el pool esta fragmentado.
    pub fn scanned_slots(&self) -> usize {
        self.high_water
    }

    /// Crea una bala. Devuelve `None` si el pool esta lleno.
    ///
    /// Quedarse sin sitio no es un error recuperable a mitad de patron: se
    /// ignora la bala y se sigue. Un frame con una bala de menos es
    /// infinitamente preferible a un panic o a una reasignacion en pleno bucle.
    pub fn spawn(&mut self, s: Spawn) -> Option<u32> {
        let i = match self.free.pop() {
            Some(i) => i as usize,
            None => {
                if self.high_water >= self.capacity() {
                    return None;
                }
                let i = self.high_water;
                self.high_water += 1;
                i
            }
        };

        self.pos_x[i] = s.pos.x;
        self.pos_y[i] = s.pos.y;
        self.vel_x[i] = s.vel.x;
        self.vel_y[i] = s.vel.y;
        self.accel[i] = s.accel;
        self.spin[i] = s.spin;
        self.ttl[i] = s.ttl;
        self.kind[i] = s.kind;
        self.flags[i] = s.flags;
        self.alive[i] = true;
        self.live += 1;
        Some(i as u32)
    }

    pub fn kill(&mut self, i: u32) {
        let i = i as usize;
        if !self.alive[i] {
            return;
        }
        self.alive[i] = false;
        self.free.push(i as u32);
        self.live -= 1;
    }

    /// Mata todas las balas. Es lo que pasa cuando muere el jugador.
    pub fn clear(&mut self) {
        for i in 0..self.high_water {
            self.alive[i] = false;
        }
        self.free.clear();
        self.high_water = 0;
        self.live = 0;
    }

    /// Avanza todas las balas un paso.
    ///
    /// Este es el bucle caliente del juego. Cuando llegue el momento de
    /// optimizar (SIMD, hilos, GPU), es este bucle y no otro.
    pub fn update(&mut self, dt: f32) {
        for i in 0..self.high_water {
            if !self.alive[i] {
                continue;
            }

            let mut vx = self.vel_x[i];
            let mut vy = self.vel_y[i];

            let spin = self.spin[i];
            if spin != 0.0 {
                let (s, c) = math::sin_cos(spin * dt);
                let nx = vx * c - vy * s;
                vy = vx * s + vy * c;
                vx = nx;
            }

            let accel = self.accel[i];
            if accel != 0.0 {
                let len = (vx * vx + vy * vy).sqrt();
                if len > 1e-6 {
                    // Se deja que la velocidad cruce el cero y se invierta:
                    // "sale, frena, se para y vuelve" es un patron clasico del
                    // genero, no un caso degenerado que haya que evitar.
                    let f = 1.0 + accel * dt / len;
                    vx *= f;
                    vy *= f;
                    // Y al invertirse, la aceleracion cambia de signo con
                    // ella: ahora empuja hacia donde va. Sin esto seguia
                    // frenando en la direccion nueva, la velocidad oscilaba
                    // alrededor de cero tick a tick y la bala se quedaba
                    // temblando donde se paro hasta caducar: frenaba, pero
                    // no volvia nunca.
                    if f < 0.0 {
                        self.accel[i] = -accel;
                    }
                }
            }

            self.vel_x[i] = vx;
            self.vel_y[i] = vy;

            let x = self.pos_x[i] + vx * dt;
            let y = self.pos_y[i] + vy * dt;
            self.pos_x[i] = x;
            self.pos_y[i] = y;

            let ttl = self.ttl[i] - dt;
            self.ttl[i] = ttl;

            let fuera = x < -CULL_MARGIN
                || x > ARENA_W + CULL_MARGIN
                || y < -CULL_MARGIN
                || y > ARENA_H + CULL_MARGIN;
            if ttl <= 0.0 || fuera {
                self.alive[i] = false;
                self.free.push(i as u32);
                self.live -= 1;
            }
        }
    }

    /// Tuerce cada bala viva hacia `objetivo`, sin cambiar su velocidad.
    ///
    /// Solo lo usa la serpentina, contra el jefe, y va aparte de `update` a
    /// proposito: el bucle caliente de las balas del jefe no paga ni una
    /// comparacion por un tiro que la mayoria de partidas no lleva.
    pub(crate) fn perseguir(&mut self, objetivo: Vec2) {
        for i in 0..self.high_water {
            if !self.alive[i] {
                continue;
            }
            let p = Vec2::new(self.pos_x[i], self.pos_y[i]);
            let v = crate::equipo::guiar(Vec2::new(self.vel_x[i], self.vel_y[i]), objetivo - p);
            self.vel_x[i] = v.x;
            self.vel_y[i] = v.y;
        }
    }

    /// Primera bala que solapa el circulo `(p, r)`.
    ///
    /// Fuerza bruta sobre arrays contiguos, y a proposito: el jugador es **un
    /// punto** contra N circulos, asi que esto es un barrido lineal que el
    /// compilador puede vectorizar. No hay grid espacial, ni lo habra hasta
    /// que `docs/PERF.md` demuestre que hace falta.
    pub fn hit_circle(&self, p: Vec2, r: f32) -> Option<u32> {
        for i in 0..self.high_water {
            if !self.alive[i] {
                continue;
            }
            let rad = BULLET_KINDS[self.kind[i] as usize].radius + r;
            let dx = self.pos_x[i] - p.x;
            let dy = self.pos_y[i] - p.y;
            // Al cuadrado: una raiz cuadrada por bala y por frame no la paga
            // nadie a 20.000 balas.
            if dx * dx + dy * dy <= rad * rad {
                return Some(i as u32);
            }
        }
        None
    }

    /// Mata todas las balas que solapan el circulo y devuelve cuantas eran.
    ///
    /// A diferencia de [`Bullets::hit_circle`], que para en la primera, aqui
    /// interesan todas: es el impacto de los disparos del jugador contra el
    /// jefe, y cada bala tiene que contar su dano.
    pub fn damage_circle(&mut self, p: Vec2, r: f32) -> u32 {
        let mut n = 0;
        for i in 0..self.high_water {
            if !self.alive[i] {
                continue;
            }
            let rad = BULLET_KINDS[self.kind[i] as usize].radius + r;
            let dx = self.pos_x[i] - p.x;
            let dy = self.pos_y[i] - p.y;
            if dx * dx + dy * dy <= rad * rad {
                self.alive[i] = false;
                self.free.push(i as u32);
                self.live -= 1;
                n += 1;
            }
        }
        n
    }

    /// Neutraliza las balas parryables dentro del circulo. Devuelve cuantas.
    ///
    /// Solo toca las marcadas con [`FLAG_PARRYABLE`]: el parry es una lectura
    /// del patron, no un boton de limpiar pantalla.
    pub fn parry_circle(&mut self, p: Vec2, r: f32) -> u32 {
        let mut n = 0;
        for i in 0..self.high_water {
            if !self.alive[i] || self.flags[i] & FLAG_PARRYABLE == 0 {
                continue;
            }
            let dx = self.pos_x[i] - p.x;
            let dy = self.pos_y[i] - p.y;
            let rad = BULLET_KINDS[self.kind[i] as usize].radius + r;
            if dx * dx + dy * dy <= rad * rad {
                self.alive[i] = false;
                self.free.push(i as u32);
                self.live -= 1;
                n += 1;
            }
        }
        n
    }

    /// Marca como rozadas las balas dentro del circulo y devuelve cuantas son
    /// nuevas.
    ///
    /// La marca es permanente para esa bala: rozar la misma diez veces mientras
    /// pasa a tu lado tiene que pagar una, no diez. Si no, el graze premiaria
    /// quedarse quieto pegado a una bala lenta, que es lo contrario de lo que
    /// se busca.
    pub fn graze_circle(&mut self, p: Vec2, r: f32) -> u32 {
        let mut n = 0;
        for i in 0..self.high_water {
            if !self.alive[i] || self.flags[i] & FLAG_GRAZED != 0 {
                continue;
            }
            let dx = self.pos_x[i] - p.x;
            let dy = self.pos_y[i] - p.y;
            let rad = BULLET_KINDS[self.kind[i] as usize].radius + r;
            if dx * dx + dy * dy <= rad * rad {
                self.flags[i] |= FLAG_GRAZED;
                n += 1;
            }
        }
        n
    }

    pub fn get(&self, i: u32) -> BulletView {
        let i = i as usize;
        BulletView {
            pos: Vec2::new(self.pos_x[i], self.pos_y[i]),
            vel: Vec2::new(self.vel_x[i], self.vel_y[i]),
            ttl: self.ttl[i],
            kind: self.kind[i],
            flags: self.flags[i],
        }
    }

    /// Mezcla el estado de las balas vivas en una huella.
    ///
    /// Se hashean posicion, velocidad, ttl y tipo, pero no `accel` ni `spin`:
    /// esos no cambian nunca despues del spawn, y cualquier divergencia en
    /// ellos se manifiesta en la velocidad al tick siguiente igualmente.
    pub(crate) fn hash_into(&self, h: &mut Fnv1a) {
        h.write_u64(self.live as u64);
        h.write_u64(self.high_water as u64);
        for i in 0..self.high_water {
            if !self.alive[i] {
                continue;
            }
            h.write_u64(i as u64);
            h.write_f32(self.pos_x[i]);
            h.write_f32(self.pos_y[i]);
            h.write_f32(self.vel_x[i]);
            h.write_f32(self.vel_y[i]);
            h.write_f32(self.ttl[i]);
            h.write_u64(u64::from(self.kind[i]));
            h.write_u64(u64::from(self.flags[i]));
        }
    }

    /// Recorre las balas vivas. Solo para el render y los tests.
    pub fn iter_live(&self) -> impl Iterator<Item = BulletView> + '_ {
        self.iter_live_slots().map(|(_, b)| b)
    }

    /// Lo mismo, con el hueco de cada bala. Un hueco que sigue vivo de un tick
    /// al siguiente es la misma bala, asi que esto permite seguirla: es como
    /// `examples/partituras.rs` dibuja la trayectoria entera de cada una.
    pub fn iter_live_slots(&self) -> impl Iterator<Item = (u32, BulletView)> + '_ {
        (0..self.high_water)
            .filter(move |&i| self.alive[i])
            .map(move |i| (i as u32, self.get(i as u32)))
    }
}

impl Default for Bullets {
    fn default() -> Self {
        Self::with_capacity(MAX_BULLETS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DT;

    fn centro() -> Vec2 {
        Vec2::new(ARENA_W * 0.5, ARENA_H * 0.5)
    }

    fn en(p: Vec2) -> Spawn {
        Spawn {
            pos: p,
            ..Default::default()
        }
    }

    #[test]
    fn spawn_y_kill_llevan_la_cuenta() {
        let mut b = Bullets::with_capacity(16);
        assert_eq!(b.live_count(), 0);
        let i = b.spawn(en(centro())).unwrap();
        assert_eq!(b.live_count(), 1);
        b.kill(i);
        assert_eq!(b.live_count(), 0);
        // Matar dos veces no debe descontar dos veces.
        b.kill(i);
        assert_eq!(b.live_count(), 0);
    }

    #[test]
    fn los_slots_se_reutilizan() {
        let mut b = Bullets::with_capacity(4);
        let i = b.spawn(en(centro())).unwrap();
        b.kill(i);
        let j = b.spawn(en(centro())).unwrap();
        assert_eq!(i, j, "deberia haber reciclado el hueco");
        assert_eq!(
            b.scanned_slots(),
            1,
            "y no haber crecido el high water mark"
        );
    }

    #[test]
    fn el_pool_lleno_devuelve_none_en_vez_de_crecer() {
        let mut b = Bullets::with_capacity(3);
        for _ in 0..3 {
            assert!(b.spawn(en(centro())).is_some());
        }
        assert!(b.spawn(en(centro())).is_none());
        assert_eq!(b.capacity(), 3, "la capacidad no puede cambiar en caliente");
    }

    #[test]
    fn las_balas_se_mueven_en_linea_recta() {
        let mut b = Bullets::with_capacity(4);
        b.spawn(Spawn {
            pos: centro(),
            vel: Vec2::new(60.0, 0.0),
            ..Default::default()
        });
        for _ in 0..60 {
            b.update(DT);
        }
        let v = b.iter_live().next().unwrap();
        assert!((v.pos.x - (centro().x + 60.0)).abs() < 0.5);
        assert!((v.pos.y - centro().y).abs() < 0.001);
    }

    #[test]
    fn el_spin_curva_la_trayectoria_sin_cambiar_el_modulo() {
        let mut b = Bullets::with_capacity(4);
        b.spawn(Spawn {
            pos: centro(),
            vel: Vec2::new(100.0, 0.0),
            spin: 1.0,
            ..Default::default()
        });
        for _ in 0..30 {
            b.update(DT);
        }
        let v = b.iter_live().next().unwrap();
        assert!(v.vel.y.abs() > 1.0, "deberia haber girado");
        assert!(
            (v.vel.length() - 100.0).abs() < 0.01,
            "girar no debe cambiar la rapidez"
        );
    }

    #[test]
    fn la_aceleracion_negativa_frena_y_devuelve_la_bala() {
        let mut b = Bullets::with_capacity(4);
        b.spawn(Spawn {
            pos: centro(),
            vel: Vec2::new(0.0, -200.0),
            accel: -400.0,
            ..Default::default()
        });
        // Frena en medio segundo y vuelve acelerando: al segundo y medio va
        // hacia abajo a unos 400. Se miran dos ticks seguidos porque el fallo
        // de antes era justo ese, una velocidad que cambiaba de signo en cada
        // tick y pasaba el test o no segun el tick en que se mirase.
        for t in 0..92 {
            b.update(DT);
            if t >= 90 {
                let v = b.iter_live().next().unwrap();
                assert!(
                    v.vel.y > 300.0,
                    "deberia volver acelerando, va a {}",
                    v.vel.y
                );
            }
        }
    }

    #[test]
    fn las_balas_mueren_al_salirse_y_al_caducar() {
        let mut b = Bullets::with_capacity(8);
        // Se va por arriba.
        b.spawn(Spawn {
            pos: Vec2::new(ARENA_W * 0.5, 10.0),
            vel: Vec2::new(0.0, -600.0),
            ..Default::default()
        });
        // Quieta, pero con ttl corto.
        b.spawn(Spawn {
            pos: centro(),
            ttl: 0.5,
            ..Default::default()
        });
        assert_eq!(b.live_count(), 2);

        for _ in 0..60 {
            b.update(DT);
        }
        assert_eq!(b.live_count(), 0);
    }

    #[test]
    fn no_muere_justo_al_cruzar_el_borde() {
        let mut b = Bullets::with_capacity(4);
        b.spawn(Spawn {
            pos: Vec2::new(ARENA_W * 0.5, 2.0),
            vel: Vec2::new(0.0, -60.0),
            ..Default::default()
        });
        // Un segundo: se aleja 60 unidades, aun dentro del margen de 64.
        for _ in 0..60 {
            b.update(DT);
        }
        assert_eq!(
            b.live_count(),
            1,
            "el margen de culling deberia mantenerla viva"
        );
    }

    #[test]
    fn la_colision_detecta_el_solape() {
        let mut b = Bullets::with_capacity(4);
        let p = centro();
        b.spawn(Spawn {
            pos: p,
            kind: KIND_MEDIUM,
            ..Default::default()
        });

        assert!(b.hit_circle(p, 2.5).is_some(), "encima deberia colisionar");

        let radio = BULLET_KINDS[KIND_MEDIUM as usize].radius;
        let justo_fuera = p + Vec2::new(radio + 2.5 + 0.5, 0.0);
        assert!(b.hit_circle(justo_fuera, 2.5).is_none(), "por fuera no");

        let justo_dentro = p + Vec2::new(radio + 2.5 - 0.5, 0.0);
        assert!(b.hit_circle(justo_dentro, 2.5).is_some(), "por dentro si");
    }

    #[test]
    fn la_colision_ignora_las_balas_muertas() {
        let mut b = Bullets::with_capacity(4);
        let p = centro();
        let i = b.spawn(en(p)).unwrap();
        assert!(b.hit_circle(p, 2.5).is_some());
        b.kill(i);
        assert!(b.hit_circle(p, 2.5).is_none());
    }

    #[test]
    fn damage_circle_mata_todas_las_que_tocan() {
        let mut b = Bullets::with_capacity(32);
        let p = centro();
        // Tres encima y dos lejos.
        for _ in 0..3 {
            b.spawn(en(p));
        }
        b.spawn(en(p + Vec2::new(200.0, 0.0)));
        b.spawn(en(p + Vec2::new(0.0, 200.0)));

        assert_eq!(b.damage_circle(p, 5.0), 3);
        assert_eq!(b.live_count(), 2, "las lejanas siguen vivas");
        assert_eq!(b.damage_circle(p, 5.0), 0, "no se cobran dos veces");
    }

    #[test]
    fn el_parry_solo_toca_las_parryables() {
        let mut b = Bullets::with_capacity(32);
        let p = centro();
        b.spawn(Spawn {
            pos: p,
            flags: FLAG_PARRYABLE,
            ..Default::default()
        });
        b.spawn(en(p)); // normal, no parryable
        assert_eq!(b.parry_circle(p, 30.0), 1);
        assert_eq!(b.live_count(), 1, "la normal sigue ahi");
    }

    #[test]
    fn el_parry_no_alcanza_lo_que_esta_lejos() {
        let mut b = Bullets::with_capacity(16);
        let p = centro();
        b.spawn(Spawn {
            pos: p + Vec2::new(200.0, 0.0),
            flags: FLAG_PARRYABLE,
            ..Default::default()
        });
        assert_eq!(b.parry_circle(p, 30.0), 0);
        assert_eq!(b.live_count(), 1);
    }

    #[test]
    fn el_graze_solo_se_cobra_una_vez_por_bala() {
        let mut b = Bullets::with_capacity(16);
        let p = centro();
        b.spawn(en(p));
        assert_eq!(b.graze_circle(p, 30.0), 1);
        assert_eq!(
            b.graze_circle(p, 30.0),
            0,
            "la misma bala no paga dos veces"
        );
        assert_eq!(b.live_count(), 1, "rozar no mata la bala");
    }

    #[test]
    fn el_graze_cuenta_cada_bala_nueva() {
        let mut b = Bullets::with_capacity(32);
        let p = centro();
        for i in 0..5 {
            b.spawn(en(p + Vec2::new(i as f32 * 2.0, 0.0)));
        }
        assert_eq!(b.graze_circle(p, 30.0), 5);
    }

    #[test]
    fn clear_deja_el_pool_como_nuevo() {
        let mut b = Bullets::with_capacity(64);
        for i in 0..40 {
            b.spawn(en(centro() + Vec2::new(i as f32, 0.0)));
        }
        b.clear();
        assert_eq!(b.live_count(), 0);
        assert_eq!(b.scanned_slots(), 0);
        assert!(b.hit_circle(centro(), 50.0).is_none());
        // Y sigue sirviendo.
        assert!(b.spawn(en(centro())).is_some());
    }

    #[test]
    fn el_pool_no_reserva_memoria_durante_el_juego() {
        let mut b = Bullets::with_capacity(1024);
        let cap_inicial = b.capacity();
        for ciclo in 0..50 {
            for i in 0..500 {
                b.spawn(Spawn {
                    pos: centro(),
                    vel: Vec2::new(i as f32, ciclo as f32),
                    ttl: 0.05,
                    ..Default::default()
                });
            }
            for _ in 0..10 {
                b.update(DT);
            }
        }
        assert_eq!(b.capacity(), cap_inicial, "la capacidad no debe moverse");
    }
}
