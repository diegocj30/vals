//! El paseo: el nivel de correr y disparar que se anda antes de cada jefe.
//!
//! La idea: niveles de correr y disparar, como en Cuphead, **ademas** de los
//! jefes: un paseo por la calle de cada baile, con enemigos de su mundo, que
//! acaba en la puerta del salon donde espera el jefe. Esto es su simulacion.
//!
//! Vive aparte del `World` a proposito. El mundo del jefe es una arena fija de
//! 640 x 800 con un pool de 32k balas que se matan al salirse de ella, y un
//! paseo es una calle de varios miles de unidades que se recorre con la camara.
//! Meter las dos cosas en el mismo sitio obligaba a tocar la simulacion del
//! jefe, y esa tiene un replay dorado que no se toca. Aqui hay
//! pocas cosas vivas a la vez —una docena de enemigos, unos cuantos platos—,
//! asi que bastan unos `Vec` con la capacidad reservada al empezar.
//!
//! Las reglas del core valen igual: ticks fijos, sin reloj ni ficheros, el
//! azar sale de una `Pcg32` sembrada y la trigonometria de `math`. Mismo estado
//! mas mismo input da el mismo paseo, y hay test que lo vigila.
//!
//! El nivel es **datos**: un RON en `assets/paseos/`, igual que los jefes. Los
//! otros tres paseos seran tres ficheros mas.

use glam::Vec2;
use serde::Deserialize;

use crate::equipo::{ABANICO, Tiro, guiar};
use crate::events::Events;
use crate::hash::Fnv1a;
use crate::math::{TAU, sin, sin_cos};
use crate::player::{
    AIR_CONTROL, COYOTE_TICKS, DASH_COOLDOWN_TICKS, DASH_IFRAME_TICKS, DASH_SPEED, DASH_TICKS,
    Dash, FOCUS_RAMP_TICKS, GRAVITY, INPUT_BUFFER_TICKS, JUMP_BUFFER_TICKS, JUMP_CUT, JUMP_SPEED,
    PARRY_COOLDOWN_TICKS, PARRY_RADIUS, PLAYER_ACCEL_TICKS, PLAYER_DECEL_TICKS, PLAYER_SPEED,
    SHOT_SPEED, mover_hacia,
};
use crate::{DT, Equipo, InputFrame, Pcg32, Player};

/// Los paseos que trae el juego, embebidos como los jefes.
///
/// Cada uno dice delante de que jefe va (`jefe`), asi que el orden de esta
/// lista no significa nada: un baile sin paseo se entra directo, como antes.
pub const PASEO_RONS: [&str; 4] = [
    include_str!("../../../assets/paseos/viena.ron"),
    include_str!("../../../assets/paseos/arrabal.ron"),
    include_str!("../../../assets/paseos/chicago.ron"),
    include_str!("../../../assets/paseos/montmartre.ron"),
];

/// Lo que se ve de la calle, en unidades logicas.
///
/// **Apaisado**, al reves que la arena de los jefes. En un danmaku las balas
/// bajan y el alto es tiempo de reaccion; aqui se corre de lado, y lo que hace
/// falta es ver lo que viene por delante.
pub const VISTA_W: f32 = 960.0;
/// Donde lleva la camara a la jugadora, como fraccion de la vista. Esta aqui y
/// no en el dibujo porque la arena del portero se cierra justo cuando la
/// camara llega a ella: cerrarla antes o despues seria un salto de camara.
pub const ENCUADRE: f32 = 0.38;
pub const VISTA_H: f32 = 540.0;

/// Donde pisan los pies en la acera.
pub const SUELO_Y: f32 = 470.0;

/// Del centro del cuerpo a las suelas. `Player::pos` es el centro, como en los
/// combates, asi que el dibujo no tiene que saber que aqui se anda.
pub const PIES: f32 = 30.0;

/// Golpes que aguanta la jugadora: tres, como las vidas de un combate.
pub const VIDAS: u32 = 3;

/// Invulnerabilidad tras un golpe. Menos que los 120 del respawn del jefe: aqui
/// no se limpia la pantalla, pero tampoco hay una pared de balas encima, y un
/// segundo y medio basta para salir de donde te han dado.
pub const INVULNERABLE_TRAS_GOLPE: u32 = 90;

/// El cuerpo con el que se recibe: una capsula vertical, no el punto de 2,5 del
/// danmaku. Aqui no se esquiva al pixel entre balas, se salta por encima de
/// una pareja, y con un punto se atravesarian enemigos enteros.
const CUERPO_RADIO: f32 = 10.0;
/// Media altura de la capsula, sin contar el radio.
const CUERPO_MEDIO: f32 = 16.0;

/// A que distancia por delante de la jugadora despierta un enemigo. Con la
/// camara llevandola al 38 % de la vista, el borde derecho queda a unas 595:
/// despiertan justo fuera de cuadro y entran andando.
const ACTIVAR: f32 = 640.0;
/// A que distancia por detras se olvida. Lo que se queda atras en un
/// run-and-gun no vuelve.
const OLVIDAR: f32 = 900.0;
/// Cuanto por debajo de la acera se da por caida en un foso.
const CAIDA: f32 = 180.0;

/// El alcance de un disparo es lo que vive, a `SHOT_SPEED`: con la aguja,
/// unas 630 unidades, dos tercios de la vista. Dispara a lo que ves, no a lo
/// que aun no ha salido. Cada tiro lo dice en `Tiro::ttl_calle`.
const DISPARO_RADIO: f32 = 6.0;
/// Tope de disparos vivos. La aguja, a 15 por segundo con 0,7 de vida, nunca
/// pasa de 11; el abanico, de tres en tres cada 6 ticks, llega a 21.
const MAX_DISPAROS: usize = 32;

/// El radio de una ficha. Algo mas gorda que un plato: se coge al rozarla, y
/// el sitio dificil tiene que ser llegar, no acertarle al pixel.
pub const RADIO_FICHA: f32 = 12.0;

/// Los platos caen mas despacio que la jugadora: un arco que se lee con
/// tiempo, no una pedrada.
const GRAVEDAD_PLATO: f32 = 900.0;
/// Lo que tarda un plato en llegar a donde estabas al tirarlo.
const VUELO_PLATO: f32 = 1.05;
const RADIO_PLATO: f32 = 9.0;
/// Cada cuantos ticks tira un camarero, y hasta donde ve.
pub const CADENCIA_CAMARERO: u32 = 96;
const ALCANCE_CAMARERO: f32 = 620.0;

// Viena. El verbo del vals es **girar**, a tres tiempos: las hojas bajan
// dando vueltas, el husar llega valsando y el director gira la batuta.

/// La hoja del Prater da una vuelta por compas alrededor de un centro que baja
/// y se va hacia la izquierda. La vuelta se ve de lado: en pantalla es una
/// elipse, ancha y aplastada, como el giro de una pareja.
const GIRO_HOJA_X: f32 = 46.0;
const GIRO_HOJA_Y: f32 = 16.0;
const DERIVA_HOJA: f32 = 70.0;
const CAIDA_HOJA: f32 = 42.0;
/// El husar sale por detras, a esto de la jugadora: fuera de cuadro, que la
/// camara la lleva al 38 % y el borde izquierdo queda a unas 365.
pub const DETRAS_HUSAR: f32 = 420.0;
/// Su vals: el paso largo del uno, mas rapido que ella corriendo, y los
/// pasitos del dos y el tres. De media corre algo mas que ella: si no se le
/// tumba, alcanza.
const PASO_LARGO_HUSAR: f32 = 560.0;
const PASO_CORTO_HUSAR: f32 = 230.0;
/// El director, portero de Viena: con la aguja (15 por segundo si entran
/// todos) son unos 11 segundos de fuego, y con lo que se esquiva, unos quince.
/// Con 330 eran 22 y se hacia largo: un portero es un jefe pequeno.
const VIDA_DIRECTOR: i32 = 160;
/// La espiral de la batuta: cuantas notas suelta en el uno y el dos de cada
/// compas, y a que velocidad, por fase.
pub const BRAZOS_ESPIRAL: [usize; 3] = [3, 4, 5];
const VEL_ESPIRAL: [f32; 3] = [150.0, 165.0, 180.0];
/// El tres: la nota rosa que tira a donde estas en el tercer tiempo.
const VEL_TRES: f32 = 210.0;
/// El remolino, desde la segunda fase: cada cuantos compases (el anterior lo
/// pasa preparandolo, quieto y con los brazos arriba) y cuantas notas.
pub const REMOLINO_CADA: [u64; 3] = [0, 4, 3];
pub const NOTAS_REMOLINO: usize = 12;
/// Las notas del remolino salen despacio y girando, y aceleran hasta el tope:
/// una espiral que se abre y luego se va recta.
const VEL_REMOLINO: f32 = 110.0;
const TOPE_REMOLINO: f32 = 300.0;
const ACELERA_REMOLINO: f32 = 1.015;
const GIRO_REMOLINO: f32 = 2.4;
/// De la mano a la punta de la batuta.
pub const BATUTA: f32 = 24.0;

/// Por donde va la batuta en el tick `t`: una vuelta cada cuatro compases de
/// tres, un doceavo por tiempo. El dibujo usa esta cuenta, asi que las notas
/// salen de donde se ve la punta.
pub fn giro_batuta(t: f32, tiempo: u32) -> f32 {
    TAU * t / (12 * tiempo.max(1)) as f32
}

/// Que hace el director en el tick `t`: que tiempo del compas suena (0, 1 o
/// 2), cuantos ticks lleva, y si ese compas lo pasa preparando el remolino.
pub fn batuta(tick: u64, tiempo: u32, fase: usize) -> (u64, u64, bool) {
    let tiempo = u64::from(tiempo.max(1));
    let cada = REMOLINO_CADA[fase.min(2)];
    let prepara = cada > 0 && (tick / (3 * tiempo)) % cada == cada - 1;
    ((tick / tiempo) % 3, tick % tiempo, prepara)
}

/// Las perlas de la flapper: cuantas suelta a la vez, repartidas por el
/// collar, y a que velocidad salen. Rectas y sin gravedad: son un collar que
/// gira, no una piedra, y lo que se lee es el giro.
pub const PERLAS_POR_GOLPE: usize = 3;
const VEL_PERLA: f32 = 200.0;
/// Hasta donde ven la flapper y el saxofonista. Menos que el camarero: los
/// dos estan en pantalla antes de empezar, y lo que se esquiva tiene que salir
/// de algo que ya se ha visto.
const ALCANCE_CHICAGO: f32 = 560.0;
/// Las notas del saxo: una cada `SEPARACION_SAXO` ticks dentro de la rafaga.
const SEPARACION_SAXO: u64 = 6;
const VEL_SAXO: f32 = 250.0;
/// Los brincos del saltarin: alto en los dos golpes de tres, bajo en el de
/// dos, y lo que avanza en cada uno. Los ultimos ticks de cada golpe los pasa
/// en el suelo, que es el unico momento en que se le puede saltar por encima.
const BRINCO_LARGO: f32 = 120.0;
const BRINCO_CORTO: f32 = 45.0;
const AVANCE_LARGO: f32 = 150.0;
const AVANCE_CORTO: f32 = 50.0;
pub const EN_EL_SUELO: u64 = 6;
/// El canotier: lo que avanza hacia ti por segundo, y lo ancho y lo alto de
/// la vuelta que da alrededor de su centro.
const VEL_CANOTIER: f32 = 110.0;
pub const VUELTA_CANOTIER: (f32, f32) = (46.0, 64.0);
/// Los ticks del tiron de cada golpe: un tercio de vuelta, de golpe y
/// frenando. El resto del golpe planea.
pub const TIRON_CANOTIER: u64 = 14;
/// La fila: cuantas son, de centro a centro con el hueco cerrado, y cuanto
/// se abre el hueco del medio. Abierto, entre las dos del medio quedan unas
/// 110 unidades libres: cabe ella y sobra.
pub const FILA: u32 = 4;
pub const ESPACIO_FILA: f32 = 46.0;
pub const HUECO_FILA: f32 = 120.0;
/// Lo que tarda el hueco en abrirse o cerrarse.
pub const TICKS_HUECO: u64 = 12;
/// El paso de la fila: avanza esto en los primeros ticks de cada golpe.
const PASO_FILA: f32 = 30.0;
const TICKS_PASO_FILA: u64 = 12;
/// Las negras del contrabajista: salen a esto y se tuercen hacia el suelo
/// tantos radianes por segundo. Es un arco de circulo, no una parabola: el
/// charleston gira.
const VEL_NEGRA: f32 = 340.0;
const GIRO_NEGRA: f32 = 1.0;
/// La onda del slap: a ras de acera, a esto por segundo.
const VEL_ONDA: f32 = 340.0;

// El arrabal. El verbo del tango en el jefe es la **aceleracion negativa**:
// balas que salen disparadas, frenan en seco y se quedan. Sus enemigos se
// mueven igual: arrancan a tope y se clavan.

/// El compadrito, en ticks: un tiempo de amago (se echa atras, se ve venir),
/// uno de estocada frenando y dos plantado. Con el tango a 30 ticks por tiempo
/// es un compas justo, y arranca siempre en un tiempo de la musica.
pub const AMAGO_COMPADRITO: u32 = 30;
pub const ESTOCADA_COMPADRITO: u32 = 30;
pub const PLANTADO_COMPADRITO: u32 = 60;
pub const CICLO_COMPADRITO: u32 = AMAGO_COMPADRITO + ESTOCADA_COMPADRITO + PLANTADO_COMPADRITO;
/// La velocidad con que sale la estocada, tres veces la de la jugadora. Frena
/// en linea recta hasta cero: unas 230 unidades, algo mas que un salto.
const ESTOCADA_V: f32 = 900.0;
/// Hasta donde ve un compadrito: no se lanza a lo que no tiene cerca.
const ALCANCE_COMPADRITO: f32 = 420.0;

/// Cada cuantos ticks tira una rosa la florista: un compas de tango.
pub const CADENCIA_FLORISTA: u32 = 120;
const ALCANCE_FLORISTA: f32 = 600.0;
/// Una rosa sale a esto y pierde un 7 % por tick: a los `VUELO_ROSA` ticks
/// casi no se mueve, y ha recorrido unas 215 unidades hacia donde estabas.
const ROSA_V: f32 = 950.0;
const FRENO_ROSA: f32 = 0.93;
pub const VUELO_ROSA: u64 = 45;
/// Lo que se queda colgada en el aire tras frenar, antes de marchitarse y
/// caer: dos tiempos. Es el corte del tango hecho objeto.
pub const COLGADA_ROSA: u64 = 60;
/// Una rosa marchita cae despacio, como cae una flor.
const GRAVEDAD_ROSA: f32 = 320.0;

/// El compadrito colgado del cable del tranvia se suelta cuando lo tiene
/// debajo y te tiene cerca, pero no encima: nunca cae sobre la cabeza.
const SALTO_CERCA: f32 = 80.0;
const SALTO_LEJOS: f32 = 320.0;

/// La gaviota, en ticks: un tiempo de remonte (sube a su altura y abre las
/// alas: el aviso), uno de picado frenando hasta cero y dos colgada. Un
/// compas, y arranca en un tiempo de la musica, como el compadrito.
pub const REMONTE_GAVIOTA: u32 = 30;
pub const PICADO_GAVIOTA: u32 = 30;
pub const COLGADA_GAVIOTA: u32 = 60;
pub const CICLO_GAVIOTA: u32 = REMONTE_GAVIOTA + PICADO_GAVIOTA + COLGADA_GAVIOTA;
const ALCANCE_GAVIOTA: f32 = 520.0;
/// Lo mas largo de un picado: llega a donde estabas si estas mas cerca.
const PICADO_MAX: f32 = 300.0;

/// Hasta donde ve el malevo. Se vuelve hacia ti al empezar cada compas, y no
/// antes: el que se le pone detras tiene un compas para tumbarlo.
const ALCANCE_MALEVO: f32 = 560.0;
/// Lo que quita una rosa devuelta de un parry: tumba a un malevo de una.
pub const DANO_DEVUELTA: i32 = 20;
const VEL_DEVUELTA: f32 = 720.0;
const ALCANCE_DEVUELTA: f32 = 560.0;

/// El guapo aguanta unos 12 segundos de aguja sin fallar una; esquivando,
/// unos quince.
pub const VIDA_GUAPO: i32 = 175;
/// La estocada del guapo: unas 380 unidades, casi media arena. La finta es la
/// misma estocada en pequeno, unas 50: sale y se clava enseguida.
const ESTOCADA_GUAPO: f32 = 1500.0;
const FINTA_GUAPO: f32 = 200.0;
/// Lo que se separa de las paredes de la arena.
const MARGEN_ARENA: f32 = 50.0;
/// El abanico de rosas: de tres, y de cinco desde la segunda fase. Lo que se
/// abre de una a la siguiente.
const ABRE_ROSAS: f32 = 0.3;
/// Lo que baja la primera rosa del abanico, en radianes: llega a la altura del
/// pecho de la jugadora.
const BAJA_ROSA: f32 = 0.25;

/// Lo que hace el guapo en cada tiempo de la musica.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paso {
    /// Se echa atras: viene la estocada.
    Amago,
    /// Una estocada corta que se clava enseguida. Avisa de la de verdad.
    Finta,
    /// Se lanza a tope y frena hasta cero.
    Estocada,
    Plantado,
    /// Echa el brazo atras con el ramo.
    Carga,
    /// Tira el abanico de rosas.
    Tira,
}

/// Las jugadas del guapo, de un compas cada una, por fase: la de estocada y
/// la de rosas, una y otra. Cada fase anade un engano a la anterior.
const JUGADAS: [[[Paso; 4]; 2]; 3] = {
    use Paso::*;
    [
        [
            [Amago, Estocada, Plantado, Plantado],
            [Carga, Tira, Plantado, Plantado],
        ],
        [
            [Finta, Amago, Estocada, Plantado],
            [Carga, Tira, Carga, Tira],
        ],
        [
            [Amago, Estocada, Estocada, Plantado],
            [Carga, Tira, Estocada, Plantado],
        ],
    ]
};

/// Lo que llena el medidor cada disparo que entra. El parry sigue siendo el
/// chorro; esto es el goteo, igual que el graze en los combates.
const METER_POR_IMPACTO: f32 = 0.8;
/// Lo que quita el super a cada enemigo en pantalla.
const DANO_SUPER: i32 = 12;

/// Lo mas empinada que puede ser una escalera: 0,7 de subida por unidad de
/// ancho. La fisica se fia de ello (ver `mover`): con un dash a 900 la acera
/// sube como mucho 10,5 en un tick, y eso es lo que se perdona al pisar.
pub const PENDIENTE_MAX: f32 = 0.7;

/// La corista patea cuando te tiene a esta distancia, y hasta aqui baja a por
/// ti con su chasse.
const ALCANCE_CORISTA: f32 = 560.0;
/// Lo largo de su pierna, de la cadera a la punta del botin.
pub const PIERNA: f32 = 40.0;
/// Radio del botin, para el golpe.
const RADIO_BOTIN: f32 = 11.0;
/// Por encima de esto de `gesto` la pierna esta arriba y hace dano. Por debajo
/// esta recogiendola, y pasar pegada a ella no se castiga.
const PIERNA_PELIGRO: f32 = 0.3;
/// El volante que levanta el taconazo: corre a ras de acera y se apaga en poco
/// mas de un segundo. Es la patada a ras de suelo del jefe, en pequeno.
const VELOCIDAD_VOLANTE: f32 = 330.0;
const VIDA_VOLANTE: f32 = 1.1;

/// La botella de champan: descorcha cada cuatro tiempos si te ve.
const ALCANCE_BOTELLA: f32 = 600.0;
/// Lo que tarda un corcho en caer donde estabas.
const VUELO_CORCHO: f32 = 0.8;
/// Las burbujas del estallido: pocas, rapidas y de medio segundo. Llegan a
/// unas 95 unidades, asi que se esquivan alejandose, no saltando.
const BURBUJAS: usize = 8;
const VELOCIDAD_BURBUJA: f32 = 210.0;
const VIDA_BURBUJA: f32 = 0.45;

/// El pintor sacude el pincel cada cuatro tiempos, en abanico de tres gotas.
const ALCANCE_PINTOR: f32 = 580.0;
const VUELO_GOTA: f32 = 0.7;
/// Lo que dura el charco de pintura en la acera antes de secarse.
const VIDA_MANCHA: f32 = 1.4;

/// El paquete de confeti vuela hacia la izquierda a brincos de galop, uno por
/// tiempo. Al tenerte a `MECHA_CONFETI_X` prende la mecha y en un tiempo
/// revienta: papelillos que salen a tope, frenan, caen revoloteando y se
/// apagan. Llegan a unas 120 unidades.
const VEL_CONFETI: f32 = 150.0;
const BRINCO_CONFETI: f32 = 24.0;
const MECHA_CONFETI_X: f32 = 190.0;
pub const MECHA_CONFETI: u32 = 24;
pub const PAPELILLOS: usize = 10;
const VEL_PAPELILLO: f32 = 300.0;
const FRENO_PAPELILLO: f32 = 0.96;
const GRAVEDAD_PAPELILLO: f32 = 260.0;
pub const VIDA_PAPELILLO: f32 = 0.9;

/// El apache, en ticks: un compas de dos tiempos esfumado (se ve su sombra
/// donde va a salir), un tiempo de navajazo y dos plantado. Se esfuma en un
/// tiempo de la musica, asi que sale en el uno del compas siguiente.
pub const OCULTO_APACHE: u32 = 48;
pub const GOLPE_APACHE: u32 = 24;
pub const PLANTADO_APACHE: u32 = 48;
pub const CICLO_APACHE: u32 = OCULTO_APACHE + GOLPE_APACHE + PLANTADO_APACHE;
/// Lo que tarda en sacar la navaja tras salir: lo justo para verle.
const AMAGO_APACHE: u32 = 8;
const ALCANCE_APACHE: f32 = 420.0;
/// A que distancia de ti sale, por el otro lado del que estaba.
pub const LADO_APACHE: f32 = 100.0;
/// El paso que da con el navajazo, y su alcance.
const ESTOCADA_APACHE: f32 = 260.0;
const RADIO_NAVAJA: f32 = 12.0;
const NAVAJA_PELIGRO: f32 = 0.5;
/// La humareda de cuando se esfuma: no hace dano, es el aviso.
const VIDA_HUMO: f32 = 0.5;

/// Monsieur Loyal: lo que dura cada numero segun la fase, y lo que vive lo
/// que suelta. Las palomas se apagan antes de cruzar la pista: de lejos solo
/// llegan los aros, y de cerca, el baston.
const NUMERO_LOYAL: [u32; 3] = [120, 96, 72];
const VEL_PALOMA: f32 = 260.0;
pub const VIDA_PALOMA: f32 = 1.3;
const VEL_ARO: f32 = 280.0;
pub const VIDA_ARO: f32 = 1.8;
const VEL_BARRIDO: f32 = 900.0;
const VIDA_BARRIDO: f32 = 0.3;
/// Por donde anda en la pista: de cerca se aparta y de lejos se acerca.
const CERCA_LOYAL: f32 = 220.0;
const LEJOS_LOYAL: f32 = 480.0;

/// Los numeros de Monsieur Loyal, en el orden en que los presenta. En la
/// primera fase se salta el baston.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Numero {
    /// Levanta la chistera y salen palomas en abanico hacia ti.
    Palomas,
    /// Tira aros que ruedan botando por la pista.
    Aros,
    /// Se acerca de un brinco y barre el suelo con el baston: se salta.
    Baston,
}

const NUMEROS: [Numero; 6] = [
    Numero::Palomas,
    Numero::Aros,
    Numero::Baston,
    Numero::Palomas,
    Numero::Baston,
    Numero::Aros,
];

/// Cuanto se ha movido en este tick la plataforma que pisa quien tiene los
/// pies en `(x, pies)`, si se mueve. Va despues de recolocarlas, asi que se
/// mira donde estaba antes: lo que el tranvia lleva encima no se le escurre.
fn arrastre(def: &PaseoDef, bases: &[Vec2], t: u64, x: f32, pies: f32) -> Option<Vec2> {
    def.plataformas.iter().zip(bases).find_map(|(pl, base)| {
        let m = pl.mueve?;
        let antes = *base + m.desvio(t);
        let encima = (pies - antes.y).abs() < 1.0 && x >= antes.x && x <= antes.x + pl.ancho;
        encima.then(|| Vec2::new(pl.x, pl.y) - antes)
    })
}

/// Una rosa recien tirada, la tire quien la tire: la florista, el malevo o el
/// guapo.
fn rosa_lanzada(pos: Vec2, vel: Vec2, rosa: bool, tick: u64) -> Enemigo {
    Enemigo {
        tipo: Tipo::Rosa,
        pos,
        vida: Tipo::Rosa.vida(),
        dir: vel.x.signum(),
        nacio: tick,
        rosa,
        golpe: 0,
        reloj: 0,
        lanzados: 0,
        base_y: pos.y,
        avance: 0.0,
        gesto: 0.0,
        vel,
        portero: false,
    }
}

/// Un tramo de acera, de `.0` a `.1`. Entre dos tramos hay un foso.
pub type Tramo = (f32, f32);

/// Una mesa de cafe o un balcon. Se atraviesa desde abajo y se pisa desde
/// arriba, que es lo que se espera de una plataforma en un run-and-gun: saltar
/// y no darse con la cabeza.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Plataforma {
    /// Borde izquierdo.
    pub x: f32,
    /// La superficie, donde se pisa.
    pub y: f32,
    pub ancho: f32,
    /// Si se mueve. Entonces `x` e `y` son donde esta **ahora**: el paseo las
    /// reescribe cada tick a partir de donde dice el RON (ver
    /// `Paseo::mover_plataformas`), asi que todo lo que pisa o dibuja
    /// plataformas vale sin enterarse.
    #[serde(default)]
    pub mueve: Option<Mueve>,
}

/// Como se mueve una plataforma: el fiacre, el tranvia, el tren, las aspas.
///
/// Sale solo del tick, sin estado propio: dos paseos con el mismo tick tienen
/// las plataformas en el mismo sitio, y el hot-reload no las descoloca.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Mueve {
    /// Va y vuelve en linea recta hasta `(dx, dy)` de donde dice el RON, en
    /// `periodo` ticks la ida y la vuelta. Frena en los extremos, que es lo
    /// que deja subirse y bajarse sin que tire. `fase`, de 0 a 1, es por
    /// donde va al empezar.
    Vaiven {
        dx: f32,
        dy: f32,
        periodo: u32,
        #[serde(default)]
        fase: f32,
    },
    /// Da vueltas alrededor de donde dice el RON sin volcar, como la cabina de
    /// una noria: las aspas del molino. `fase` reparte varias por el mismo
    /// circulo. Gira siempre en el sentido de las agujas del reloj (y hacia
    /// abajo): un `radio` negativo solo la adelanta media vuelta.
    Gira {
        radio: f32,
        periodo: u32,
        #[serde(default)]
        fase: f32,
    },
}

impl Mueve {
    /// Cuanto se ha apartado de su sitio en el tick `t`.
    pub fn desvio(self, t: u64) -> Vec2 {
        let vuelta = |periodo: u32, fase: f32| {
            let periodo = periodo.max(1);
            TAU * ((t % u64::from(periodo)) as f32 / periodo as f32 + fase)
        };
        match self {
            Mueve::Vaiven {
                dx,
                dy,
                periodo,
                fase,
            } => {
                let (_, c) = sin_cos(vuelta(periodo, fase));
                Vec2::new(dx, dy) * (1.0 - c) * 0.5
            }
            Mueve::Gira {
                radio,
                periodo,
                fase,
            } => {
                let (s, c) = sin_cos(vuelta(periodo, fase));
                Vec2::new(c, s) * radio
            }
        }
    }
}

/// La arena del portero: una vista entera, de `desde` a `desde + VISTA_W`.
///
/// Al llegar, la camara se para y la calle se cierra por los dos lados hasta
/// tumbar al portero (los enemigos con `portero: true`). Lo de detras ya no
/// vuelve: tras cerrarse, la calle no deja volver a antes de `desde`.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Porteria {
    pub desde: f32,
    /// La ficha que suelta el portero: su indice en `fichas`. No se ve ni se
    /// coge mientras siga en pie, y sale donde diga el RON.
    #[serde(default)]
    pub ficha: Option<usize>,
}

/// Una escalera: de `x` a `x + ancho` la acera sube `sube` unidades, y se
/// queda arriba. Es lo que hace que un paseo **suba** en vez de ir llano.
///
/// Para la fisica es una rampa y no peldanos: andar sobre una pendiente es
/// pegarse a una recta, y subir peldanos pedia saltar cada uno o una regla de
/// "escalon que se sube solo" que acababa siendo la misma rampa con saltitos.
/// El dibujo si pinta los peldanos, con la rampa pasando por sus bordes.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Escalera {
    pub x: f32,
    pub ancho: f32,
    pub sube: f32,
}

/// Los enemigos de los paseos. Los tres primeros son de Viena, y cada uno pide
/// una cosa distinta: saltar, ponerse a tiro y apuntar arriba. Cada ciudad
/// trae los suyos con la misma regla.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum Tipo {
    /// Una pareja que baila por la acera hacia ti, girando: paso largo en el
    /// uno del compas y dos cortos. Se salta o se tumba.
    Pareja,
    /// Un camarero plantado que tira platos en arco a donde estas. Uno de cada
    /// tres es rosa y se puede parriar.
    Camarero,
    /// Una hoja de castano del Prater que baja girando, una vuelta por compas.
    /// Las rosas se parrian.
    Hoja,
    /// Un husar que entra por detras, fuera de cuadro, y viene valsando: hay
    /// que darse la vuelta y tumbarlo, o saltarlo.
    Husar,
    /// El director de orquesta, portero de Viena: en su tarima, gira la
    /// batuta y suelta espirales de notas a tres tiempos. En el tres, una
    /// rosa. Desde la segunda fase, el remolino.
    Director,
    // --- El arrabal. El tango arranca a tope y se clava,
    // y sus enemigos tambien.
    /// Un compadrito de chambergo y panuelo: amaga un tiempo, se
    /// lanza en una estocada que frena en seco y se queda plantado. Se salta.
    Compadrito,
    /// Una florista en un balcon del conventillo que tira rosas. Esta arriba,
    /// asi que se la tumba plantada y apuntando arriba.
    Florista,
    /// La rosa que tira la florista: sale disparada, frena, se queda colgada
    /// en el aire y se marchita. No se pone en el RON; nace de la florista.
    /// Una de cada tres es rosa de verdad y se parria.
    Rosa,
    /// Una gaviota del puerto. Se cierne quieta hasta tenerte a tiro; en un
    /// tiempo de la musica remonta (el aviso), pica a donde estabas frenando
    /// hasta cero y se queda colgada en el aire dos tiempos. Las rosas se
    /// parrian.
    Gaviota,
    /// Un malevo plantado con el poncho enrollado en el brazo: los disparos de
    /// frente se los come el poncho. Cae por la espalda, o con una rosa
    /// devuelta de un parry. Tira la rosa que lleva en la boca, la mitad de
    /// las veces de las de parry.
    Malevo,
    /// El guapo del barrio, el portero del arrabal: estocadas que frenan en
    /// seco, fintas que las avisan y rosas que se quedan colgadas, a compas.
    Guapo,
    // --- Chicago. El charleston gira y va en clave 3-3-2, y
    // sus tres enemigos tambien.
    /// Una flapper plantada, bailando el charleston en el sitio, con el collar
    /// de perlas dando vueltas como un lazo. En cada golpe de la clave suelta
    /// tres perlas por donde va el collar: una espiral que se lee mirandola.
    /// En el golpe de dos, una es rosa. Pide el parry.
    Flapper,
    /// Un saxofonista en una escalera de incendios, por encima de la calle.
    /// Sopla rafagas de tres, tres y dos notas apuntadas a ti: se esquiva
    /// andando y se le tumba plantada, apuntando arriba.
    Saxo,
    /// Un dandi de canotier que viene brincando el charleston: dos brincos
    /// altos y uno bajo por compas. Por debajo de los altos se pasa corriendo;
    /// el bajo se salta o se tumba.
    Saltarin,
    /// Un canotier que vuela solo, como lanzado al aire en el charleston. Da
    /// una vuelta por compas alrededor de su centro, a tirones: casi toda en
    /// los primeros ticks de cada golpe de clave, que es cuando baja en picado.
    /// Viene hacia ti. Los rosas se parrian.
    Canotier,
    /// Una chica de la fila del charleston. La fila sale entera de una sola
    /// aparicion (`FILA` chicas) y avanza a pasos en la clave; el hueco del
    /// medio se abre en el segundo golpe y se cierra en el uno siguiente. Se
    /// pasa saltando a la pareja de delante, cayendo en el hueco y saltando a
    /// la de detras, o se tumba a tiros una a una.
    Fila,
    /// El contrabajista, el portero de Chicago. Plantado en su arena, tira
    /// negras que se tuercen en arco hacia el suelo en la clave 3-3-2, la
    /// ultima del dos rosa. Con menos vida da el slap: una onda que corre por
    /// la acera y se salta.
    Contrabajista,
    // --- Montmartre. El cancan patea y lo que suelta se
    // apaga enseguida, y sus tres enemigos tambien.
    /// Una corista del cancan. Baja a por ti con su chasse, se planta y da la
    /// patada alta; al bajar el talon levanta un volante que corre a ras de
    /// acera. De cerca, la pierna; de lejos, el volante: se salta.
    Corista,
    /// Una botella de champan en una mesa. Tiembla y descorcha: el corcho cae
    /// donde estabas y revienta en burbujas que duran medio segundo. Uno de
    /// cada tres corchos es rosa y se parria.
    Botella,
    /// Un pintor de la place du Tertre, subido en lo alto. Sacude el pincel
    /// en abanico y la pintura deja charcos que se secan enseguida. Esta
    /// arriba: se le dispara plantada y apuntando.
    Pintor,
    /// Un paquete de confeti que cruza volando a brincos de galop. Al tenerte
    /// cerca prende la mecha y revienta en papelillos que se apagan en menos
    /// de un segundo: se tumba antes, o se espera. Los rosas se parrian.
    Confeti,
    /// Un apache de la danse apache, gorra y panuelo rojo. Se esfuma en una
    /// humareda, sale a tu lado (antes se ve su sombra) y tira un navajazo.
    /// Luego se queda plantado, que es cuando se le tumba.
    Apache,
    /// Monsieur Loyal, el maestro de ceremonias: el portero de Montmartre, a
    /// la puerta del Moulin Rouge. Ver `Numero`.
    Loyal,
}

impl Tipo {
    /// Vida. Una nota cae de un par de disparos; un camarero aguanta medio
    /// segundo de fuego, que es lo que hace que merezca la pena esquivarle.
    fn vida(self) -> i32 {
        match self {
            Tipo::Pareja => 6,
            Tipo::Camarero => 8,
            Tipo::Hoja => 2,
            Tipo::Husar => 6,
            Tipo::Director => VIDA_DIRECTOR,
            Tipo::Compadrito => 8,
            Tipo::Florista => 6,
            Tipo::Rosa => 1,
            Tipo::Gaviota => 3,
            Tipo::Malevo => 12,
            Tipo::Guapo => VIDA_GUAPO,
            Tipo::Flapper => 8,
            Tipo::Saxo => 10,
            Tipo::Saltarin => 7,
            Tipo::Canotier => 3,
            Tipo::Fila => 6,
            // Unos 11 segundos de aguja sin fallar uno: con lo que se esquiva,
            // unos quince.
            Tipo::Contrabajista => 160,
            Tipo::Corista => 7,
            Tipo::Botella => 3,
            Tipo::Pintor => 6,
            Tipo::Confeti => 2,
            Tipo::Apache => 6,
            // Unos 13 segundos de aguja sin fallar uno: con lo que se esquiva,
            // unos quince.
            Tipo::Loyal => 192,
        }
    }

    /// Radio de su cuerpo, para los disparos y para el contacto.
    pub fn radio(self) -> f32 {
        match self {
            Tipo::Pareja => 24.0,
            Tipo::Camarero => 22.0,
            Tipo::Hoja => 14.0,
            Tipo::Husar => 24.0,
            Tipo::Director => 30.0,
            Tipo::Compadrito => 22.0,
            Tipo::Florista => 20.0,
            Tipo::Rosa => 14.0,
            Tipo::Gaviota => 16.0,
            Tipo::Malevo => 22.0,
            Tipo::Guapo => 26.0,
            Tipo::Flapper | Tipo::Saxo | Tipo::Saltarin => 22.0,
            Tipo::Canotier => 15.0,
            Tipo::Fila => 20.0,
            Tipo::Contrabajista => 32.0,
            Tipo::Corista => 20.0,
            Tipo::Botella => 14.0,
            Tipo::Pintor => 20.0,
            Tipo::Confeti => 14.0,
            Tipo::Apache => 20.0,
            Tipo::Loyal => 30.0,
        }
    }

    /// Cuanto por encima de donde pisa esta su centro.
    fn alto(self) -> f32 {
        match self {
            Tipo::Pareja => 38.0,
            Tipo::Camarero => 36.0,
            Tipo::Rosa => 0.0,
            Tipo::Hoja => 0.0,
            Tipo::Husar => 38.0,
            Tipo::Director => 48.0,
            Tipo::Compadrito => 38.0,
            Tipo::Florista => 36.0,
            Tipo::Gaviota => 0.0,
            Tipo::Malevo => 38.0,
            Tipo::Guapo => 44.0,
            Tipo::Flapper => 38.0,
            Tipo::Saxo | Tipo::Saltarin => 36.0,
            Tipo::Canotier => 0.0,
            Tipo::Fila => 38.0,
            Tipo::Contrabajista => 48.0,
            Tipo::Corista => 44.0,
            Tipo::Botella => 18.0,
            Tipo::Pintor => 36.0,
            Tipo::Confeti => 0.0,
            Tipo::Apache => 38.0,
            Tipo::Loyal => 52.0,
        }
    }

    fn como_u8(self) -> u8 {
        match self {
            Tipo::Pareja => 0,
            Tipo::Camarero => 1,
            Tipo::Hoja => 20,
            Tipo::Husar => 21,
            Tipo::Director => 22,
            Tipo::Flapper => 3,
            Tipo::Saxo => 4,
            Tipo::Saltarin => 5,
            Tipo::Canotier => 18,
            Tipo::Fila => 19,
            Tipo::Contrabajista => 23,
            Tipo::Corista => 6,
            Tipo::Botella => 7,
            Tipo::Pintor => 8,
            Tipo::Compadrito => 9,
            Tipo::Florista => 10,
            Tipo::Rosa => 11,
            Tipo::Confeti => 12,
            Tipo::Apache => 13,
            Tipo::Loyal => 14,
            Tipo::Gaviota => 15,
            Tipo::Malevo => 16,
            Tipo::Guapo => 17,
        }
    }
}

/// La clave 3-3-2 del charleston, contada en corcheas: golpes en la 0, la 3 y
/// la 6 de cada compas de ocho, igual que en el jefe (`boss3.ron`). Devuelve
/// que golpe suena (0, 1 o 2), cuantos ticks lleva sonando y cuanto dura.
///
/// Va con el tick del paseo y no con la edad de cada enemigo: la musica es una
/// para toda la calle, y el dibujo late con el mismo tick.
pub fn clave(tick: u64, tiempo: u32) -> (usize, u64, u64) {
    let corchea = u64::from((tiempo / 2).max(1));
    let fase = tick % (8 * corchea);
    let (golpe, desde, largo) = if fase < 3 * corchea {
        (0, 0, 3)
    } else if fase < 6 * corchea {
        (1, 3, 3)
    } else {
        (2, 6, 2)
    };
    (golpe, fase - desde * corchea, largo * corchea)
}

/// Por donde va el collar de la flapper en el tick `t`. Siete decimos de vuelta
/// por compas: asi un golpe no cae nunca donde el del compas anterior y la
/// espiral avanza. El dibujo usa esta misma cuenta, asi que las perlas salen
/// de donde se ve el collar.
pub fn giro_collar(t: f32, tiempo: u32) -> f32 {
    TAU * 0.7 * t / (4 * tiempo.max(1)) as f32
}

/// Por donde va la vuelta del canotier en el tick `tick`: un tercio en cada
/// golpe de clave, casi todo en los `TIRON_CANOTIER` primeros ticks. Va con
/// el tick del paseo, asi que todos los canotieres vuelan a la par.
pub fn giro_canotier(tick: u64, tiempo: u32) -> f32 {
    let (golpe, dentro, _) = clave(tick, tiempo);
    let f = (dentro as f32 / TIRON_CANOTIER as f32).min(1.0);
    TAU / 3.0 * (golpe as f32 + 1.0 - (1.0 - f) * (1.0 - f))
}

/// Lo abierto que esta el hueco de la fila, de 0 a 1: se cierra en el primer
/// golpe del compas, se abre en el segundo y sigue abierto en el de dos.
pub fn hueco_fila(tick: u64, tiempo: u32) -> f32 {
    let (golpe, dentro, _) = clave(tick, tiempo);
    let f = (dentro as f32 / TICKS_HUECO as f32).min(1.0);
    let suave = f * f * (3.0 - 2.0 * f);
    match golpe {
        0 => 1.0 - suave,
        1 => suave,
        _ => 1.0,
    }
}

/// Donde va la chica `k` de la fila respecto al centro, con el hueco abierto
/// `hueco`: las de la izquierda del hueco se apartan a la izquierda y las
/// otras a la derecha.
pub fn sitio_en_la_fila(k: u32, hueco: f32) -> f32 {
    let medio = (FILA - 1) as f32 * 0.5;
    let lado = if k < FILA / 2 { -1.0 } else { 1.0 };
    (k as f32 - medio) * ESPACIO_FILA + lado * hueco * HUECO_FILA * 0.5
}

/// Que es lo que vuela. El dibujo lo necesita, y la fisica tambien: un plato
/// cae y una perla no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    Plato,
    /// Del collar de la flapper.
    Perla,
    /// Del saxo.
    Corchea,
    /// De la batuta del director, recta.
    Batuta,
    /// Del remolino del director: sale girando y se abre.
    Remolino,
    /// Del contrabajista: se tuerce hacia el suelo.
    Negra,
    /// El slap del contrabajista, a ras de acera.
    Onda,
}

impl Forma {
    fn radio(self) -> f32 {
        match self {
            Forma::Plato => RADIO_PLATO,
            Forma::Perla => 7.0,
            Forma::Corchea => 8.0,
            Forma::Batuta => 8.0,
            Forma::Remolino => 7.0,
            Forma::Negra => 9.0,
            Forma::Onda => 12.0,
        }
    }
}

/// Lo que suelta el cancan: sale, revienta y se apaga enseguida.
///
/// Es el verbo del jefe del cancan —balas de `ttl` corto, una patada dura lo
/// que dura la patada— llevado a la calle. Ninguna vive mas de dos segundos.
/// Va aparte de `Proyectil` y su `Forma` porque esos vuelan hasta romperse o
/// salirse de la vista, sin reloj; esto se apaga a su tiempo y **cambia**: un corcho que toca la acera es un punado de burbujas, y una gota
/// de pintura, un charco.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clase {
    /// La ola del taconazo de la corista, a ras de acera.
    Volante,
    /// Un corcho de champan en arco. Al caer, revienta.
    Corcho,
    /// Una burbuja del estallido, en linea recta y de medio segundo.
    Burbuja,
    /// Una gota de pintura en arco. Al caer, charco.
    Gota,
    /// El charco: quieto en la acera, hasta que se seca.
    Mancha,
    /// Un papelillo del confeti: sale a tope, frena y cae revoloteando.
    Papelillo,
    /// Una paloma de la chistera de Monsieur Loyal, en linea recta.
    Paloma,
    /// Un aro de Monsieur Loyal, rodando y botando por la acera.
    Aro,
    /// El barrido del baston, a ras de acera y de un tercio de segundo.
    Barrido,
    /// La humareda del apache al esfumarse. No hace dano: es el aviso.
    Humo,
}

impl Clase {
    pub fn radio(self) -> f32 {
        match self {
            Clase::Volante => 12.0,
            Clase::Corcho => 7.0,
            Clase::Burbuja => 7.0,
            Clase::Gota => 6.0,
            Clase::Mancha => 14.0,
            Clase::Papelillo => 5.0,
            Clase::Paloma => 10.0,
            Clase::Aro => 16.0,
            Clase::Barrido => 14.0,
            Clase::Humo => 0.0,
        }
    }
}

/// Algo fugaz del cancan. Ver `Clase`.
#[derive(Clone, Copy, Debug)]
pub struct Fugaz {
    pub clase: Clase,
    pub pos: Vec2,
    pub vel: Vec2,
    /// Lo que le queda, en segundos. Nunca empieza por encima de dos.
    pub ttl: f32,
    pub rosa: bool,
}

/// Donde sale un enemigo.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Aparicion {
    pub tipo: Tipo,
    pub x: f32,
    /// La altura de vuelo de lo que vuela, o la superficie donde esta de pie un
    /// camarero. Cero o sin poner es la acera.
    #[serde(default)]
    pub y: f32,
    /// Solo en lo que vuela (hojas, gaviotas, canotiers, confeti): si se
    /// puede parriar.
    #[serde(default)]
    pub rosa: bool,
    /// Si es el portero del paseo: el que cierra la arena del final, con su
    /// barra de vida.
    #[serde(default)]
    pub portero: bool,
}

/// Un paseo, tal como viene del RON.
#[derive(Clone, Debug, Deserialize)]
pub struct PaseoDef {
    pub nombre: String,
    /// El jefe al que lleva, por nombre, igual que el guardado.
    pub jefe: String,
    /// La linea de debajo del nombre en la cartela de entrada.
    pub subtitulo: String,
    /// Ticks por tiempo de su musica. Las parejas bailan con esto.
    pub tiempo: u32,
    pub largo: f32,
    pub salida: f32,
    /// La puerta del salon: llegar aqui acaba el paseo.
    pub puerta: f32,
    pub suelo: Vec<Tramo>,
    pub plataformas: Vec<Plataforma>,
    pub enemigos: Vec<Aparicion>,
    /// Las escaleras, si el paseo sube. Sin ellas la acera es llana, a
    /// `SUELO_Y`, que es lo que era antes de que las hubiera.
    #[serde(default)]
    pub escaleras: Vec<Escalera>,
    /// Las fichas del paseo: el centro de cada una. Se guardan
    /// por el nombre del paseo y su sitio en esta lista, asi que **se anaden
    /// al final**: meter una en medio cambiaria cual tiene cada uno.
    #[serde(default)]
    pub fichas: Vec<(f32, f32)>,
    /// La arena del portero, si el paseo acaba con uno.
    #[serde(default)]
    pub porteria: Option<Porteria>,
}

impl PaseoDef {
    pub fn from_ron(src: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(src.trim_start_matches('\u{feff}'))
    }

    /// Los paseos embebidos.
    ///
    /// # Panics
    ///
    /// Si alguno no parsea. Estan embebidos, asi que un fallo aqui es un bug
    /// del build y no algo que pueda pasar jugando; lo vigila un test.
    pub fn de_serie() -> Vec<Self> {
        PASEO_RONS
            .iter()
            .map(|r| Self::from_ron(r).expect("paseo embebido invalido"))
            .collect()
    }

    /// Si hay acera debajo de `x`.
    pub fn hay_suelo(&self, x: f32) -> bool {
        self.suelo.iter().any(|&(a, b)| x >= a && x <= b)
    }

    /// A que altura va la acera en `x`, haya foso o no: `SUELO_Y` menos lo que
    /// hayan subido las escaleras que quedan a la izquierda. Sobre un foso es
    /// la altura a la que iria la acera si lo hubiera, que es de donde se mide
    /// la caida.
    pub fn altura(&self, x: f32) -> f32 {
        self.escaleras.iter().fold(SUELO_Y, |y, e| {
            y - e.sube * ((x - e.x) / e.ancho.max(1.0)).clamp(0.0, 1.0)
        })
    }

    /// Donde se pisa en `x`, o nada si hay foso.
    pub fn suelo_en(&self, x: f32) -> Option<f32> {
        self.hay_suelo(x).then(|| self.altura(x))
    }
    /// Si en `x` hay algo que pisar a la altura `y`: la acera o una
    /// plataforma. Es lo que mira quien anda por una azotea para no tirarse.
    pub fn hay_pie(&self, x: f32, y: f32) -> bool {
        (y >= self.altura(x) - 0.5 && self.hay_suelo(x))
            || self
                .plataformas
                .iter()
                .any(|pl| (pl.y - y).abs() < 0.5 && x >= pl.x && x <= pl.x + pl.ancho)
    }
}

/// Un enemigo despierto.
#[derive(Clone, Debug)]
pub struct Enemigo {
    pub tipo: Tipo,
    pub pos: Vec2,
    pub vida: i32,
    /// Hacia donde mira: -1 o 1.
    pub dir: f32,
    /// Tick en que desperto. Su baile va contado desde aqui, asi que dos
    /// parejas iguales no se mueven en bloque.
    pub nacio: u64,
    pub rosa: bool,
    /// Ticks de parpadeo tras recibir un disparo. Solo lo lee el dibujo.
    pub golpe: u32,
    /// Ticks que le faltan para tirar (camarero). El dibujo lo usa para
    /// echar el brazo atras justo antes.
    pub reloj: u32,
    /// Platos tirados (camarero), para saber cual toca rosa.
    lanzados: u32,
    /// Altura de vuelo (nota), o donde esta su centro con los pies en el
    /// suelo (saltarin, que vuelve ahi de cada brinco).
    base_y: f32,
    /// Lo que avanza por segundo en el brinco en curso (saltarin). Se decide
    /// al despegar, que es cuando mira si al otro lado hay donde caer.
    avance: f32,
    /// Lo levantada que lleva la pierna una corista, de 0 a 1 y vuelta a 0.
    /// Esta en el estado y no se calcula al dibujar para que el golpe y el
    /// dibujo sean la misma pierna. En la botella y el pintor es el golpe de
    /// tirar, que salta a 1 y se apaga en doce ticks: eso solo lo lee el
    /// dibujo.
    pub gesto: f32,
    /// Velocidad propia (rosa), que se frena tick a tick.
    pub vel: Vec2,
    /// Si es el portero: cierra la arena y lleva barra de vida.
    pub portero: bool,
}

impl Enemigo {
    /// Si se le puede dar. Un portero no, hasta que se cierra su arena: de
    /// lejos y quieta se le tumbaba antes de que empezase a bailar.
    fn tocable(&self, arena: bool) -> bool {
        self.vida > 0 && self.presente() && (arena || !self.portero)
    }

    /// La vida con que nacio, para la barra del portero.
    pub fn vida_max(&self) -> i32 {
        self.tipo.vida()
    }

    /// En que fase va un portero, de 0 a 2: un tercio de la vida cada una.
    pub fn fase(&self) -> usize {
        let (v, max) = (self.vida.max(0) * 3, self.vida_max());
        if v > 2 * max {
            0
        } else if v > max {
            1
        } else {
            2
        }
    }

    /// La mano de la batuta del director. La punta da vueltas alrededor.
    pub fn mano(&self) -> Vec2 {
        self.pos + Vec2::new(self.dir * 22.0, -30.0)
    }

    /// En que paso va el guapo y cuanto lleva de el, de 0 a 1. Antes de que
    /// se cierre la arena, plantado.
    pub fn paso(&self, tiempo: u32) -> Option<(Paso, f32)> {
        if self.tipo != Tipo::Guapo {
            return None;
        }
        let t = tiempo.max(1);
        let jugada = JUGADAS[self.gesto as usize][self.lanzados as usize % 2];
        let k = (self.reloj / t) as usize % 4;
        Some((jugada[k], (self.reloj % t) as f32 / t as f32))
    }

    /// Si el poncho del malevo para este disparo: lo que le viene de frente.
    /// La rosa devuelta pasa.
    fn para(&self, d: &Disparo) -> bool {
        self.tipo == Tipo::Malevo && !d.rosa && d.vel.x * self.dir < 0.0
    }

    /// Cual es en la fila, de 0 (la de la izquierda) a `FILA - 1`.
    pub fn en_la_fila(&self) -> u32 {
        self.lanzados
    }

    /// La pierna de una corista: de la cadera a la punta del botin. Parada
    /// cuelga recta; con `gesto` a 1 apunta arriba y un poco hacia ti, que es
    /// la patada del cancan.
    pub fn pierna(&self) -> Option<(Vec2, Vec2)> {
        if self.tipo != Tipo::Corista {
            return None;
        }
        let cadera = self.pos + Vec2::new(self.dir * 4.0, 4.0);
        // De colgar (-90 grados) a pasada la vertical (100), por delante.
        let a = -TAU * 0.25 + 3.3 * self.gesto;
        let (s, c) = sin_cos(a);
        Some((cadera, cadera + Vec2::new(self.dir * c, -s) * PIERNA))
    }

    /// Si esta en la calle. Un apache esfumado no esta: ni se le dispara ni
    /// se choca con el, solo se ve su sombra donde va a salir.
    pub fn presente(&self) -> bool {
        self.tipo != Tipo::Apache || self.reloj <= GOLPE_APACHE + PLANTADO_APACHE
    }

    /// La punta de la navaja de un apache, mientras pega.
    pub fn navaja(&self) -> Option<Vec2> {
        (self.tipo == Tipo::Apache && self.gesto > NAVAJA_PELIGRO)
            .then(|| self.pos + Vec2::new(self.dir * (18.0 + 34.0 * self.gesto), -8.0))
    }

    /// El numero que hace o va a hacer Monsieur Loyal.
    pub fn numero(&self) -> Option<Numero> {
        (self.tipo == Tipo::Loyal).then(|| NUMEROS[self.lanzados as usize % NUMEROS.len()])
    }
}

/// Algo que le tiran a la jugadora: los platos del camarero.
#[derive(Clone, Copy, Debug)]
pub struct Proyectil {
    pub pos: Vec2,
    pub vel: Vec2,
    pub rosa: bool,
    pub forma: Forma,
}

/// Un disparo de la jugadora.
#[derive(Clone, Copy, Debug)]
pub struct Disparo {
    pub pos: Vec2,
    pub vel: Vec2,
    ttl: f32,
    /// La rosa devuelta de un parry: pasa el poncho y pega fuerte.
    pub rosa: bool,
}

/// Todo el estado de un paseo.
#[derive(Clone, Debug)]
pub struct Paseo {
    pub def: PaseoDef,
    pub tick: u64,
    /// La misma `Player` de los combates, para que el dibujo —la pose, la
    /// falda, el parpadeo de los i-frames— valga sin cambiar nada. Lo que no se
    /// reusa es su `update`, que esta atado a la arena de 640 y a un suelo
    /// plano (ver `mover`).
    pub jugadora: Player,
    pub vidas: u32,
    /// El tiro y el amuleto que se llevan, igual que en el
    /// combate.
    pub equipo: Equipo,
    /// Las fichas de `def.fichas` que ya no estan en la calle: cogidas en este
    /// paseo, o en uno anterior (eso lo pone la app antes de empezar).
    pub fichas: Vec<bool>,
    /// Hacia donde dispara: de frente, o arriba si se planta y apunta.
    pub apunta: Vec2,
    pub enemigos: Vec<Enemigo>,
    pub proyectiles: Vec<Proyectil>,
    /// Lo fugaz del cancan: volantes, corchos, burbujas, pintura.
    pub fugaces: Vec<Fugaz>,
    pub disparos: Vec<Disparo>,
    /// Tick en que se llego a la puerta.
    pub completado: Option<u64>,
    pub derrota: bool,
    /// Lo que ha pasado en el ultimo tick. Es el mismo `Events` de los
    /// combates, asi que el sonido y el zumo lo entienden sin traducir nada:
    /// `boss_hit` es "un disparo ha dado a un enemigo", `player_died` "te han
    /// dado" y `victory` "has llegado a la puerta".
    ///
    /// `boss_down` es solo para el portero: en el combate es el final de un
    /// jefe, con la sacudida y el frenazo mas gordos del juego, y aqui caen
    /// veinte enemigos por calle. Para los demas esta `caidos`.
    pub events: Events,
    /// Enemigos que han caido en el ultimo tick. Como `events`, no es estado.
    pub caidos: u32,
    /// Donde fue el ultimo impacto o la ultima caida de un enemigo, para las
    /// chispas. Como `events`, no es estado: no entra en la huella.
    pub impacto: Vec2,
    /// La siguiente aparicion por despertar. Las apariciones van ordenadas por
    /// x, asi que basta un indice.
    siguiente: usize,
    /// El ultimo sitio de acera pisado, de donde se vuelve tras caer a un foso.
    seguro: Vec2,
    /// Tras caer a un foso, el lado no cuenta hasta que se suelta. Quien se
    /// cae suele seguir con el stick a la derecha sin darse cuenta de que ha
    /// vuelto a la acera, y se tiraba otra vez al mismo foso: tres caidas
    /// seguidas y fuera, sin haber podido reaccionar. Es lo que hacen los
    /// juegos de plataformas: despues de reaparecer hay que volver a pulsar.
    soltar: bool,
    /// Donde estan las plataformas segun el RON. Las que se mueven se
    /// recolocan cada tick a partir de aqui.
    bases: Vec<Vec2>,
    /// La plataforma que pisa, para llevarla con ella si se mueve.
    sobre: Option<usize>,
    /// Lo mas a la izquierda que se puede ir. Cero hasta que se cierra la
    /// arena del portero; desde entonces, su borde: lo de detras no vuelve.
    muro: f32,
    prev_input: InputFrame,
    rng: Pcg32,
}

impl Paseo {
    /// Un paseo sin equipo, que es como se anda si no se ha comprado nada.
    pub fn new(def: PaseoDef, seed: u64) -> Self {
        Self::con_equipo(def, seed, Equipo::default())
    }

    pub fn con_equipo(mut def: PaseoDef, seed: u64, equipo: Equipo) -> Self {
        // Ordenadas por x: es lo que deja despertarlas con un solo indice. Con
        // `total_cmp` para que un NaN en el RON no cambie el orden segun la
        // plataforma, y estable para que dos en la misma x salgan como se
        // escribieron.
        def.enemigos.sort_by(|a, b| a.x.total_cmp(&b.x));
        let salida = Vec2::new(def.salida, def.altura(def.salida) - PIES);
        let mut jugadora = Player::new(salida);
        jugadora.facing = Vec2::new(1.0, 0.0);
        jugadora.on_ground = true;
        let fichas = vec![false; def.fichas.len()];
        let bases = def
            .plataformas
            .iter()
            .map(|pl| Vec2::new(pl.x, pl.y))
            .collect();
        let mut p = Self {
            def,
            tick: 0,
            jugadora,
            vidas: VIDAS + equipo.amuleto.vidas_extra(),
            equipo,
            fichas,
            apunta: Vec2::new(1.0, 0.0),
            enemigos: Vec::with_capacity(32),
            proyectiles: Vec::with_capacity(32),
            fugaces: Vec::with_capacity(64),
            disparos: Vec::with_capacity(MAX_DISPAROS),
            completado: None,
            derrota: false,
            events: Events::default(),
            caidos: 0,
            impacto: salida,
            siguiente: 0,
            seguro: salida,
            soltar: false,
            bases,
            sobre: None,
            muro: 0.0,
            prev_input: InputFrame::NONE,
            rng: Pcg32::new(seed),
        };
        p.colocar_plataformas(0);
        p
    }

    /// Rehace el nivel con otra definicion, con la jugadora donde estaba.
    ///
    /// Es el hot-reload: al afinar el tercer tramo no se quiere volver a
    /// andar los dos primeros cada vez que se guarda. Lo que queda por detras
    /// no se despierta.
    pub fn recargar(&mut self, def: PaseoDef) {
        let (pos, vidas) = (self.jugadora.pos, self.vidas);
        let (seguro, mut fichas) = (self.seguro, std::mem::take(&mut self.fichas));
        *self = Self::con_equipo(def, self.rng.next_u32().into(), self.equipo);
        fichas.resize(self.def.fichas.len(), false);
        self.fichas = fichas;
        self.jugadora.pos = pos;
        self.jugadora.prev_pos = pos;
        self.vidas = vidas;
        self.seguro = seguro;
        while self
            .def
            .enemigos
            .get(self.siguiente)
            .is_some_and(|a| a.x < pos.x)
        {
            self.siguiente += 1;
        }
    }

    /// Si ya no se juega: llegaste a la puerta o te quedaste sin vidas.
    pub fn terminado(&self) -> bool {
        self.completado.is_some() || self.derrota
    }

    /// Cuanto del camino a la puerta llevas, de 0 a 1.
    pub fn progreso(&self) -> f32 {
        let total = (self.def.puerta - self.def.salida).max(1.0);
        ((self.jugadora.pos.x - self.def.salida) / total).clamp(0.0, 1.0)
    }

    /// Da el paseo por andado. La trampa de F3, igual que saltarse un baile:
    /// ver como avanza el juego no deberia costar pasarse la calle cada vez.
    pub fn saltar(&mut self) {
        if self.terminado() {
            return;
        }
        let p = Vec2::new(self.def.puerta, self.def.altura(self.def.puerta) - PIES);
        self.jugadora.pos = p;
        self.jugadora.prev_pos = p;
        self.llegar();
    }

    /// Planta a la jugadora a la entrada de la arena del portero, con lo de
    /// antes sin despertar. Es para probar porteros sin andar la calle: lo
    /// llama F5 con `--pruebas`. Sin porteria no hace nada.
    pub fn a_la_porteria(&mut self) {
        let Some(po) = self.def.porteria else { return };
        if self.terminado() {
            return;
        }
        let x = po.desde + 40.0;
        let p = Vec2::new(x, self.def.altura(x) - PIES);
        self.jugadora.pos = p;
        self.jugadora.prev_pos = p;
        self.jugadora.vel = Vec2::ZERO;
        self.seguro = p;
        self.enemigos.retain(|e| e.portero);
        self.proyectiles.clear();
        self.fugaces.clear();
        while self
            .def
            .enemigos
            .get(self.siguiente)
            .is_some_and(|a| a.x < po.desde && !a.portero)
        {
            self.siguiente += 1;
        }
    }

    /// Avanza un tick. Mismo estado mas mismo input, mismo resultado.
    pub fn step(&mut self, input: InputFrame) {
        self.events = Events::default();
        self.caidos = 0;
        if self.terminado() {
            // Se queda quieta en la puerta o donde cayo; el resto del mundo
            // tambien, que la escena ya ha acabado.
            let p = &mut self.jugadora;
            p.prev_pos = p.pos;
            p.vel = Vec2::ZERO;
            self.tick += 1;
            return;
        }

        let input = self.sin_lado_hasta_soltar(input);
        self.mover_plataformas();
        self.mover(input);
        self.cerrar_arena();
        self.caida();
        self.recoger();
        self.disparar(input);
        self.parry();
        self.super_(input);
        self.despertar();
        self.enemigos_actuan();
        self.mover_proyectiles();
        self.mover_fugaces();
        self.mover_disparos();
        self.recibir();
        self.olvidar();
        if self.jugadora.pos.x >= self.def.puerta && !self.portero_vivo() {
            self.llegar();
        }

        self.prev_input = input;
        self.tick += 1;
    }

    /// Pone las plataformas que se mueven donde tocan en el tick `t`, y
    /// devuelve cuanto se ha movido la que pisa la jugadora.
    fn colocar_plataformas(&mut self, t: u64) -> Vec2 {
        let mut arrastre = Vec2::ZERO;
        for (i, pl) in self.def.plataformas.iter_mut().enumerate() {
            let Some(m) = pl.mueve else { continue };
            let nueva = self.bases[i] + m.desvio(t);
            if self.sobre == Some(i) {
                arrastre = nueva - Vec2::new(pl.x, pl.y);
            }
            (pl.x, pl.y) = (nueva.x, nueva.y);
        }
        arrastre
    }

    /// Las plataformas al tick que empieza, llevandose a quien este encima.
    /// Va antes de la fisica: la jugadora se mueve con el fiacre y luego
    /// anda sobre el, como en cualquier plataformas.
    fn mover_plataformas(&mut self) {
        let arrastre = self.colocar_plataformas(self.tick + 1);
        if self.jugadora.on_ground {
            self.jugadora.pos += arrastre;
        }
    }

    /// Cierra la arena del portero cuando la camara llega a ella: con la
    /// jugadora a `ENCUADRE` de la vista, la camara empieza justo en `desde`.
    fn cerrar_arena(&mut self) {
        if let Some(po) = self.def.porteria
            && self.muro < po.desde
            && self.jugadora.pos.x >= po.desde + VISTA_W * ENCUADRE
            && self.portero_vivo()
        {
            self.muro = po.desde;
        }
    }

    /// Si queda portero por tumbar, despierto o sin despertar.
    pub fn portero_vivo(&self) -> bool {
        self.enemigos.iter().any(|e| e.portero)
            || self.def.enemigos[self.siguiente..]
                .iter()
                .any(|a| a.portero)
    }

    /// El borde izquierdo de la arena mientras esta cerrada.
    pub fn cierre(&self) -> Option<f32> {
        let po = self.def.porteria?;
        (self.muro >= po.desde && self.portero_vivo()).then_some(po.desde)
    }

    /// Lo mas a la izquierda que puede ir la camara: tras la arena, su borde.
    pub fn muro(&self) -> f32 {
        self.muro
    }

    /// El portero despierto, para su barra de vida.
    pub fn portero(&self) -> Option<&Enemigo> {
        self.enemigos.iter().find(|e| e.portero)
    }

    /// Si la ficha `i` esta en la calle: sin coger, y si es la del portero,
    /// con el portero ya tumbado.
    pub fn ficha_visible(&self, i: usize) -> bool {
        let del_portero = self.def.porteria.and_then(|po| po.ficha) == Some(i);
        !self.fichas[i] && !(del_portero && self.portero_vivo())
    }

    /// Las fichas que toca, se las lleva. Aunque sea en un dash o recien
    /// golpeada: cogerla es pasar por donde esta, y el riesgo es llegar.
    fn recoger(&mut self) {
        for i in 0..self.fichas.len() {
            let (x, y) = self.def.fichas[i];
            if self.ficha_visible(i) && self.toca(Vec2::new(x, y), RADIO_FICHA) {
                self.fichas[i] = true;
                self.events.fichas += 1;
            }
        }
    }

    /// Cuantas fichas de este paseo estan ya fuera de la calle.
    pub fn fichas_cogidas(&self) -> usize {
        self.fichas.iter().filter(|f| **f).count()
    }

    fn llegar(&mut self) {
        self.completado = Some(self.tick);
        self.events.victory = true;
        // Lo que venia volando ya no llega: se acabo la calle.
        self.proyectiles.clear();
        self.fugaces.clear();
    }

    /// La fisica de la jugadora en la calle.
    ///
    /// Es la de `Player::update_gravity` —los mismos numeros de `player.rs`:
    /// gravedad, salto variable, coyote, buffer, dash— con dos cambios que no
    /// caben alli sin tocar el combate: el suelo tiene fosos y plataformas, y
    /// no hay paredes a 640. Copiar la fisica y no reusarla tiene un precio
    /// (afinar el salto en un sitio no lo afina en el otro), pero la
    /// alternativa era meter geometria en la simulacion del jefe, que tiene un
    /// replay dorado detras. Los numeros son los mismos porque son los mismos
    /// `const`.
    fn mover(&mut self, input: InputFrame) {
        const ACCEL: f32 = PLAYER_SPEED / (PLAYER_ACCEL_TICKS * DT);
        const DECEL: f32 = PLAYER_SPEED / (PLAYER_DECEL_TICKS * DT);
        let prev = self.prev_input;
        let flanco = |b: u16| input.is_down(b) && !prev.is_down(b);
        let izquierda = self.muro + CUERPO_RADIO;
        let derecha = match self.cierre() {
            Some(a) => a + VISTA_W,
            None => self.def.largo,
        } - CUERPO_RADIO;
        let p = &mut self.jugadora;
        p.prev_pos = p.pos;

        // Los contadores bajan al principio, como en `Player::update`.
        p.dash.ticks_left = p.dash.ticks_left.saturating_sub(1);
        p.dash.cooldown = p.dash.cooldown.saturating_sub(1);
        p.dash.buffer = p.dash.buffer.saturating_sub(1);
        p.iframes = p.iframes.saturating_sub(1);
        p.shot_cooldown = p.shot_cooldown.saturating_sub(1);
        p.parry_window = p.parry_window.saturating_sub(1);
        p.parry_cooldown = p.parry_cooldown.saturating_sub(1);
        p.super_ticks = p.super_ticks.saturating_sub(1);
        p.coyote = p.coyote.saturating_sub(1);
        p.jump_buffer = p.jump_buffer.saturating_sub(1);

        // FOCUS es **plantarse**, el boton de fijar de Cuphead: quieta en el
        // suelo y apuntando con la cruceta. Hace falta un boton aparte porque
        // arriba ya es saltar, y apuntar arriba sin plantarse haria saltar a
        // la vez. Plantada, arriba apunta y no salta.
        let plantada = input.is_down(InputFrame::FOCUS);
        p.focused = plantada;
        let rampa = 1.0 / FOCUS_RAMP_TICKS;
        p.focus_t = if plantada {
            (p.focus_t + rampa).min(1.0)
        } else {
            (p.focus_t - rampa).max(0.0)
        };

        let (ax, ay) = input.axis();
        if ax != 0.0 {
            p.facing = Vec2::new(ax, 0.0);
        }
        self.apunta = if plantada && ay < 0.0 {
            Vec2::new(ax, -1.0).normalize()
        } else {
            Vec2::new(p.facing.x, 0.0)
        };

        if flanco(InputFrame::PARRY) && p.parry_cooldown == 0 {
            p.parry_window = self.equipo.amuleto.ventana_parry();
            p.parry_cooldown = PARRY_COOLDOWN_TICKS;
        }
        if flanco(InputFrame::DASH) {
            p.dash.buffer = INPUT_BUFFER_TICKS;
        }
        if p.dash.buffer > 0 && p.dash.ticks_left == 0 && p.dash.cooldown == 0 {
            p.dash.buffer = 0;
            p.dash.ticks_left = DASH_TICKS;
            p.dash.cooldown = DASH_COOLDOWN_TICKS;
            p.iframes = p.iframes.max(DASH_IFRAME_TICKS);
            p.dash.dir = Vec2::new(p.facing.x, 0.0);
        }
        if flanco(InputFrame::JUMP) && !plantada {
            p.jump_buffer = JUMP_BUFFER_TICKS;
        }

        if p.is_dashing() {
            // Horizontal y sin gravedad: cruzar un foso con dash es la jugada.
            p.vel = Vec2::new(p.dash.dir.x * DASH_SPEED, 0.0);
        } else {
            let quieta = plantada && p.on_ground;
            let objetivo = if quieta { 0.0 } else { ax * PLAYER_SPEED };
            let rate = if ax == 0.0 || quieta {
                DECEL
            } else if p.on_ground {
                ACCEL
            } else {
                ACCEL * AIR_CONTROL
            };
            p.vel.x = mover_hacia(p.vel.x, objetivo, rate * DT);

            if p.jump_buffer > 0 && (p.on_ground || p.coyote > 0) {
                p.vel.y = -JUMP_SPEED;
                p.jump_buffer = 0;
                p.coyote = 0;
                p.on_ground = false;
            }
            let g = if p.vel.y < 0.0 && !input.is_down(InputFrame::JUMP) {
                GRAVITY * JUMP_CUT
            } else {
                GRAVITY
            };
            // Con tope de caida: en un foso hondo la velocidad creceria sin
            // limite y un tick se saltaria una plataforma entera.
            p.vel.y = (p.vel.y + g * DT).min(JUMP_SPEED * 1.6);
        }

        // Primero en x, con los fosos haciendo de pared: con los pies por
        // debajo de la acera, un tramo de acera es un muro. El muro se mide
        // con el centro y no con el ancho del cuerpo: con el ancho, al bajar
        // del borde andando, el cuerpo aun solapa la acera y el muro la
        // empujaba de golpe hacia el foso.
        //
        // Solo al **entrar** al tramo, y a la altura del borde por donde se
        // entra. Con escaleras hay pies por debajo de la acera sin foso
        // ninguno —saltando contra una rampa que sube— y eso se pisa (abajo),
        // no se choca: mirar todo el tramo la mandaba al otro extremo.
        const PARED: f32 = 2.0;
        let def = &self.def;
        p.pos.x = (p.pos.x + p.vel.x * DT).clamp(izquierda, derecha);
        let (antes, pies) = (p.prev_pos.x, p.pos.y + PIES);
        for &(a, b) in &def.suelo {
            let fuera = antes < a || antes > b;
            let borde = if antes <= a { a } else { b };
            if fuera && pies > def.altura(borde) + 1.0 && p.pos.x + PARED > a && p.pos.x - PARED < b
            {
                p.pos.x = if antes <= a { a - PARED } else { b + PARED };
                p.vel.x = 0.0;
            }
        }

        // Luego en y. Solo se aterriza bajando y cruzando la superficie en este
        // tick: es lo que deja atravesar las plataformas desde abajo.
        let pies_antes = p.prev_pos.y + PIES;
        let pisaba = p.on_ground;
        p.pos.y += p.vel.y * DT;
        p.on_ground = false;
        if p.vel.y >= 0.0 {
            let pies = p.pos.y + PIES;
            let cruza = |y: f32| pies_antes <= y + 0.5 && pies >= y;
            let x = p.pos.x;
            let mut pisa = None;
            if let Some(y) = def.suelo_en(x) {
                // En una escalera la acera se mueve bajo los pies: subiendo, se
                // mete por encima de ellos, y bajando se va por debajo mas
                // deprisa que la gravedad. Se perdona justo lo que se ha
                // movido en este tick, asi que en llano es el cruce de siempre.
                let margen = 0.5 + (def.altura(p.prev_pos.x) - y).abs();
                if pies_antes <= y + margen && (pies >= y || pisaba && pies >= y - margen) {
                    pisa = Some(y);
                }
            }
            let mut sobre = None;
            for (i, pl) in def.plataformas.iter().enumerate() {
                // Una que se mueve puede subir por encima de los pies en el
                // mismo tick en que se cae sobre ella: se perdona su paso.
                let entra = cruza(pl.y) || pl.mueve.is_some() && cruza(pl.y - 3.0);
                if entra && x >= pl.x && x <= pl.x + pl.ancho && pisa.is_none_or(|y| pl.y < y) {
                    // Si hay dos, la de mas arriba: es la que se ha cruzado
                    // primero.
                    pisa = Some(pl.y);
                    sobre = Some(i);
                }
            }
            if pisa.is_some_and(|y| def.suelo_en(x) == Some(y)) {
                sobre = None;
            }
            self.sobre = sobre;
            if let Some(y) = pisa {
                p.pos.y = y - PIES;
                p.vel.y = 0.0;
                p.on_ground = true;
                p.coyote = COYOTE_TICKS;
                if def.suelo_en(x) == Some(y)
                    && let Some(&(a, b)) = def.suelo.iter().find(|&&(a, b)| x >= a && x <= b)
                {
                    // El sitio seguro, metido hacia dentro del tramo: volver
                    // al borde exacto del foso seria volver a caerse.
                    let margen = 50.0_f32.min((b - a) * 0.5);
                    let sx = x.clamp(a + margen, b - margen);
                    self.seguro = Vec2::new(sx, def.altura(sx) - PIES);
                } else if !def.hay_suelo(x)
                    && let Some(pl) = def.plataformas.iter().find(|pl| {
                        pl.mueve.is_none() && pl.y == y && x >= pl.x && x <= pl.x + pl.ancho
                    })
                {
                    // Sobre el vacio, la plataforma es el suelo: en las azoteas
                    // de Chicago no hay acera en toda la manzana, y caerse entre
                    // dos tejados no puede devolverte al pie de la escalera de
                    // incendios. Se vuelve al ultimo tejado pisado, tambien
                    // metido hacia dentro.
                    let margen = 40.0_f32.min(pl.ancho * 0.5);
                    self.seguro =
                        Vec2::new(x.clamp(pl.x + margen, pl.x + pl.ancho - margen), y - PIES);
                }
            }
        }
    }

    /// Tras reaparecer, quita izquierda, derecha y dash hasta que se suelte el
    /// lado. Saltar, disparar y lo demas siguen valiendo.
    fn sin_lado_hasta_soltar(&mut self, input: InputFrame) -> InputFrame {
        const LADO: u16 = InputFrame::LEFT | InputFrame::RIGHT;
        if !self.soltar {
            return input;
        }
        if input.bits() & LADO == 0 {
            self.soltar = false;
            return input;
        }
        InputFrame::from_bits(input.bits() & !(LADO | InputFrame::DASH))
    }

    /// Caer a un foso cuesta un golpe y te devuelve a la ultima acera pisada.
    fn caida(&mut self) {
        let p = &self.jugadora;
        if p.pos.y + PIES < self.def.altura(p.pos.x) + CAIDA {
            return;
        }
        let p = &mut self.jugadora;
        p.pos = self.seguro;
        p.prev_pos = self.seguro;
        p.vel = Vec2::ZERO;
        p.on_ground = true;
        self.soltar = true;
        self.golpe();
    }

    fn golpe(&mut self) {
        self.vidas = self.vidas.saturating_sub(1);
        self.events.player_died = true;
        let p = &mut self.jugadora;
        p.iframes = INVULNERABLE_TRAS_GOLPE;
        p.dash = Dash::default();
        if self.vidas == 0 {
            self.derrota = true;
            self.events.defeat = true;
        }
    }

    fn disparar(&mut self, input: InputFrame) {
        let p = &mut self.jugadora;
        if !input.is_down(InputFrame::SHOOT)
            || p.shot_cooldown > 0
            || self.disparos.len() >= MAX_DISPAROS
        {
            return;
        }
        let tiro = self.equipo.tiro;
        p.shot_cooldown = tiro.cada_calle();
        self.events.player_shot = true;
        // Sale de la mano, a media altura: de frente pasa por encima de una
        // nota baja y le da a una pareja en el pecho.
        let (pos, ttl) = (self.boca(), tiro.ttl_calle());
        let disparo = |vel: Vec2| Disparo {
            pos,
            vel,
            ttl,
            rosa: false,
        };
        if tiro == Tiro::Abanico {
            // Tres, abiertos hacia donde apunta: el de arriba alcanza lo que
            // vuela y el de abajo lo que viene por la acera sin plantarse.
            for a in [-ABANICO, 0.0, ABANICO] {
                let (s, c) = sin_cos(a);
                let d = self.apunta;
                let vel = Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c) * SHOT_SPEED;
                self.disparos.push(disparo(vel));
            }
        } else {
            self.disparos.push(disparo(self.apunta * SHOT_SPEED));
        }
    }

    /// De donde salen los disparos. El dibujo la usa para el fogonazo.
    pub fn boca(&self) -> Vec2 {
        self.jugadora.pos + Vec2::new(0.0, -6.0) + self.apunta * 18.0
    }

    /// El parry de los combates, contra lo rosa de la calle: los platos rosas
    /// y las notas rosas. Si se acierta en el aire, rebota, que es el gesto del
    /// parry de Cuphead y lo que deja encadenar dos.
    fn parry(&mut self) {
        let p = &mut self.jugadora;
        if !p.is_parrying() {
            return;
        }
        let centro = p.pos;
        let mut n = 0u32;
        self.proyectiles.retain(|q| {
            let dentro = q.rosa && q.pos.distance_squared(centro) <= PARRY_RADIUS * PARRY_RADIUS;
            n += u32::from(dentro);
            !dentro
        });
        self.fugaces.retain(|f| {
            let dentro = f.rosa && f.pos.distance_squared(centro) <= PARRY_RADIUS * PARRY_RADIUS;
            n += u32::from(dentro);
            !dentro
        });
        for e in &mut self.enemigos {
            let r = PARRY_RADIUS + e.tipo.radio();
            if e.rosa && e.vida > 0 && e.pos.distance_squared(centro) <= r * r {
                e.vida = 0;
                n += 1;
            }
        }
        if n == 0 {
            return;
        }
        p.on_parry(n);
        self.devolver(centro);
        let p = &mut self.jugadora;
        if !p.on_ground {
            p.vel.y = -JUMP_SPEED * 0.8;
        }
        self.events.parried += n;
        self.impacto = centro;
        self.retirar_caidos();
    }

    /// La rosa devuelta: lo rosa que se parria sale de vuelta hacia el malevo
    /// o el guapo mas cercano que este a la vista. Es lo que tumba al malevo
    /// de frente, y lo que hace que en el arrabal el parry pague.
    fn devolver(&mut self, desde: Vec2) {
        let arena = self.cierre().is_some();
        let blanco = self
            .enemigos
            .iter()
            .filter(|e| {
                matches!(e.tipo, Tipo::Malevo | Tipo::Guapo)
                    && e.tocable(arena)
                    && (e.pos.x - desde.x).abs() < ALCANCE_DEVUELTA
            })
            .min_by(|a, b| {
                let (da, db) = (a.pos.distance_squared(desde), b.pos.distance_squared(desde));
                da.total_cmp(&db)
            });
        if let Some(e) = blanco {
            self.disparos.push(Disparo {
                pos: desde,
                vel: (e.pos - desde).normalize_or(Vec2::X) * VEL_DEVUELTA,
                ttl: 1.0,
                rosa: true,
            });
        }
    }

    /// El super, igual que en los combates: medidor lleno, todo o nada. Aqui
    /// limpia lo que vuela y pega a todo lo que se ve.
    fn super_(&mut self, input: InputFrame) {
        let pulsado =
            input.is_down(InputFrame::SUPER) && !self.prev_input.is_down(InputFrame::SUPER);
        if !pulsado || !self.jugadora.spend_meter() {
            return;
        }
        self.events.super_fired = true;
        self.proyectiles.clear();
        self.fugaces.clear();
        let x = self.jugadora.pos.x;
        let arena = self.cierre().is_some();
        for e in &mut self.enemigos {
            if (e.pos.x - x).abs() < VISTA_W * 0.6 && e.tocable(arena) {
                e.vida -= DANO_SUPER;
                e.golpe = 8;
            }
        }
        self.retirar_caidos();
    }

    /// Despierta lo que tiene la jugadora a tiro de vista.
    fn despertar(&mut self) {
        let frente = self.jugadora.pos.x + ACTIVAR;
        while let Some(a) = self.def.enemigos.get(self.siguiente).copied() {
            if a.x > frente {
                break;
            }
            self.siguiente += 1;
            // El husar despierta como todos, pero sale por detras de ella.
            let x = if a.tipo == Tipo::Husar {
                self.jugadora.pos.x - DETRAS_HUSAR
            } else {
                a.x
            };
            // Distinto de cero y no mayor: en lo alto de una escalera se pisa
            // por encima de la lamina, en y negativas.
            let pisa = if a.y != 0.0 { a.y } else { self.def.altura(x) };
            let y = match a.tipo {
                Tipo::Hoja => a.y,
                Tipo::Canotier => a.y,
                t => pisa - t.alto(),
            };
            // El primer plato sale con un poco de azar: dos camareros que se
            // despiertan juntos no deben tirar a la vez.
            let mut reloj = CADENCIA_CAMARERO / 2 + self.rng.next_u32() % 30;
            if matches!(
                a.tipo,
                Tipo::Compadrito | Tipo::Gaviota | Tipo::Malevo | Tipo::Guapo
            ) {
                // El compadrito espera a tenerte cerca y a que caiga un tiempo.
                reloj = 0;
            }
            if matches!(a.tipo, Tipo::Apache | Tipo::Loyal) {
                // `reloj` es su ciclo, que empieza a cero.
                reloj = 0;
            }
            self.enemigos.push(Enemigo {
                tipo: a.tipo,
                pos: Vec2::new(x, y),
                vida: a.tipo.vida(),
                dir: -1.0,
                nacio: self.tick,
                rosa: a.rosa && matches!(a.tipo, Tipo::Hoja | Tipo::Confeti | Tipo::Canotier),
                golpe: 0,
                reloj,
                lanzados: 0,
                base_y: y,
                avance: 0.0,
                gesto: 0.0,
                vel: Vec2::ZERO,
                portero: a.portero,
            });
            // La gaviota rosa se parria, como la nota rosa.
            if a.tipo == Tipo::Gaviota
                && let Some(e) = self.enemigos.last_mut()
            {
                e.rosa = a.rosa;
            }
            // El canotier vuela alrededor de donde salio, y la fila entera
            // va con el centro de la fila: las dos cosas, en `avance`.
            let primera = self.enemigos.len() - 1;
            if matches!(a.tipo, Tipo::Canotier | Tipo::Fila) {
                self.enemigos[primera].avance = a.x;
            }
            if a.tipo == Tipo::Fila {
                // La fila sale entera de una aparicion, y cada una sabe su
                // sitio (`lanzados`): despertarlas por separado las
                // desacompasaria.
                for k in 1..FILA {
                    let mut e = self.enemigos[primera].clone();
                    e.lanzados = k;
                    self.enemigos.push(e);
                }
            }
        }
    }

    fn enemigos_actuan(&mut self) {
        let objetivo = self.jugadora.pos;
        let tiempo = u64::from(self.def.tiempo.max(1));
        // Lo que nace este tick (las rosas). Se anade al acabar, que no se
        // puede empujar a la lista que se esta recorriendo.
        let mut nuevas = Vec::new();
        // El portero no empieza hasta que se cierra su arena.
        let arena = self.cierre().is_some();
        // La pista de Monsieur Loyal, si esta cerrada.
        let (pista_loyal, muro) = (self.cierre(), self.muro);
        // La arena del portero, si esta cerrada: el guapo espera a eso.
        let arena_guapo = self.cierre();
        for e in &mut self.enemigos {
            e.golpe = e.golpe.saturating_sub(1);
            let edad = self.tick - e.nacio;
            match e.tipo {
                Tipo::Pareja | Tipo::Husar => {
                    // Un vals: en el uno del compas se vuelve hacia ti y da el
                    // paso largo; en el dos y el tres, pasitos. Es el tres por
                    // cuatro hecho andar, y se lee: sabes cuando va a acelerar.
                    // El husar, igual pero a la carga.
                    let fase = edad % (3 * tiempo);
                    if fase == 0 && objetivo.x != e.pos.x {
                        e.dir = (objetivo.x - e.pos.x).signum();
                    }
                    let (largo, corto) = if e.tipo == Tipo::Husar {
                        (PASO_LARGO_HUSAR, PASO_CORTO_HUSAR)
                    } else {
                        (170.0, 40.0)
                    };
                    let v = if fase < tiempo { largo } else { corto };
                    let nx = e.pos.x + e.dir * v * DT;
                    // No se tira al canal: en el borde de la acera da la vuelta.
                    if self.def.hay_suelo(nx + e.dir * e.tipo.radio()) {
                        e.pos.x = nx;
                    } else {
                        e.dir = -e.dir;
                    }
                }
                Tipo::Camarero => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if dx.abs() > ALCANCE_CAMARERO {
                        continue;
                    }
                    e.reloj = e.reloj.saturating_sub(1);
                    if e.reloj > 0 {
                        continue;
                    }
                    e.reloj = CADENCIA_CAMARERO;
                    e.lanzados += 1;
                    // El arco se calcula para caer donde estabas al tirarlo, en
                    // un tiempo fijo. Si te mueves, falla: se esquiva andando,
                    // que es lo que se le pide a un plato.
                    let origen = e.pos + Vec2::new(e.dir * 14.0, -26.0);
                    let t = VUELO_PLATO;
                    let vx = ((objetivo.x - origen.x) / t).clamp(-460.0, 460.0);
                    let dy = SUELO_Y - 6.0 - origen.y;
                    let vy = (dy - 0.5 * GRAVEDAD_PLATO * t * t) / t;
                    self.proyectiles.push(Proyectil {
                        pos: origen,
                        vel: Vec2::new(vx, vy),
                        rosa: e.lanzados % 3 == 0,
                        forma: Forma::Plato,
                    });
                }
                Tipo::Hoja => {
                    // Una vuelta por compas alrededor de un centro que baja y
                    // deriva a la izquierda: en x se suma la velocidad de la
                    // vuelta, en y se pone donde toca.
                    let w = TAU / (3 * tiempo) as f32;
                    let (s, c) = sin_cos(w * edad as f32);
                    e.pos.x += -DERIVA_HOJA * DT + GIRO_HOJA_X * w * c;
                    e.pos.y = e.base_y + CAIDA_HOJA * edad as f32 * DT + GIRO_HOJA_Y * s;
                    // Hacia donde va en la vuelta: el dibujo le da la cara.
                    e.dir = if c >= 0.0 { 1.0 } else { -1.0 };
                }
                Tipo::Director => {
                    e.gesto = (e.gesto - 1.0 / 12.0).max(0.0);
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if !arena {
                        continue;
                    }
                    let fase = e.fase();
                    let (golpe, dentro, prepara) = batuta(self.tick, self.def.tiempo, fase);
                    if prepara {
                        // Un compas quieto, con los brazos arriba: el aviso.
                        // `reloj` a 1 es "remolino preparado".
                        e.reloj = 1;
                        continue;
                    }
                    if dentro != 0 {
                        continue;
                    }
                    e.gesto = 1.0;
                    e.lanzados += 1;
                    let mano = e.mano();
                    let giro = giro_batuta(self.tick as f32, self.def.tiempo);
                    if golpe == 0 && e.reloj == 1 {
                        // El remolino: un anillo entero que sale girando.
                        e.reloj = 0;
                        for k in 0..NOTAS_REMOLINO {
                            let a = giro + TAU * k as f32 / NOTAS_REMOLINO as f32;
                            let (s, c) = sin_cos(a);
                            let d = Vec2::new(c, s);
                            self.proyectiles.push(Proyectil {
                                pos: e.pos + d * 34.0,
                                vel: d * VEL_REMOLINO,
                                rosa: false,
                                forma: Forma::Remolino,
                            });
                        }
                    } else if golpe == 2 {
                        // El tres: una sola nota, rosa, a donde estas.
                        let d = (objetivo - mano).normalize_or(Vec2::new(e.dir, 0.0));
                        self.proyectiles.push(Proyectil {
                            pos: mano,
                            vel: d * VEL_TRES,
                            rosa: true,
                            forma: Forma::Batuta,
                        });
                    } else {
                        // El uno y el dos: la espiral, por donde va la punta.
                        let n = BRAZOS_ESPIRAL[fase];
                        for k in 0..n {
                            let (s, c) = sin_cos(giro + TAU * k as f32 / n as f32);
                            let d = Vec2::new(c, s);
                            self.proyectiles.push(Proyectil {
                                pos: mano + d * BATUTA,
                                vel: d * VEL_ESPIRAL[fase],
                                rosa: false,
                                forma: Forma::Batuta,
                            });
                        }
                    }
                }
                Tipo::Flapper => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    let (golpe, dentro, _) = clave(self.tick, self.def.tiempo);
                    if dentro != 0 || dx.abs() > ALCANCE_CHICAGO {
                        continue;
                    }
                    // El collar gira alrededor del cuello, y en el golpe se
                    // sueltan las perlas por donde pasa. Tres, repartidas: una
                    // espiral que avanza a saltos de tres, tres y dos.
                    let giro = giro_collar(self.tick as f32, self.def.tiempo);
                    let cuello = e.pos + Vec2::new(0.0, -12.0);
                    for k in 0..PERLAS_POR_GOLPE {
                        let a = giro + TAU * k as f32 / PERLAS_POR_GOLPE as f32;
                        let (s, c) = sin_cos(a);
                        let dir = Vec2::new(c, s);
                        self.proyectiles.push(Proyectil {
                            pos: cuello + dir * 24.0,
                            vel: dir * VEL_PERLA,
                            rosa: golpe == 2 && k == 0,
                            forma: Forma::Perla,
                        });
                    }
                    e.lanzados += 1;
                }
                Tipo::Saxo => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if dx.abs() > ALCANCE_CHICAGO {
                        continue;
                    }
                    // Tres, tres y dos notas: una rafaga por golpe de clave,
                    // cada nota apuntada a donde estas al soplarla. Se esquiva
                    // andando, igual que el plato.
                    let (golpe, dentro, _) = clave(self.tick, self.def.tiempo);
                    let notas = if golpe == 2 { 2 } else { 3 };
                    if dentro % SEPARACION_SAXO != 0 || dentro / SEPARACION_SAXO >= notas {
                        continue;
                    }
                    let boca = e.pos + Vec2::new(e.dir * 20.0, 6.0);
                    let dir = (objetivo - boca).normalize_or(Vec2::new(e.dir, 0.0));
                    // La ultima del compas, rosa: una por compas.
                    let rosa = golpe == 2 && dentro / SEPARACION_SAXO == notas - 1;
                    self.proyectiles.push(Proyectil {
                        pos: boca,
                        vel: dir * VEL_SAXO,
                        rosa,
                        forma: Forma::Corchea,
                    });
                    e.lanzados += 1;
                }
                Tipo::Saltarin => {
                    let (golpe, dentro, largo) = clave(self.tick, self.def.tiempo);
                    let alto = if golpe == 2 {
                        BRINCO_CORTO
                    } else {
                        BRINCO_LARGO
                    };
                    let vuelo = largo.saturating_sub(EN_EL_SUELO).max(1);
                    if dentro == 0 {
                        // Al despegar se vuelve hacia ti y mira si al otro
                        // lado hay donde caer. Si no, brinca en el sitio: no se
                        // tira de una azotea, igual que la pareja no se tira al
                        // canal.
                        if objetivo.x != e.pos.x {
                            e.dir = (objetivo.x - e.pos.x).signum();
                        }
                        let paso = if golpe == 2 {
                            AVANCE_CORTO
                        } else {
                            AVANCE_LARGO
                        };
                        let pies = e.base_y + e.tipo.alto();
                        let cae = e.pos.x + e.dir * (paso + e.tipo.radio());
                        e.avance = if self.def.hay_pie(cae, pies) {
                            e.dir * paso / (vuelo as f32 * DT)
                        } else {
                            0.0
                        };
                    }
                    if dentro < vuelo {
                        // Una parabola exacta en funcion del tiempo, no una
                        // integracion: toca el suelo justo al acabar el vuelo.
                        let f = dentro as f32 / vuelo as f32;
                        e.pos.y = e.base_y - 4.0 * alto * f * (1.0 - f);
                        e.pos.x += e.avance * DT;
                    } else {
                        e.pos.y = e.base_y;
                    }
                }
                Tipo::Canotier => {
                    // Un centro que viene hacia ti y la vuelta alrededor:
                    // arriba, abajo por delante y abajo por detras, un tercio
                    // de un tiron en cada golpe.
                    let centro = Vec2::new(e.avance - VEL_CANOTIER * edad as f32 * DT, e.base_y);
                    let (s, c) = sin_cos(giro_canotier(self.tick, self.def.tiempo));
                    let (rx, ry) = VUELTA_CANOTIER;
                    e.pos = centro + Vec2::new(-rx * s, -ry * c);
                    e.dir = -1.0;
                }
                Tipo::Fila => {
                    // El centro de la fila avanza un paso en cada golpe, si
                    // por delante de la primera hay acera. Todas hacen la
                    // misma cuenta con el mismo centro, asi que van a la par
                    // aunque se tumbe a alguna.
                    let (_, dentro, _) = clave(self.tick, self.def.tiempo);
                    let frente = e.avance + sitio_en_la_fila(0, 1.0) - e.tipo.radio() - 20.0;
                    if dentro < TICKS_PASO_FILA && self.def.hay_suelo(frente) {
                        e.avance -= PASO_FILA / TICKS_PASO_FILA as f32;
                    }
                    let hueco = hueco_fila(self.tick, self.def.tiempo);
                    e.pos.x = e.avance + sitio_en_la_fila(e.lanzados, hueco);
                    e.pos.y = self.def.altura(e.pos.x) - e.tipo.alto();
                    e.dir = -1.0;
                }
                Tipo::Contrabajista => {
                    e.gesto = (e.gesto - 1.0 / 12.0).max(0.0);
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    // Solo toca con la arena cerrada: antes no hay baile.
                    let (golpe, dentro, _) = clave(self.tick, self.def.tiempo);
                    if !arena || dentro != 0 {
                        continue;
                    }
                    let compas = self.tick / (8 * u64::from((self.def.tiempo / 2).max(1)));
                    let fase = e.fase();
                    if golpe == 0 && (fase == 2 || fase == 1 && compas.is_multiple_of(2)) {
                        // El slap: una onda a cada lado, a ras de acera.
                        e.gesto = 1.0;
                        let pies = e.pos.y + e.tipo.alto();
                        for lado in [-1.0, 1.0] {
                            self.proyectiles.push(Proyectil {
                                pos: Vec2::new(e.pos.x + lado * 40.0, pies - Forma::Onda.radio()),
                                vel: Vec2::new(lado * VEL_ONDA, 0.0),
                                rosa: false,
                                forma: Forma::Onda,
                            });
                        }
                        continue;
                    }
                    // Las negras: tres, tres y dos, en abanico hacia ti y
                    // cada golpe por otros huecos. La ultima del dos, rosa.
                    let grados: &[f32] = match golpe {
                        0 => &[12.0, 30.0, 48.0],
                        1 => &[20.0, 38.0, 56.0],
                        _ => &[25.0, 45.0],
                    };
                    let mastil = e.pos + Vec2::new(e.dir * 10.0, -46.0);
                    for (i, g) in grados.iter().enumerate() {
                        let (s, c) = sin_cos(g * TAU / 360.0);
                        self.proyectiles.push(Proyectil {
                            pos: mastil,
                            vel: Vec2::new(e.dir * c, -s) * VEL_NEGRA,
                            rosa: golpe == 2 && i == grados.len() - 1,
                            forma: Forma::Negra,
                        });
                    }
                    e.lanzados += 1;
                }
                Tipo::Corista => {
                    // El cancan va en compas de dos, y ella en cuatro tiempos:
                    // dos de chasse hacia ti y dos de patada. En el uno del
                    // siguiente, el talon baja y levanta el volante. Todo va
                    // con la musica, asi que se aprende a saltar al compas.
                    let fase = edad % (4 * tiempo);
                    let cerca = (objetivo.x - e.pos.x).abs() < ALCANCE_CORISTA;
                    if fase == 0 {
                        // `reloj` es aqui "ha pateado este ciclo": el azar con
                        // que despierta se borra en su primer tick.
                        if e.reloj == 1 {
                            let x = e.pos.x + e.dir * PIERNA * 0.6;
                            if let Some(y) = self.def.suelo_en(x) {
                                self.fugaces.push(Fugaz {
                                    clase: Clase::Volante,
                                    pos: Vec2::new(x, y - 10.0),
                                    vel: Vec2::new(e.dir * VELOCIDAD_VOLANTE, 0.0),
                                    ttl: VIDA_VOLANTE,
                                    rosa: false,
                                });
                            }
                        }
                        e.reloj = 0;
                        if objetivo.x != e.pos.x {
                            e.dir = (objetivo.x - e.pos.x).signum();
                        }
                    }
                    if fase < 2 * tiempo {
                        e.gesto = 0.0;
                        if cerca {
                            // Por la acera, escaleras incluidas, y sin tirarse
                            // a un foso, como la pareja.
                            let nx = e.pos.x + e.dir * 130.0 * DT;
                            if self.def.hay_suelo(nx + e.dir * e.tipo.radio()) {
                                e.pos.x = nx;
                            } else {
                                e.dir = -e.dir;
                            }
                        }
                    } else if fase == 2 * tiempo && cerca || e.reloj == 1 {
                        e.reloj = 1;
                        let k = (fase - 2 * tiempo) as f32 / (2 * tiempo) as f32;
                        e.gesto = sin(TAU * 0.5 * k);
                    }
                    e.pos.y = self.def.altura(e.pos.x) - e.tipo.alto();
                }
                Tipo::Botella | Tipo::Pintor => {
                    e.gesto = (e.gesto - 1.0 / 12.0).max(0.0);
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    let alcance = if e.tipo == Tipo::Botella {
                        ALCANCE_BOTELLA
                    } else {
                        ALCANCE_PINTOR
                    };
                    if dx.abs() > alcance {
                        continue;
                    }
                    e.reloj = e.reloj.saturating_sub(1);
                    if e.reloj > 0 {
                        continue;
                    }
                    // Cada cuatro tiempos, como la corista.
                    e.reloj = 4 * self.def.tiempo.max(1);
                    e.lanzados += 1;
                    e.gesto = 1.0;
                    // Las dos tiran en arco a donde tienes los pies, en un
                    // tiempo fijo: se esquiva moviendose, como el plato.
                    let pies = objetivo.y + PIES - 8.0;
                    let arco = |origen: Vec2, x: f32, t: f32| {
                        let vx = ((x - origen.x) / t).clamp(-420.0, 420.0);
                        let vy = (pies - origen.y - 0.5 * GRAVEDAD_PLATO * t * t) / t;
                        Vec2::new(vx, vy)
                    };
                    if e.tipo == Tipo::Botella {
                        let origen = e.pos + Vec2::new(0.0, -22.0);
                        self.fugaces.push(Fugaz {
                            clase: Clase::Corcho,
                            pos: origen,
                            vel: arco(origen, objetivo.x, VUELO_CORCHO),
                            ttl: 1.6,
                            rosa: e.lanzados % 3 == 0,
                        });
                    } else {
                        // Un abanico de tres: a tus pies, delante y detras.
                        // Tres charcos no se saltan de pie: se sale de ahi.
                        let origen = e.pos + Vec2::new(e.dir * 20.0, -30.0);
                        for d in [-70.0, 0.0, 70.0] {
                            self.fugaces.push(Fugaz {
                                clase: Clase::Gota,
                                pos: origen,
                                vel: arco(origen, objetivo.x + d, VUELO_GOTA),
                                ttl: 1.6,
                                rosa: false,
                            });
                        }
                    }
                }
                Tipo::Compadrito => {
                    let dx = objetivo.x - e.pos.x;
                    // Subido al tranvia, el tranvia se lo lleva.
                    let alto = e.tipo.alto();
                    if let Some(d) =
                        arrastre(&self.def, &self.bases, self.tick, e.pos.x, e.pos.y + alto)
                    {
                        e.pos += d;
                    }
                    let pies = e.pos.y + alto;
                    if !self.def.hay_pie(e.pos.x, pies) {
                        // Sin nada debajo: colgado del cable del tranvia, o
                        // cayendo de el. `gesto` es eso, para el dibujo.
                        e.gesto = 1.0;
                        if e.vel == Vec2::ZERO {
                            // Se suelta en un tiempo, si el tranvia va a
                            // estar debajo al caer y te tiene cerca.
                            let caida = (2.0 * 90.0 / GRAVITY).sqrt() / DT;
                            let llega = self.tick + 1 + caida as u64;
                            let debajo =
                                self.def
                                    .plataformas
                                    .iter()
                                    .zip(&self.bases)
                                    .any(|(pl, base)| {
                                        let Some(m) = pl.mueve else { return false };
                                        let x = base.x + m.desvio(llega).x;
                                        pl.y > pies
                                            && e.pos.x > x + 40.0
                                            && e.pos.x < x + pl.ancho - 40.0
                                    });
                            let cerca = (SALTO_CERCA..SALTO_LEJOS).contains(&dx.abs());
                            if debajo && cerca && self.tick.is_multiple_of(tiempo) {
                                e.vel.y = 1.0;
                            }
                            continue;
                        }
                        e.vel.y += GRAVITY * DT;
                        let nuevos = pies + e.vel.y * DT;
                        let x = e.pos.x;
                        let cruza = |y: f32| pies <= y + 0.5 && nuevos >= y;
                        let cae = self
                            .def
                            .plataformas
                            .iter()
                            .filter(|pl| x >= pl.x && x <= pl.x + pl.ancho)
                            .map(|pl| pl.y)
                            .chain(self.def.suelo_en(x))
                            .find(|&y| cruza(y));
                        match cae {
                            Some(y) => {
                                // Cae y se clava, como al acabar la estocada.
                                e.pos.y = y - alto;
                                e.vel = Vec2::ZERO;
                                e.gesto = 0.0;
                                e.reloj = PLANTADO_COMPADRITO;
                            }
                            None => e.pos.y = nuevos - alto,
                        }
                        continue;
                    }
                    if e.reloj == 0 {
                        // Quieto hasta tenerte a tiro, y entonces arranca en
                        // el siguiente tiempo de la musica: se lanza a compas.
                        if dx.abs() > ALCANCE_COMPADRITO || !self.tick.is_multiple_of(tiempo) {
                            continue;
                        }
                        if dx != 0.0 {
                            e.dir = dx.signum();
                        }
                        // Este tick ya es el primero del amago.
                        e.reloj = CICLO_COMPADRITO;
                        continue;
                    }
                    e.reloj -= 1;
                    // La estocada: de `ESTOCADA_V` a cero en linea recta. Es
                    // aceleracion negativa constante, el corte del jefe.
                    let quedan = e.reloj.saturating_sub(PLANTADO_COMPADRITO);
                    if quedan == 0 || quedan > ESTOCADA_COMPADRITO {
                        continue;
                    }
                    let v = ESTOCADA_V * quedan as f32 / ESTOCADA_COMPADRITO as f32;
                    let nx = e.pos.x + e.dir * v * DT;
                    if self.def.hay_pie(nx + e.dir * e.tipo.radio(), pies) {
                        e.pos.x = nx;
                    } else {
                        // Al borde del Riachuelo, o del techo del tranvia, se
                        // clava antes de tiempo.
                        e.reloj = PLANTADO_COMPADRITO;
                    }
                }
                Tipo::Florista => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if dx.abs() > ALCANCE_FLORISTA {
                        continue;
                    }
                    e.reloj = e.reloj.saturating_sub(1);
                    if e.reloj > 0 {
                        continue;
                    }
                    e.reloj = CADENCIA_FLORISTA;
                    e.lanzados += 1;
                    // Derecha a ti: no hay arco que leer, hay una rosa que
                    // viene rapida y se para antes de llegar si estas lejos.
                    let origen = e.pos + Vec2::new(e.dir * 16.0, -24.0);
                    let hacia = (objetivo - origen).normalize_or(Vec2::new(e.dir, 0.0));
                    nuevas.push(Enemigo {
                        tipo: Tipo::Rosa,
                        pos: origen,
                        vida: Tipo::Rosa.vida(),
                        dir: e.dir,
                        nacio: self.tick,
                        rosa: e.lanzados % 3 == 0,
                        golpe: 0,
                        reloj: 0,
                        lanzados: 0,
                        base_y: origen.y,
                        avance: 0.0,
                        gesto: 0.0,
                        vel: hacia * ROSA_V,
                        portero: false,
                    });
                }
                Tipo::Rosa => {
                    // Vuela frenando, se queda colgada y cae marchita. Por
                    // edad y no por velocidad, para que colgar dure lo mismo
                    // la tire quien la tire.
                    if edad < VUELO_ROSA {
                        e.pos += e.vel * DT;
                        e.vel *= FRENO_ROSA;
                    } else if edad < VUELO_ROSA + COLGADA_ROSA {
                        e.vel = Vec2::ZERO;
                    } else {
                        e.vel.y += GRAVEDAD_ROSA * DT;
                        e.pos += e.vel * DT;
                    }
                }
                Tipo::Confeti => {
                    // Hacia la izquierda a brincos, uno por tiempo: el galop.
                    // Con la mecha prendida va a medio gas, y `gesto` es lo
                    // que lleva quemado.
                    let mecha = e.gesto > 0.0;
                    let v = if mecha {
                        VEL_CONFETI * 0.5
                    } else {
                        VEL_CONFETI
                    };
                    e.pos.x -= v * DT;
                    let f = (edad % tiempo) as f32 / tiempo as f32;
                    e.pos.y = e.base_y - BRINCO_CONFETI * sin(TAU * 0.5 * f);
                    e.dir = -1.0;
                    let d = objetivo - e.pos;
                    if mecha || d.x.abs() < MECHA_CONFETI_X && d.y.abs() < 260.0 {
                        e.gesto += 1.0 / MECHA_CONFETI as f32;
                    }
                    if e.gesto >= 1.0 - 1e-4 {
                        // Revienta: papelillos en corona, y si es rosa, uno
                        // rosa. Se va sin contar como caido (abajo).
                        for k in 0..PAPELILLOS {
                            let (s, c) = sin_cos(TAU * k as f32 / PAPELILLOS as f32);
                            self.fugaces.push(Fugaz {
                                clase: Clase::Papelillo,
                                pos: e.pos,
                                vel: Vec2::new(c, s) * VEL_PAPELILLO,
                                ttl: VIDA_PAPELILLO,
                                rosa: e.rosa && k == 0,
                            });
                        }
                        e.gesto = 2.0;
                    }
                }
                Tipo::Apache => {
                    let dx = objetivo.x - e.pos.x;
                    if e.reloj == 0 {
                        // En guardia y de cara. En un tiempo, si te tiene a
                        // tiro, se esfuma y sale por tu otro lado; si alli no
                        // hay acera, por el suyo.
                        if dx != 0.0 {
                            e.dir = dx.signum();
                        }
                        if dx.abs() > ALCANCE_APACHE || !self.tick.is_multiple_of(tiempo) {
                            continue;
                        }
                        let def = &self.def;
                        let sale = [e.dir, -e.dir]
                            .into_iter()
                            .map(|lado| objetivo.x + lado * LADO_APACHE)
                            .find(|&x| {
                                x > muro + 20.0
                                    && def.hay_suelo(x - 20.0)
                                    && def.hay_suelo(x + 20.0)
                            });
                        let Some(x) = sale else { continue };
                        self.fugaces.push(Fugaz {
                            clase: Clase::Humo,
                            pos: e.pos,
                            vel: Vec2::ZERO,
                            ttl: VIDA_HUMO,
                            rosa: false,
                        });
                        e.pos = Vec2::new(x, def.altura(x) - e.tipo.alto());
                        e.reloj = CICLO_APACHE;
                        continue;
                    }
                    e.reloj -= 1;
                    let r = e.reloj;
                    e.gesto = 0.0;
                    if r > GOLPE_APACHE + PLANTADO_APACHE || r <= PLANTADO_APACHE {
                        // Esfumado (ver `presente`), o plantado tras el golpe.
                        continue;
                    }
                    // Sale de cara a ti, amaga y tira el navajazo dando un
                    // paso: la navaja sube y baja en lo que queda del tiempo.
                    let g = GOLPE_APACHE + PLANTADO_APACHE - r;
                    if g == 0 && dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    if g >= AMAGO_APACHE {
                        let k = (g - AMAGO_APACHE) as f32 / (GOLPE_APACHE - AMAGO_APACHE) as f32;
                        e.gesto = sin(TAU * 0.5 * k);
                        let nx = e.pos.x + e.dir * ESTOCADA_APACHE * DT;
                        if self.def.hay_suelo(nx + e.dir * e.tipo.radio()) {
                            e.pos = Vec2::new(nx, self.def.altura(nx) - e.tipo.alto());
                        }
                    }
                }
                Tipo::Loyal => {
                    let dx = objetivo.x - e.pos.x;
                    if dx != 0.0 {
                        e.dir = dx.signum();
                    }
                    // El numero no empieza hasta que se cierra la pista.
                    if self.def.porteria.is_some() && pista_loyal.is_none() {
                        continue;
                    }
                    let fase = e.fase();
                    if e.reloj == 0 {
                        // Lo que presenta ahora. En la primera fase, sin baston.
                        while fase == 0 && e.numero() == Some(Numero::Baston) {
                            e.lanzados += 1;
                        }
                    }
                    let numero = NUMEROS[e.lanzados as usize % NUMEROS.len()];
                    // Un tiempo de aviso (la chistera, el aro o el baston en
                    // alto) y luego lo suelta, una o varias veces.
                    let (r, aviso) = (e.reloj, tiempo as u32);
                    let (veces, cada) = match numero {
                        Numero::Palomas => (1 + u32::from(fase == 2), aviso),
                        Numero::Aros if fase == 2 => (3, aviso / 2),
                        Numero::Aros => (2 + u32::from(fase > 0), aviso),
                        Numero::Baston => (1, aviso),
                    };
                    let ultimo = aviso + cada * (veces - 1);
                    e.gesto = if r < aviso {
                        r as f32 / aviso as f32
                    } else {
                        (e.gesto - 1.0 / 12.0).max(0.0)
                    };
                    let pies = e.pos.y + e.tipo.alto();
                    if r >= aviso && r <= ultimo && (r - aviso).is_multiple_of(cada) {
                        e.gesto = 1.0;
                        match numero {
                            Numero::Palomas => {
                                // En abanico hacia ti; la del medio, rosa.
                                let boca = e.pos + Vec2::new(e.dir * 6.0, -62.0);
                                let hacia = (objetivo - boca).normalize_or(Vec2::new(e.dir, 0.0));
                                let n = 3 + fase;
                                for k in 0..n {
                                    let (s, c) = sin_cos((k as f32 - (n - 1) as f32 * 0.5) * 0.24);
                                    let v = Vec2::new(
                                        hacia.x * c - hacia.y * s,
                                        hacia.x * s + hacia.y * c,
                                    );
                                    self.fugaces.push(Fugaz {
                                        clase: Clase::Paloma,
                                        pos: boca,
                                        vel: v * VEL_PALOMA,
                                        ttl: VIDA_PALOMA,
                                        rosa: k == n / 2,
                                    });
                                }
                            }
                            Numero::Aros => self.fugaces.push(Fugaz {
                                clase: Clase::Aro,
                                pos: Vec2::new(e.pos.x + e.dir * 26.0, pies - 18.0),
                                vel: Vec2::new(e.dir * VEL_ARO, 0.0),
                                ttl: VIDA_ARO,
                                rosa: false,
                            }),
                            Numero::Baston => {
                                // Al final, barre a los dos lados.
                                let lados = if fase == 2 { 2 } else { 1 };
                                for lado in [e.dir, -e.dir].into_iter().take(lados) {
                                    self.fugaces.push(Fugaz {
                                        clase: Clase::Barrido,
                                        pos: Vec2::new(e.pos.x + lado * 34.0, pies - 12.0),
                                        vel: Vec2::new(lado * VEL_BARRIDO, 0.0),
                                        ttl: VIDA_BARRIDO,
                                        rosa: false,
                                    });
                                }
                            }
                        }
                    }
                    // Por la pista: antes del baston, un brinco hacia ti; tras
                    // el numero, de cerca se aparta y de lejos se acerca.
                    let v = if numero == Numero::Baston && r < aviso && dx.abs() > 160.0 {
                        220.0
                    } else if r > ultimo && dx.abs() < CERCA_LOYAL {
                        -120.0
                    } else if r > ultimo && dx.abs() > LEJOS_LOYAL {
                        80.0
                    } else {
                        0.0
                    } * e.dir;
                    let (izq, der) = pista_loyal
                        .map_or((f32::MIN, f32::MAX), |a| (a + 60.0, a + VISTA_W - 60.0));
                    let nx = (e.pos.x + v * DT).clamp(izq, der);
                    if v != 0.0 && self.def.hay_suelo(nx + v.signum() * e.tipo.radio()) {
                        e.pos.x = nx;
                    }
                    e.pos.y = self.def.altura(e.pos.x) - e.tipo.alto();
                    e.reloj += 1;
                    if e.reloj >= NUMERO_LOYAL[fase] {
                        e.reloj = 0;
                        e.lanzados += 1;
                    }
                }
                Tipo::Gaviota => {
                    let dx = objetivo.x - e.pos.x;
                    if e.reloj == 0 {
                        // Se cierne donde esta hasta tenerte a tiro, y arranca
                        // en un tiempo de la musica.
                        if dx.abs() > ALCANCE_GAVIOTA || !self.tick.is_multiple_of(tiempo) {
                            continue;
                        }
                        e.reloj = CICLO_GAVIOTA;
                    }
                    e.reloj -= 1;
                    let picado = PICADO_GAVIOTA + COLGADA_GAVIOTA;
                    if e.reloj >= picado {
                        // El remonte: vuelve a su altura frenando, y en el
                        // ultimo tick apunta a donde estas.
                        e.pos.y += (e.base_y - e.pos.y) * 0.15;
                        if dx != 0.0 {
                            e.dir = dx.signum();
                        }
                        if e.reloj == picado {
                            let suelo = self.def.altura(objetivo.x) - 40.0;
                            let d = Vec2::new(objetivo.x, objetivo.y.min(suelo)) - e.pos;
                            let largo = d.length().min(PICADO_MAX);
                            // Frenando en linea recta hasta cero: con `n`
                            // ticks recorre v0 (n + 1) / 2 por tick.
                            let n = PICADO_GAVIOTA as f32;
                            e.vel = d.normalize_or(Vec2::new(e.dir, 0.0)) * 2.0 * largo
                                / ((n + 1.0) * DT);
                        }
                    } else if e.reloj >= COLGADA_GAVIOTA {
                        let quedan = (e.reloj - COLGADA_GAVIOTA + 1) as f32;
                        e.pos += e.vel * quedan / PICADO_GAVIOTA as f32 * DT;
                    }
                }
                Tipo::Malevo => {
                    // Lo que se sacude el poncho al parar un disparo.
                    e.gesto = (e.gesto - 1.0 / 8.0).max(0.0);
                    let dx = objetivo.x - e.pos.x;
                    let compas = 4 * tiempo;
                    if dx.abs() > ALCANCE_MALEVO {
                        continue;
                    }
                    if self.tick.is_multiple_of(compas) {
                        // Se vuelve de golpe al empezar el compas, y echa mano
                        // a la rosa: la tira dos tiempos despues.
                        if dx != 0.0 {
                            e.dir = dx.signum();
                        }
                        e.reloj = 2 * self.def.tiempo.max(1);
                    }
                    if e.reloj == 0 {
                        continue;
                    }
                    e.reloj -= 1;
                    if e.reloj > 0 {
                        continue;
                    }
                    e.lanzados += 1;
                    let origen = e.pos + Vec2::new(e.dir * 16.0, -26.0);
                    let hacia = (objetivo - origen).normalize_or(Vec2::new(e.dir, 0.0));
                    // La primera, rosa: el malevo se ensena con lo que lo tumba.
                    let rosa = e.lanzados % 2 == 1;
                    nuevas.push(rosa_lanzada(origen, hacia * ROSA_V, rosa, self.tick));
                }
                Tipo::Guapo => {
                    // Quieto, mirandote, hasta que se cierra la arena.
                    let dx = objetivo.x - e.pos.x;
                    let Some(desde) = arena_guapo else {
                        if dx != 0.0 {
                            e.dir = dx.signum();
                        }
                        continue;
                    };
                    if e.reloj == 0 {
                        // Cada jugada empieza en un tiempo, con la fase de
                        // ahora: un cambio de fase no corta una jugada a medias.
                        if !self.tick.is_multiple_of(tiempo) {
                            continue;
                        }
                        e.gesto = e.fase() as f32;
                    }
                    let t = self.def.tiempo.max(1);
                    let jugada = JUGADAS[e.gesto as usize][e.lanzados as usize % 2];
                    let (paso, dentro) = (jugada[(e.reloj / t) as usize % 4], e.reloj % t);
                    if dentro == 0 {
                        // Se vuelve hacia ti en seco en cada tiempo.
                        if dx != 0.0 {
                            e.dir = dx.signum();
                        }
                        e.vel = match paso {
                            Paso::Estocada => Vec2::new(e.dir * ESTOCADA_GUAPO, 0.0),
                            Paso::Finta => Vec2::new(e.dir * FINTA_GUAPO, 0.0),
                            _ => Vec2::ZERO,
                        };
                        if paso == Paso::Tira {
                            // De la mano hacia ti y abriendose hacia arriba:
                            // la de abajo va a la altura del pecho y es la de
                            // parry; las otras se quedan colgadas encima, que
                            // es por donde se salta la estocada.
                            let n = if e.gesto == 0.0 { 3 } else { 5 };
                            let origen = e.pos + Vec2::new(e.dir * 20.0, -30.0);
                            for k in 0..n {
                                let (s, c) = sin_cos(BAJA_ROSA - ABRE_ROSAS * k as f32);
                                let v = Vec2::new(e.dir * c, s) * ROSA_V;
                                nuevas.push(rosa_lanzada(origen, v, k == 0, self.tick));
                            }
                        }
                    }
                    if matches!(paso, Paso::Estocada | Paso::Finta) {
                        // La aceleracion negativa del jefe: de tope a cero en
                        // linea recta, y contra la pared de la arena se clava.
                        let quedan = (t - dentro) as f32 / t as f32;
                        let (a, b) = (desde + MARGEN_ARENA, desde + VISTA_W - MARGEN_ARENA);
                        let nx = e.pos.x + e.vel.x * quedan * DT;
                        e.pos.x = nx.clamp(a, b);
                        if nx != e.pos.x {
                            e.vel = Vec2::ZERO;
                        }
                    }
                    e.reloj += 1;
                    if e.reloj >= 4 * t {
                        e.reloj = 0;
                        e.lanzados += 1;
                    }
                }
            }
        }
        self.enemigos.extend(nuevas);
        // Una rosa que se ha caido por debajo de la calle ya no es nada, ni un
        // compadrito que ha caido al agua desde el cable. Se van sin contar
        // como caida, que nadie los ha tumbado.
        let def = &self.def;
        self.enemigos.retain(|e| {
            !matches!(e.tipo, Tipo::Rosa | Tipo::Compadrito) || e.pos.y < def.altura(e.pos.x) + 90.0
        });
        // Y una hoja que llega a la acera se queda alli, que ya no es nada.
        self.enemigos
            .retain(|e| e.tipo != Tipo::Hoja || e.pos.y < def.altura(e.pos.x) - 8.0);
        // Un confeti reventado es papelillos: se va sin contar como caido.
        self.enemigos
            .retain(|e| e.tipo != Tipo::Confeti || e.gesto < 2.0);
    }

    /// Lo fugaz se mueve, se transforma al tocar la acera y se apaga.
    fn mover_fugaces(&mut self) {
        // Solo los que habia al empezar el tick: lo que nace de un corcho
        // empieza a moverse en el siguiente, como todo lo que nace.
        for i in 0..self.fugaces.len() {
            let mut f = self.fugaces[i];
            f.ttl -= DT;
            match f.clase {
                Clase::Volante => {
                    // Corre pegado a la acera, escaleras arriba si hace falta.
                    // Donde se acaba la acera se acaba el: un foso lo apaga.
                    f.pos.x += f.vel.x * DT;
                    match self.def.suelo_en(f.pos.x) {
                        Some(y) => f.pos.y = y - 10.0,
                        None => f.ttl = 0.0,
                    }
                }
                Clase::Corcho | Clase::Gota => {
                    f.vel.y += GRAVEDAD_PLATO * DT;
                    f.pos += f.vel * DT;
                    let suelo = self.def.suelo_en(f.pos.x).filter(|&y| f.pos.y >= y);
                    if let Some(y) = suelo {
                        f.pos.y = y;
                        if f.clase == Clase::Gota {
                            f = Fugaz {
                                clase: Clase::Mancha,
                                pos: Vec2::new(f.pos.x, y - 4.0),
                                vel: Vec2::ZERO,
                                ttl: VIDA_MANCHA,
                                rosa: false,
                            };
                        } else {
                            f.ttl = 0.0;
                        }
                    }
                    if f.clase == Clase::Corcho && f.ttl <= 0.0 {
                        // Revienta: al caer, o en el aire si se le acaba.
                        let centro = f.pos - Vec2::new(0.0, 8.0);
                        for k in 0..BURBUJAS {
                            let (s, c) = sin_cos(TAU * k as f32 / BURBUJAS as f32);
                            self.fugaces.push(Fugaz {
                                clase: Clase::Burbuja,
                                pos: centro,
                                vel: Vec2::new(c, s) * VELOCIDAD_BURBUJA,
                                ttl: VIDA_BURBUJA,
                                rosa: false,
                            });
                        }
                    }
                }
                Clase::Burbuja => f.pos += f.vel * DT,
                Clase::Mancha => {}
                Clase::Paloma => f.pos += f.vel * DT,
                Clase::Papelillo => {
                    f.vel *= FRENO_PAPELILLO;
                    f.vel.y += GRAVEDAD_PAPELILLO * DT;
                    f.pos += f.vel * DT;
                }
                Clase::Aro | Clase::Barrido => {
                    // A ras de acera como el volante; el aro, botando a
                    // cada tiempo del galop (0,4 s).
                    f.pos.x += f.vel.x * DT;
                    let (alto, bote) = if f.clase == Clase::Aro {
                        (18.0, 26.0 * sin(TAU * 1.25 * f.ttl).abs())
                    } else {
                        (12.0, 0.0)
                    };
                    match self.def.suelo_en(f.pos.x) {
                        Some(y) => f.pos.y = y - alto - bote,
                        None => f.ttl = 0.0,
                    }
                }
                Clase::Humo => f.pos.y -= 30.0 * DT,
            }
            self.fugaces[i] = f;
        }
        self.fugaces.retain(|f| f.ttl > 0.0);
    }

    fn mover_proyectiles(&mut self) {
        for q in &mut self.proyectiles {
            if q.forma == Forma::Plato {
                q.vel.y += GRAVEDAD_PLATO * DT;
            } else if q.forma == Forma::Remolino && q.vel.length() < TOPE_REMOLINO {
                // Gira y acelera a la vez: la vuelta se abre en espiral, y al
                // llegar al tope sigue recta.
                let (s, c) = sin_cos(GIRO_REMOLINO * DT);
                let v = q.vel;
                q.vel = Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c) * ACELERA_REMOLINO;
            }
            if q.forma == Forma::Negra {
                // Se tuerce hacia el suelo, vaya al lado que vaya.
                let (s, c) = sin_cos(GIRO_NEGRA * DT * q.vel.x.signum());
                q.vel = Vec2::new(q.vel.x * c - q.vel.y * s, q.vel.x * s + q.vel.y * c);
            }
            q.pos += q.vel * DT;
        }
        // Un plato que llega a la acera se rompe. Sobre un foso sigue cayendo
        // hasta perderse, que no hace dano a nadie. Lo que vuela recto se
        // rompe igual contra la acera, y sobre el vacio se va al salir de la
        // vista; por los lados lo quita `olvidar`.
        let def = &self.def;
        self.proyectiles.retain(|q| match q.forma {
            Forma::Plato => q.pos.y < SUELO_Y,
            _ => {
                let en_acera = q.pos.y >= SUELO_Y && def.hay_suelo(q.pos.x);
                q.pos.y > -40.0 && q.pos.y < VISTA_H + 40.0 && !en_acera
            }
        });
    }

    fn mover_disparos(&mut self) {
        let serpentina = self.equipo.tiro == Tiro::Serpentina;
        for d in &mut self.disparos {
            // La serpentina se tuerce hacia el enemigo vivo mas cercano.
            if serpentina
                && let Some(e) = self
                    .enemigos
                    .iter()
                    .filter(|e| e.vida > 0 && e.presente())
                    .min_by(|a, b| {
                        let (da, db) =
                            (a.pos.distance_squared(d.pos), b.pos.distance_squared(d.pos));
                        da.total_cmp(&db)
                    })
            {
                d.vel = guiar(d.vel, e.pos - d.pos);
            }
            d.pos += d.vel * DT;
            d.ttl -= DT;
        }
        self.disparos.retain(|d| d.ttl > 0.0);

        let arena = self.cierre().is_some();
        let mut i = 0;
        while i < self.disparos.len() {
            let d = self.disparos[i];
            let blanco = self.enemigos.iter_mut().find(|e| {
                let r = e.tipo.radio() + DISPARO_RADIO;
                e.tocable(arena) && e.pos.distance_squared(d.pos) <= r * r
            });
            if let Some(e) = blanco {
                if e.para(&d) {
                    // El poncho se come el disparo: ni dano ni medidor. Solo
                    // se sacude, que es lo que dice que ahi no se tira.
                    e.gesto = 1.0;
                    self.disparos.swap_remove(i);
                    continue;
                }
                e.vida -= if d.rosa {
                    DANO_DEVUELTA
                } else {
                    self.equipo.tiro.dano_calle()
                };
                e.golpe = 6;
                self.events.boss_hit = true;
                self.impacto = d.pos;
                self.jugadora.add_meter(METER_POR_IMPACTO);
                self.disparos.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.retirar_caidos();
    }

    /// Quita a los enemigos sin vida, avisando de cada caida.
    fn retirar_caidos(&mut self) {
        let mut caido = None;
        let mut portero = None;
        let mut n = 0;
        self.enemigos.retain(|e| {
            if e.vida <= 0 {
                caido = Some(e.pos);
                if e.portero {
                    portero = Some(e.pos);
                }
                n += 1;
            }
            e.vida > 0
        });
        self.caidos += n;
        // El portero cae como un jefe: el estallido sale de el aunque en el
        // mismo tick caiga otro.
        if let Some(pos) = portero.or(caido) {
            self.impacto = pos;
        }
        self.events.boss_down |= portero.is_some();
    }

    /// Si la capsula de la jugadora toca un circulo.
    fn toca(&self, c: Vec2, r: f32) -> bool {
        let p = self.jugadora.pos;
        let (radio, medio) = self.equipo.amuleto.cuerpo(CUERPO_RADIO, CUERPO_MEDIO);
        let y = c.y.clamp(p.y - medio, p.y + medio);
        let rr = r + radio;
        Vec2::new(p.x, y).distance_squared(c) <= rr * rr
    }

    /// Los golpes que se llevan: platos y contacto con enemigos. Uno por tick
    /// como mucho, y ninguno durante los i-frames.
    fn recibir(&mut self) {
        if self.jugadora.is_invulnerable() {
            return;
        }
        let plato = self
            .proyectiles
            .iter()
            .position(|q| self.toca(q.pos, q.forma.radio()));
        let fugaz = self
            .fugaces
            .iter()
            .position(|f| f.clase != Clase::Humo && self.toca(f.pos, f.clase.radio()));
        // La pierna de la corista solo pega arriba: recogida es decorado.
        let patada = self.enemigos.iter().any(|e| {
            e.gesto > PIERNA_PELIGRO
                && e.pierna()
                    .is_some_and(|(_, pie)| self.toca(pie, RADIO_BOTIN))
        });
        let navaja = self
            .enemigos
            .iter()
            .any(|e| e.navaja().is_some_and(|n| self.toca(n, RADIO_NAVAJA)));
        let tocado = plato.is_some()
            || fugaz.is_some()
            || patada
            || navaja
            || self
                .enemigos
                .iter()
                .any(|e| e.presente() && self.toca(e.pos, e.tipo.radio() * 0.8));
        if let Some(i) = plato {
            self.proyectiles.swap_remove(i);
        }
        if let Some(i) = fugaz {
            self.fugaces.swap_remove(i);
        }
        if tocado {
            // Un empujon hacia atras y arriba: que se vea que te han dado y
            // que te saque de encima de la pareja que te ha pisado.
            let p = &mut self.jugadora;
            p.vel = Vec2::new(-p.facing.x * 160.0, -420.0);
            p.on_ground = false;
            self.golpe();
        }
    }

    /// Lo que se ha quedado muy atras se va.
    fn olvidar(&mut self) {
        let x = self.jugadora.pos.x;
        self.enemigos.retain(|e| e.pos.x > x - OLVIDAR);
        self.proyectiles.retain(|q| (q.pos.x - x).abs() < OLVIDAR);
        self.fugaces.retain(|f| (f.pos.x - x).abs() < OLVIDAR);
    }

    /// Huella del estado, para el test de determinismo. Como la del mundo,
    /// deja fuera lo derivado: la estela, los eventos y el punto de impacto.
    pub fn huella(&self) -> u64 {
        let mut h = Fnv1a::new();
        h.write_u64(self.tick);
        let (s, i) = self.rng.raw_state();
        h.write_u64(s);
        h.write_u64(i);
        h.write_u64(u64::from(self.prev_input.bits()));
        let p = &self.jugadora;
        h.write_vec2(p.pos);
        h.write_vec2(p.vel);
        h.write_vec2(p.facing);
        h.write_u64(u64::from(p.iframes));
        h.write_u64(u64::from(p.dash.ticks_left));
        h.write_u64(u64::from(p.dash.cooldown));
        h.write_u64(u64::from(p.on_ground));
        h.write_u64(u64::from(p.coyote));
        h.write_u64(u64::from(p.jump_buffer));
        h.write_u64(u64::from(p.parry_window));
        h.write_f32(p.meter);
        h.write_u64(u64::from(self.vidas));
        h.write_vec2(self.apunta);
        h.write_vec2(self.seguro);
        h.write_u64(u64::from(self.soltar));
        h.write_u64(self.siguiente as u64);
        for e in &self.enemigos {
            h.write_u64(u64::from(e.tipo.como_u8()));
            h.write_vec2(e.pos);
            h.write_u64(e.vida as u64);
            h.write_f32(e.dir);
            h.write_u64(u64::from(e.reloj));
            h.write_f32(e.avance);
            h.write_f32(e.gesto);
            h.write_vec2(e.vel);
        }
        for q in &self.proyectiles {
            h.write_vec2(q.pos);
            h.write_vec2(q.vel);
            h.write_u64(u64::from(q.rosa));
            h.write_u64(q.forma as u64);
        }
        for f in &self.fugaces {
            h.write_u64(f.clase as u64);
            h.write_vec2(f.pos);
            h.write_vec2(f.vel);
            h.write_f32(f.ttl);
            h.write_u64(u64::from(f.rosa));
        }
        for d in &self.disparos {
            h.write_vec2(d.pos);
            h.write_f32(d.ttl);
        }
        h.write_u64(self.completado.unwrap_or(u64::MAX));
        h.write_u64(u64::from(self.derrota));
        let [t, a] = self.equipo.a_bytes();
        h.write_u64(u64::from(t) << 8 | u64::from(a));
        for f in &self.fichas {
            h.write_u64(u64::from(*f));
        }
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DERECHA: u16 = InputFrame::RIGHT;
    const DISPARA: u16 = InputFrame::SHOOT;

    fn pulsar(bits: u16) -> InputFrame {
        InputFrame::from_bits(bits)
    }

    /// Una calle de pruebas: acera de 0 a 3000, sin fosos ni nadie.
    fn calle() -> PaseoDef {
        PaseoDef {
            nombre: "Prueba".into(),
            jefe: "El Vals".into(),
            subtitulo: String::new(),
            tiempo: 21,
            largo: 3200.0,
            salida: 100.0,
            puerta: 3000.0,
            suelo: vec![(0.0, 3200.0)],
            plataformas: vec![],
            enemigos: vec![],
            escaleras: vec![],
            fichas: vec![],
            porteria: None,
        }
    }

    fn con(enemigos: Vec<Aparicion>) -> PaseoDef {
        PaseoDef {
            enemigos,
            ..calle()
        }
    }

    fn correr(p: &mut Paseo, bits: u16, n: usize) {
        for _ in 0..n {
            p.step(pulsar(bits));
        }
    }

    /// El paseo de serie que va antes de `jefe`.
    fn de_serie(jefe: &str) -> PaseoDef {
        PaseoDef::de_serie()
            .into_iter()
            .find(|d| d.jefe == jefe)
            .unwrap_or_else(|| panic!("no hay paseo antes de {jefe}"))
    }

    #[test]
    fn los_paseos_de_serie_cargan_y_tienen_sentido() {
        let defs = PaseoDef::de_serie();
        assert_eq!(defs.len(), PASEO_RONS.len());
        for d in &defs {
            assert!(
                d.hay_suelo(d.salida),
                "{}: se empieza pisando acera",
                d.nombre
            );
            assert!(d.hay_suelo(d.puerta), "{}: la puerta en la acera", d.nombre);
            assert!(d.puerta < d.largo);
            // Los tramos en orden y sin pisarse: si se solapasen, un foso
            // podria quedar tapado sin que el dibujo lo supiera.
            for par in d.suelo.windows(2) {
                assert!(par[0].1 < par[1].0, "{}: tramos desordenados", d.nombre);
            }
            assert!(d.suelo.len() >= 2, "{}: sin fosos no hay paseo", d.nombre);
            assert!(
                d.enemigos.iter().any(|a| a.rosa),
                "{}: sin nada rosa no hay parry en la calle",
                d.nombre
            );
            // Dos paseos para el mismo jefe: el segundo no se andaria nunca.
            let mismos = defs.iter().filter(|o| o.jefe == d.jefe).count();
            assert_eq!(mismos, 1, "dos paseos antes de {}", d.jefe);
        }

        let viena = de_serie("El Vals");
        assert!(viena.suelo.len() >= 3, "hace falta algun foso que saltar");
        for t in [
            Tipo::Pareja,
            Tipo::Camarero,
            Tipo::Hoja,
            Tipo::Husar,
            Tipo::Director,
        ] {
            assert!(
                viena.enemigos.iter().any(|a| a.tipo == t),
                "falta {t:?} en Viena"
            );
        }
        // Las notas volaban por las cuatro calles; en Viena caen hojas.
        // El husar sale a `DETRAS_HUSAR` de donde va ella al despertarlo, y
        // ahi tiene que haber acera.
        for a in viena.enemigos.iter().filter(|a| a.tipo == Tipo::Husar) {
            let x = a.x - ACTIVAR - DETRAS_HUSAR;
            assert!(viena.hay_suelo(x) && viena.hay_suelo(x - 60.0), "{a:?}");
        }
        // El director en su arena, y la puerta pasada ella.
        let po = viena.porteria.expect("Viena acaba con el director");
        let dire = viena.enemigos.iter().find(|a| a.portero).expect("portero");
        assert!(dire.x > po.desde && dire.x < po.desde + VISTA_W);
        assert!(viena.puerta > po.desde + VISTA_W);
        assert_eq!(po.ficha, Some(viena.fichas.len() - 1), "la suya, la ultima");

        // Chicago se cruza por arriba: tiene que haber un tramo largo sin
        // calle, y la clave del charleston en su `tiempo`.
        let chicago = de_serie("El Charleston");
        let vacio = chicago
            .suelo
            .windows(2)
            .map(|par| par[1].0 - par[0].1)
            .fold(0.0, f32::max);
        assert!(vacio > 2000.0, "Chicago no tiene tejados: {vacio}");
        assert_eq!(chicago.tiempo, 36, "el Maple Leaf va a 100");
        for t in [
            Tipo::Flapper,
            Tipo::Saxo,
            Tipo::Saltarin,
            Tipo::Canotier,
            Tipo::Fila,
            Tipo::Contrabajista,
        ] {
            assert!(
                chicago.enemigos.iter().any(|a| a.tipo == t),
                "falta {t:?} en Chicago"
            );
        }
        // Cada flapper, saxo y saltarin, de pie sobre algo: uno puesto en el
        // aire flotaria, y un saltarin en el aire no sabria donde caer.
        for a in &chicago.enemigos {
            if matches!(
                a.tipo,
                Tipo::Flapper | Tipo::Saxo | Tipo::Saltarin | Tipo::Fila | Tipo::Contrabajista
            ) {
                let pisa = if a.y > 0.0 { a.y } else { SUELO_Y };
                assert!(chicago.hay_pie(a.x, pisa), "{a:?} flota");
            }
        }
        // Las notas de Viena no vuelan por Chicago: alli vuelan canotieres.
        // El portero, dentro de su arena, y la puerta pasada la arena.
        let po = chicago
            .porteria
            .expect("Chicago acaba con el contrabajista");
        let portero = chicago
            .enemigos
            .iter()
            .find(|a| a.portero)
            .expect("sin portero");
        assert!((po.desde..po.desde + VISTA_W).contains(&portero.x));
        assert!(chicago.puerta > po.desde + VISTA_W);

        // El arrabal, antes del tango, con sus enemigos y mas saltos que Viena.
        let arrabal = de_serie("El Tango");
        assert!(
            arrabal.suelo.len() > viena.suelo.len(),
            "el Riachuelo pide mas saltos"
        );
        assert_eq!(arrabal.tiempo, 30, "el tango va a 120");
        for t in [Tipo::Compadrito, Tipo::Florista] {
            assert!(
                arrabal.enemigos.iter().any(|a| a.tipo == t),
                "falta {t:?} en el arrabal"
            );
        }
        // Las rosas no se ponen: las tira la florista. Y una florista tiene
        // que estar de pie sobre algo.
        assert!(!arrabal.enemigos.iter().any(|a| a.tipo == Tipo::Rosa));
        for a in arrabal.enemigos.iter().filter(|a| a.tipo == Tipo::Florista) {
            assert!(arrabal.hay_pie(a.x, a.y), "florista en el aire: {a:?}");
        }
        // Sus voladores son gaviotas, trae malevos y acaba con el guapo en su
        // arena, antes de la puerta.
        for t in [Tipo::Gaviota, Tipo::Malevo] {
            assert!(arrabal.enemigos.iter().any(|a| a.tipo == t), "falta {t:?}");
        }
        let po = arrabal.porteria.expect("el arrabal acaba con portero");
        let guapo: Vec<&Aparicion> = arrabal.enemigos.iter().filter(|a| a.portero).collect();
        assert!(guapo.len() == 1 && guapo[0].tipo == Tipo::Guapo);
        assert!((po.desde..po.desde + VISTA_W).contains(&guapo[0].x));
        assert!(
            arrabal.puerta > po.desde + VISTA_W,
            "la puerta, pasada la arena"
        );
        assert!(po.ficha.is_some_and(|i| i < arrabal.fichas.len()));
        // La sudestada: un tramo sin acera que no se salta, y el tranvia de
        // una orilla a la otra.
        let pl = arrabal
            .plataformas
            .iter()
            .find(|pl| pl.mueve.is_some())
            .expect("falta el tranvia");
        let Some(Mueve::Vaiven { dx, .. }) = pl.mueve else {
            panic!("el tranvia va y viene")
        };
        assert!(
            !arrabal.hay_suelo(pl.x + pl.ancho * 0.5),
            "el tranvia, en el agua"
        );
        assert!(arrabal.hay_suelo(pl.x - 20.0), "sale de una orilla");
        assert!(
            arrabal.hay_suelo(pl.x + pl.ancho + dx + 20.0),
            "y llega a la otra"
        );

        // Montmartre: sube, lleva a su jefe y trae los enemigos del cancan.
        let mont = defs
            .iter()
            .find(|d| d.nombre == "El paseo de Montmartre")
            .expect("falta Montmartre");
        assert_eq!(mont.jefe, "El Cancan", "Montmartre va antes del cancan");
        assert!(mont.hay_suelo(mont.salida) && mont.hay_suelo(mont.puerta));
        assert!(mont.puerta < mont.largo);
        for par in mont.suelo.windows(2) {
            assert!(par[0].1 < par[1].0, "tramos desordenados: {par:?}");
        }
        assert!(
            mont.altura(mont.puerta) < mont.altura(mont.salida) - 400.0,
            "Montmartre es una cuesta"
        );
        for t in [Tipo::Corista, Tipo::Botella, Tipo::Pintor] {
            assert!(mont.enemigos.iter().any(|a| a.tipo == t), "falta {t:?}");
        }
        assert!(mont.enemigos.iter().any(|a| a.rosa));
        // Las escaleras de todos, dentro de lo que la fisica perdona y sin
        // empezar ni acabar en un foso: un borde de foso a media rampa no
        // tiene una altura que se lea.
        for d in &defs {
            for e in &d.escaleras {
                assert!(
                    e.sube / e.ancho <= PENDIENTE_MAX,
                    "escalera empinada: {e:?}"
                );
                assert!(d.hay_suelo(e.x) && d.hay_suelo(e.x + e.ancho), "{e:?}");
            }
        }
    }

    #[test]
    fn un_ron_roto_se_rechaza() {
        assert!(PaseoDef::from_ron("(nombre: \"a\",").is_err());
    }

    #[test]
    fn la_jugadora_aterriza_en_la_acera_y_se_queda() {
        let mut p = Paseo::new(calle(), 1);
        // Se la suelta desde arriba para que tenga que caer.
        p.jugadora.pos.y -= 120.0;
        p.jugadora.on_ground = false;
        correr(&mut p, 0, 90);
        assert!(p.jugadora.on_ground);
        assert_eq!(p.jugadora.pos.y + PIES, SUELO_Y, "los pies en la acera");
        correr(&mut p, DERECHA, 60);
        assert!(p.jugadora.on_ground, "andar no la hunde");
        assert_eq!(p.jugadora.pos.y + PIES, SUELO_Y);
    }

    #[test]
    fn saltar_sube_y_vuelve_a_la_acera() {
        let mut p = Paseo::new(calle(), 1);
        p.step(pulsar(InputFrame::JUMP));
        correr(&mut p, InputFrame::JUMP, 15);
        assert!(p.jugadora.pos.y + PIES < SUELO_Y - 100.0, "no ha subido");
        correr(&mut p, 0, 60);
        assert!(p.jugadora.on_ground);
    }

    #[test]
    fn las_plataformas_se_atraviesan_por_debajo_y_se_pisan_por_encima() {
        let mut def = calle();
        def.plataformas = vec![Plataforma {
            x: 50.0,
            y: 400.0,
            ancho: 100.0,
            mueve: None,
        }];
        let mut p = Paseo::new(def, 1);
        p.step(pulsar(InputFrame::JUMP));
        correr(&mut p, InputFrame::JUMP, 60);
        assert!(p.jugadora.on_ground);
        assert_eq!(p.jugadora.pos.y + PIES, 400.0, "tenia que acabar encima");
    }

    #[test]
    fn caer_a_un_foso_cuesta_una_vida_y_devuelve_a_la_acera() {
        let mut def = calle();
        def.suelo = vec![(0.0, 400.0), (700.0, 3200.0)];
        let mut p = Paseo::new(def, 1);
        let mut caida = false;
        for _ in 0..240 {
            p.step(pulsar(DERECHA));
            caida |= p.events.player_died;
            if caida {
                break;
            }
        }
        assert!(caida, "tenia que caerse al foso");
        assert_eq!(p.vidas, VIDAS - 1);
        assert!(p.def.hay_suelo(p.jugadora.pos.x), "vuelve a la acera");
        assert!(p.jugadora.pos.x < 400.0, "a la de antes del foso");
        assert!(p.jugadora.is_invulnerable(), "y con margen para respirar");
    }

    #[test]
    fn tras_caer_hay_que_soltar_el_lado_para_volver_a_andar() {
        let mut def = calle();
        def.suelo = vec![(0.0, 400.0), (700.0, 3200.0)];
        let mut p = Paseo::new(def, 1);
        while !p.events.player_died {
            p.step(pulsar(DERECHA));
        }
        // Sigue con el stick a la derecha: no se mueve, no se vuelve a tirar.
        let x = p.jugadora.pos.x;
        for _ in 0..120 {
            p.step(pulsar(DERECHA));
        }
        assert_eq!(p.vidas, VIDAS - 1, "no se cae otra vez en bucle");
        assert!((p.jugadora.pos.x - x).abs() < 1.0, "sin soltar no anda");
        // Suelta un tick y vuelve a pulsar: ahora si.
        p.step(InputFrame::NONE);
        for _ in 0..30 {
            p.step(pulsar(DERECHA));
        }
        assert!(p.jugadora.pos.x > x + 20.0, "tras soltar, anda");
    }

    #[test]
    fn un_foso_es_una_pared_desde_dentro() {
        // Con los pies por debajo de la acera no se puede andar a traves de un
        // tramo de suelo como si fuera aire.
        let mut def = calle();
        def.suelo = vec![(0.0, 400.0), (600.0, 3200.0)];
        let mut p = Paseo::new(def, 1);
        p.jugadora.pos = Vec2::new(500.0, SUELO_Y + 40.0);
        p.jugadora.on_ground = false;
        for _ in 0..8 {
            p.step(pulsar(DERECHA));
            assert!(p.jugadora.pos.x < 600.0, "ha atravesado la acera");
        }
    }

    #[test]
    fn los_disparos_van_de_frente_y_tumban_a_una_pareja() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Pareja,
                x: 450.0,
                y: 0.0,
                rosa: false,
                portero: false,
            }]),
            1,
        );
        p.step(pulsar(DISPARA));
        assert!(p.events.player_shot);
        assert!(p.disparos[0].vel.x > 0.0 && p.disparos[0].vel.y == 0.0);
        let mut cayo = false;
        for _ in 0..90 {
            p.step(pulsar(DISPARA));
            cayo |= p.caidos > 0;
        }
        assert!(cayo, "la pareja tenia que caer");
        assert!(p.enemigos.is_empty());
        assert_eq!(p.vidas, VIDAS, "sin dejarla llegar");
    }

    #[test]
    fn plantada_apunta_arriba_y_no_salta() {
        let mut p = Paseo::new(calle(), 1);
        let arriba = InputFrame::FOCUS | InputFrame::UP | InputFrame::JUMP | DISPARA;
        correr(&mut p, arriba, 10);
        assert!(p.jugadora.on_ground, "plantada no salta");
        assert_eq!(p.apunta, Vec2::new(0.0, -1.0));
        assert!(p.disparos.iter().all(|d| d.vel.y < 0.0 && d.vel.x == 0.0));
    }

    #[test]
    fn tras_un_golpe_hay_invulnerabilidad() {
        // Una pareja que viene bailando hacia ella, y la jugadora quieta.
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Pareja,
                x: 220.0,
                y: 0.0,
                rosa: false,
                portero: false,
            }]),
            1,
        );
        let mut n = 0;
        while !p.events.player_died {
            p.step(InputFrame::NONE);
            n += 1;
            assert!(n < 600, "la pareja no ha llegado nunca");
        }
        assert_eq!(p.vidas, VIDAS - 1);
        // Mientras dura, la pareja puede pasarle por encima sin hacer nada.
        for _ in 1..INVULNERABLE_TRAS_GOLPE {
            p.step(InputFrame::NONE);
            assert!(!p.events.player_died, "golpe durante la invulnerabilidad");
        }
        // Y se acaba cuando dice: al siguiente tick ya se puede recibir (y si
        // la pareja sigue encima, recibe).
        p.step(InputFrame::NONE);
        assert!(
            !p.jugadora.is_invulnerable() || p.events.player_died,
            "la invulnerabilidad dura de mas"
        );
    }

    #[test]
    fn sin_vidas_se_acaba() {
        let mut p = Paseo::new(calle(), 1);
        for _ in 0..VIDAS {
            p.jugadora.iframes = 0;
            p.golpe();
        }
        assert!(p.derrota && p.terminado());
        assert!(p.events.defeat);
    }

    #[test]
    fn parriar_un_plato_rosa_lo_quita_y_llena_el_medidor() {
        let mut p = Paseo::new(calle(), 1);
        let pos = p.jugadora.pos + Vec2::new(20.0, -10.0);
        p.proyectiles.push(Proyectil {
            pos,
            vel: Vec2::ZERO,
            rosa: true,
            forma: Forma::Plato,
        });
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
        assert!(p.proyectiles.is_empty());
        assert!(p.jugadora.meter > 0.0);
        assert_eq!(p.vidas, VIDAS, "parriado no hace dano");
    }

    #[test]
    fn el_camarero_tira_platos_en_arco_y_uno_de_cada_tres_es_rosa() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Camarero,
                x: 600.0,
                y: 0.0,
                rosa: false,
                portero: false,
            }]),
            1,
        );
        // Invulnerable, para contar platos sin que la maten.
        let mut rosas = 0;
        let mut vistos = 0;
        let mut antes = 0;
        for _ in 0..(CADENCIA_CAMARERO * 7) {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            let ahora = p.enemigos[0].lanzados;
            if ahora > antes {
                vistos += 1;
                let q = p.proyectiles.last().expect("acaba de tirar");
                assert!(q.vel.y < 0.0, "un plato sale hacia arriba");
                rosas += u32::from(q.rosa);
                antes = ahora;
            }
        }
        assert!(vistos >= 6, "tira con cadencia: {vistos}");
        assert_eq!(rosas, vistos / 3);
    }

    #[test]
    fn llegar_a_la_puerta_completa_el_paseo() {
        let mut p = Paseo::new(calle(), 1);
        let mut llego = false;
        for _ in 0..(60 * 20) {
            p.step(pulsar(DERECHA));
            llego |= p.events.victory;
        }
        assert!(llego && p.completado.is_some());
        assert_eq!(p.progreso(), 1.0);
        // Y ya no se mueve: la escena ha acabado.
        let x = p.jugadora.pos.x;
        correr(&mut p, DERECHA, 30);
        assert_eq!(p.jugadora.pos.x, x);
    }

    #[test]
    fn el_mismo_input_da_el_mismo_paseo() {
        // Un minuto largo de cada paseo con input sembrado: dos paseos iguales
        // tienen que ir tick a tick a la par, enemigos incluidos.
        for def in PaseoDef::de_serie() {
            mismo_input_mismo_paseo(def);
        }
    }

    fn mismo_input_mismo_paseo(def: PaseoDef) {
        let mut r = Pcg32::new(7);
        let inputs: Vec<InputFrame> = (0..4000)
            .map(|_| {
                // Mucha derecha y disparo, para que se ande y pase de todo.
                let azar = (r.next_u32() & 0x3FF) as u16;
                pulsar(
                    azar | if r.next_f32() < 0.8 {
                        DERECHA | DISPARA
                    } else {
                        0
                    },
                )
            })
            .collect();
        let mut a = Paseo::new(def.clone(), 0x5641_4C53);
        let mut b = Paseo::new(def.clone(), 0x5641_4C53);
        for (i, input) in inputs.iter().enumerate() {
            a.step(*input);
            b.step(*input);
            assert_eq!(a.huella(), b.huella(), "{}: divergencia en {i}", def.nombre);
        }
        assert!(
            a.jugadora.pos.x > def.salida + 500.0,
            "el input no ha andado"
        );

        // Y la huella ve lo que pasa: otro input, otro paseo.
        let mut c = Paseo::new(def, 0x5641_4C53);
        c.step(pulsar(InputFrame::LEFT));
        let mut d = Paseo::new(c.def.clone(), 0x5641_4C53);
        d.step(pulsar(DERECHA));
        assert_ne!(c.huella(), d.huella());
    }

    #[test]
    fn la_clave_va_tres_tres_dos() {
        // A 36 ticks por tiempo la corchea son 18: golpes en 0, 54 y 108.
        let golpes: Vec<u64> = (0..288).filter(|&t| clave(t, 36).1 == 0).collect();
        assert_eq!(golpes, [0, 54, 108, 144, 198, 252]);
        assert_eq!(clave(110, 36), (2, 2, 36));
    }

    fn uno(tipo: Tipo, x: f32, y: f32) -> PaseoDef {
        con(vec![Aparicion {
            tipo,
            x,
            y,
            rosa: false,
            portero: false,
        }])
    }

    #[test]
    fn la_flapper_suelta_perlas_en_la_clave_y_el_collar_gira() {
        let mut def = uno(Tipo::Flapper, 500.0, 0.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        let mut angulos = Vec::new();
        let mut rosas = 0;
        for _ in 0..(144 * 3) {
            p.jugadora.iframes = 10;
            let tick = p.tick;
            let antes = p.proyectiles.len();
            p.step(InputFrame::NONE);
            let nuevas = &p.proyectiles[antes.min(p.proyectiles.len())..];
            if nuevas.is_empty() {
                continue;
            }
            // Solo en los golpes de la clave, y de tres en tres.
            assert_eq!(clave(tick, 36).1, 0, "perlas fuera de la clave en {tick}");
            assert_eq!(nuevas.len(), PERLAS_POR_GOLPE);
            assert!(nuevas.iter().all(|q| q.forma == Forma::Perla));
            rosas += nuevas.iter().filter(|q| q.rosa).count();
            let v = nuevas[0].vel;
            angulos.push(crate::math::atan2(v.y, v.x));
        }
        assert_eq!(angulos.len(), 9, "tres golpes por compas");
        assert_eq!(rosas, 3, "una rosa por compas, en el golpe de dos");
        // El collar gira: dos golpes seguidos nunca salen por el mismo sitio.
        for par in angulos.windows(2) {
            assert!((par[0] - par[1]).abs() > 0.2, "el collar no gira: {par:?}");
        }
        // Y las perlas vuelan rectas: sin gravedad, la velocidad no cambia.
        let v = p.proyectiles[0].vel;
        p.step(InputFrame::NONE);
        assert_eq!(p.proyectiles[0].vel, v);
    }

    #[test]
    fn el_saxo_sopla_rafagas_de_tres_tres_dos_apuntadas() {
        // En una escalera por encima de la jugadora.
        let mut def = uno(Tipo::Saxo, 400.0, 330.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        let mut rafagas: Vec<usize> = Vec::new();
        let mut rosas = 0;
        for _ in 0..(144 * 2) {
            p.jugadora.iframes = 10;
            let (_, dentro, _) = clave(p.tick, 36);
            let antes = p.enemigos.first().map_or(0, |e| e.lanzados);
            p.step(InputFrame::NONE);
            if p.enemigos[0].lanzados == antes {
                continue;
            }
            let q = *p.proyectiles.last().expect("acaba de soplar");
            // Apuntada a ella: hacia abajo y hacia la izquierda.
            let a_ella = (p.jugadora.pos - q.pos).normalize();
            assert!(q.vel.normalize().dot(a_ella) > 0.98, "no apunta");
            assert!(q.vel.y > 0.0 && q.forma == Forma::Corchea);
            rosas += usize::from(q.rosa);
            if dentro == 0 {
                rafagas.push(0);
            }
            *rafagas.last_mut().expect("rafaga empezada") += 1;
        }
        assert_eq!(rafagas, [3, 3, 2, 3, 3, 2]);
        assert_eq!(rosas, 2, "una rosa por compas");
        // Y despierto pero con la jugadora fuera de su alcance, calla.
        let lejos = p.def.salida + ALCANCE_CHICAGO + 40.0;
        let mut p = Paseo::new(uno(Tipo::Saxo, lejos, 330.0), 1);
        correr(&mut p, 0, 144);
        assert_eq!(p.enemigos.len(), 1, "tenia que estar despierto");
        assert!(p.proyectiles.is_empty(), "sopla sin nadie delante");
    }

    #[test]
    fn el_saltarin_brinca_alto_alto_bajo_y_no_se_tira() {
        // En una azotea corta, sin acera debajo, y la jugadora lejos a su
        // izquierda: tiene que venir brincando y pararse en el borde.
        let mut def = uno(Tipo::Saltarin, 700.0, 300.0);
        def.tiempo = 36;
        def.suelo = vec![(0.0, 200.0), (900.0, 3200.0)];
        def.plataformas = vec![Plataforma {
            x: 300.0,
            y: 300.0,
            ancho: 500.0,
            mueve: None,
        }];
        let mut p = Paseo::new(def, 1);
        p.jugadora.iframes = 10;
        let pie = 300.0 - Tipo::Saltarin.alto();
        let mut cumbres = [0.0_f32; 3];
        let mut xs = Vec::new();
        for _ in 0..(144 * 4) {
            p.jugadora.iframes = 10;
            let (golpe, _, _) = clave(p.tick, 36);
            p.step(InputFrame::NONE);
            let e = &p.enemigos[0];
            cumbres[golpe] = cumbres[golpe].max(pie - e.pos.y);
            xs.push(e.pos.x);
        }
        // Dos brincos por encima de ella y uno que hay que saltar.
        let cabeza = PIES + CUERPO_MEDIO + CUERPO_RADIO;
        assert!(cumbres[0] > 100.0 && cumbres[1] > 100.0, "{cumbres:?}");
        assert!(
            cumbres[2] < cabeza,
            "el bajo se pasaba por debajo: {cumbres:?}"
        );
        // Ha venido hacia ella, y sin salirse de su tejado.
        let e = &p.enemigos[0];
        assert!(xs.iter().all(|&x| (300.0..=800.0).contains(&x)), "se tiro");
        assert!(e.pos.x < 420.0, "no ha venido: {}", e.pos.x);
        assert_eq!(e.pos.y, pie, "acaba con los pies en el tejado");
    }

    #[test]
    fn caer_desde_una_azotea_devuelve_a_la_azotea() {
        // Dos tejados sin calle debajo. Se cae entre los dos: tiene que volver
        // al primero, no a la acera de la salida.
        let mut def = calle();
        def.suelo = vec![(0.0, 300.0), (2000.0, 3200.0)];
        def.plataformas = vec![
            Plataforma {
                x: 300.0,
                y: 470.0,
                ancho: 400.0,
                mueve: None,
            },
            Plataforma {
                x: 1000.0,
                y: 470.0,
                ancho: 400.0,
                mueve: None,
            },
        ];
        let mut p = Paseo::new(def, 1);
        let mut caida = false;
        for _ in 0..240 {
            p.step(pulsar(DERECHA));
            caida |= p.events.player_died;
            if caida {
                break;
            }
        }
        assert!(caida, "tenia que caerse entre los tejados");
        let x = p.jugadora.pos.x;
        assert!((340.0..=660.0).contains(&x), "vuelve al tejado: {x}");
        assert_eq!(p.jugadora.pos.y + PIES, 470.0);
    }

    /// Un bot invulnerable que corre y dispara, y salta cuando tiene un hueco
    /// delante o algo que subir. Como en Viena, no prueba que sea facil: prueba
    /// que se puede. En Chicago eso incluye subir la escalera de incendios.
    fn bot_anda(def: PaseoDef) -> Paseo {
        let mut p = Paseo::new(def, 3);
        let mut salto = 0;
        for _ in 0..(60 * 90) {
            p.jugadora.iframes = 5;
            let j = &p.jugadora;
            let (x, pies) = (j.pos.x, j.pos.y + PIES);
            // Hueco: nada que pisar un poco por delante, ni a la altura de los
            // pies ni en los 200 de debajo.
            let hay_debajo = |dx: f32| {
                (pies <= SUELO_Y && p.def.hay_suelo(x + dx) && SUELO_Y - pies < 200.0)
                    || p.def.plataformas.iter().any(|pl| {
                        pl.y >= pies - 0.5
                            && pl.y - pies < 200.0
                            && x + dx >= pl.x
                            && x + dx <= pl.x + pl.ancho
                    })
            };
            // Algo que subir: una plataforma por encima, a tiro de salto, y
            // donde se caeria corriendo (unas 130 por delante) dentro de ella.
            let subir = p.def.plataformas.iter().any(|pl| {
                let cae = x + 130.0;
                pies - pl.y > 20.0
                    && pies - pl.y < 140.0
                    && cae > pl.x + 10.0
                    && cae < pl.x + pl.ancho - 10.0
            });
            // Sin nada donde caer a tiro de salto y con una plataforma que se
            // mueve cerca (el tren), espera a que llegue. Y en la arena del
            // portero se planta a dispararle de frente. Se alcanza saltando
            // algo debajo a menos de un salto, o un tejado por encima, mas
            // cerca cuanto mas alto.
            let alcanza = (4..=21).any(|k| hay_debajo(10.0 * k as f32))
                || p.def.plataformas.iter().any(|pl| {
                    let alto = pies - pl.y;
                    alto > 0.0
                        && alto < 140.0
                        && pl.x + pl.ancho > x + 40.0
                        && pl.x - x <= 230.0 - alto * 0.8
                });
            let espera = j.on_ground
                && !alcanza
                && p.def
                    .plataformas
                    .iter()
                    .any(|pl| pl.mueve.is_some() && (pl.x - x).abs() < 1200.0)
                || p.cierre().is_some();
            if j.on_ground && !espera && (!hay_debajo(30.0) || subir) {
                salto = 22;
            }
            let anda = if espera { 0 } else { DERECHA };
            let bits = anda | DISPARA | if salto > 0 { InputFrame::JUMP } else { 0 };
            salto = if salto > 0 { salto - 1 } else { 0 };
            p.step(pulsar(bits));
            if p.completado.is_some() {
                break;
            }
        }
        p
    }

    #[test]
    fn chicago_se_puede_andar_entera() {
        let p = bot_anda(de_serie("El Charleston"));
        assert!(p.completado.is_some(), "atascada en x {}", p.jugadora.pos.x);
        assert_eq!(p.vidas, VIDAS, "no se ha caido de ningun tejado");
        // Ha subido al tren y ha tumbado al contrabajista disparando de
        // frente: sin el no se abre la puerta, y su ficha estaba de camino.
        assert!(p.fichas[4], "sin la ficha del contrabajista");
    }

    #[test]
    fn el_canotier_da_una_vuelta_por_compas_a_tirones_en_la_clave() {
        let mut def = uno(Tipo::Canotier, 600.0, 380.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        p.step(InputFrame::NONE);
        let centro =
            |p: &Paseo| Vec2::new(600.0 - 110.0 * p.tick.saturating_sub(1) as f32 * DT, 380.0);
        let mut pasos = [[0.0_f32; 2]; 3];
        let mut antes = p.enemigos[0].pos - centro(&p);
        let al_empezar = antes;
        for _ in 0..144 {
            let (golpe, dentro, _) = clave(p.tick, 36);
            p.step(InputFrame::NONE);
            let e = &p.enemigos[0];
            let rel = e.pos - centro(&p);
            // El tiron, los primeros ticks del golpe; el planeo, el resto.
            pasos[golpe][usize::from(dentro >= TIRON_CANOTIER)] += (rel - antes).length();
            antes = rel;
            let (rx, ry) = VUELTA_CANOTIER;
            assert!(
                rel.x.abs() <= rx + 0.01 && rel.y.abs() <= ry + 0.01,
                "fuera de su vuelta"
            );
        }
        for [tiron, planeo] in pasos {
            assert!(tiron > 60.0 && planeo < 1.0, "{pasos:?}");
        }
        // Una vuelta por compas, y el centro ha venido hacia ella.
        assert!(
            (antes - al_empezar).length() < 0.5,
            "{antes} vs {al_empezar}"
        );
        assert!(p.enemigos[0].pos.x < 600.0 - 200.0);
        // En el picado baja a la altura de ella: tiene que verse venir.
        let (_, ry) = VUELTA_CANOTIER;
        let toca = Tipo::Canotier.radio() * 0.8 + CUERPO_RADIO;
        assert!(380.0 + ry * 0.5 + toca > p.jugadora.pos.y - CUERPO_MEDIO);
    }

    #[test]
    fn un_canotier_rosa_se_parria() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                rosa: true,
                ..aparece(Tipo::Canotier, 600.0)
            }]),
            1,
        );
        p.step(InputFrame::NONE);
        assert!(p.enemigos[0].rosa);
        p.enemigos[0].avance = p.jugadora.pos.x + 20.0;
        p.enemigos[0].base_y = p.jugadora.pos.y + 20.0;
        p.step(InputFrame::NONE);
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
        assert!(p.enemigos.is_empty());
    }

    #[test]
    fn la_fila_sale_entera_y_abre_el_hueco_en_la_clave() {
        let mut def = uno(Tipo::Fila, 900.0, 0.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        p.jugadora.pos.x = 400.0;
        let mut abierto = false;
        let mut cerrado = false;
        let mut centro = f32::MAX;
        for _ in 0..(144 * 2) {
            p.jugadora.iframes = 10;
            let (golpe, dentro, _) = clave(p.tick, 36);
            p.step(InputFrame::NONE);
            let fila: Vec<&Enemigo> = p.enemigos.iter().filter(|e| e.tipo == Tipo::Fila).collect();
            assert_eq!(fila.len(), FILA as usize, "salen todas a la vez");
            let xs: Vec<f32> = fila.iter().map(|e| e.pos.x).collect();
            // Las de cada lado van hombro con hombro; el hueco, en el medio.
            assert!((xs[1] - xs[0] - ESPACIO_FILA).abs() < 1e-3);
            assert!((xs[3] - xs[2] - ESPACIO_FILA).abs() < 1e-3);
            let hueco = xs[2] - xs[1] - ESPACIO_FILA;
            if dentro >= TICKS_HUECO {
                if golpe == 0 {
                    assert!(hueco.abs() < 1e-3, "en el uno, cerrado: {hueco}");
                    cerrado = true;
                } else {
                    assert!((hueco - HUECO_FILA).abs() < 1e-3, "abierto: {hueco}");
                    abierto = true;
                }
            }
            // Avanza a pasos, en los golpes: el resto del golpe, quieta.
            let ahora = (xs[1] + xs[2]) * 0.5;
            if dentro >= TICKS_PASO_FILA {
                assert_eq!(ahora, centro, "anda fuera del paso");
            }
            centro = ahora;
        }
        assert!(abierto && cerrado);
        assert!(centro < 900.0 - 100.0, "no ha avanzado: {centro}");
        // Abierto, en el hueco cabe ella de sobra; cerrado, no hay por donde.
        let toca = Tipo::Fila.radio() * 0.8 + CUERPO_RADIO;
        assert!(ESPACIO_FILA + HUECO_FILA - 2.0 * toca > 4.0 * CUERPO_RADIO);
        assert!(ESPACIO_FILA < 2.0 * toca);
    }

    #[test]
    fn a_la_fila_se_la_tumba_de_una_en_una_y_no_se_descoloca() {
        let mut def = uno(Tipo::Fila, 600.0, 0.0);
        def.tiempo = 36;
        let mut p = Paseo::new(def, 1);
        let mut caidas = 0;
        for _ in 0..240 {
            p.jugadora.iframes = 10;
            p.step(pulsar(DISPARA));
            caidas += p.caidos;
            if caidas > 0 {
                break;
            }
        }
        assert_eq!(caidas, 1, "una cae y las otras siguen");
        // La de delante es la que cae, y las que quedan siguen en su sitio.
        let hueco = hueco_fila(p.tick - 1, 36);
        for e in &p.enemigos {
            assert!(e.lanzados > 0);
            assert!((e.pos.x - e.avance - sitio_en_la_fila(e.lanzados, hueco)).abs() < 1e-3);
        }
    }

    /// Chicago, con la jugadora en el techo del tren en el tick 0, que es
    /// cuando el tren esta en el tejado de la izquierda.
    fn en_el_tren() -> Paseo {
        let mut p = Paseo::new(de_serie("El Charleston"), 1);
        let pl = p
            .def
            .plataformas
            .iter()
            .find(|pl| pl.mueve.is_some())
            .copied()
            .expect("sin tren");
        p.jugadora.pos = Vec2::new(pl.x + 100.0, pl.y - PIES);
        p.jugadora.prev_pos = p.jugadora.pos;
        p.siguiente = p.def.enemigos.len();
        p
    }

    #[test]
    fn el_tren_cruza_el_vacio_entre_los_tejados() {
        // Sin el tren no se cruza: ni saltando con un dash al final.
        let def = de_serie("El Charleston");
        let tejado = |x: f32| {
            def.plataformas
                .iter()
                .find(move |pl| pl.mueve.is_none() && pl.x <= x && x <= pl.x + pl.ancho)
        };
        assert!(tejado(4250.0).is_some() && tejado(5300.0).is_some());
        assert!(
            (4290..5280).all(|x| tejado(x as f32).is_none()),
            "hay por donde cruzar"
        );

        // Quieta en el techo: el tren la lleva al otro lado sin caerse.
        let mut p = en_el_tren();
        let x0 = p.jugadora.pos.x;
        for _ in 0..240 {
            p.step(InputFrame::NONE);
            assert!(
                p.jugadora.on_ground,
                "se ha caido del tren en x {}",
                p.jugadora.pos.x
            );
        }
        assert!(
            (p.jugadora.pos.x - (x0 + 584.0)).abs() < 2.0,
            "x {}",
            p.jugadora.pos.x
        );
        // Anda al otro vagon sin caerse por la rendija, y salta al tejado.
        while p.jugadora.pos.x < 5180.0 {
            p.step(pulsar(DERECHA));
            assert!(p.jugadora.on_ground, "se ha caido entre los vagones");
        }
        correr(&mut p, DERECHA | InputFrame::JUMP, 40);
        correr(&mut p, DERECHA, 10);
        assert!(p.jugadora.on_ground && p.jugadora.pos.x > 5280.0);
        assert_eq!(p.jugadora.pos.y + PIES, 240.0, "en el tejado de enfrente");
        assert_eq!(p.vidas, VIDAS);
    }

    /// El contrabajista en su arena (de 1000 a 1960), y la jugadora dentro.
    fn con_contrabajista(vida: i32) -> Paseo {
        let mut def = con(vec![Aparicion {
            portero: true,
            ..aparece(Tipo::Contrabajista, 1700.0)
        }]);
        def.tiempo = 36;
        def.porteria = Some(Porteria {
            desde: 1000.0,
            ficha: Some(0),
        });
        def.fichas = vec![(1700.0, 430.0)];
        def.puerta = 2500.0;
        let mut p = Paseo::new(def, 1);
        while p.cierre().is_none() {
            p.jugadora.iframes = 10;
            p.step(pulsar(DERECHA));
            assert!(p.proyectiles.is_empty(), "toca antes de cerrarse la arena");
        }
        p.enemigos[0].vida = vida;
        p
    }

    /// Lo que suelta en `compases` compases desde el siguiente uno: las
    /// negras de cada golpe (rosas aparte) y en que golpes hay slap.
    fn lo_que_toca(p: &mut Paseo, compases: u64) -> (Vec<(usize, usize, usize)>, Vec<u64>) {
        while clave(p.tick, 36) != (0, 0, 54) {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
        }
        let (mut negras, mut slaps) = (Vec::new(), Vec::new());
        for _ in 0..(144 * compases) {
            p.jugadora.iframes = 10;
            let (golpe, _, _) = clave(p.tick, 36);
            let antes = p.proyectiles.len();
            let tick = p.tick;
            p.step(InputFrame::NONE);
            let nuevas = &p.proyectiles[antes.min(p.proyectiles.len())..];
            if nuevas.is_empty() {
                continue;
            }
            assert_eq!(clave(tick, 36).1, 0, "fuera de la clave");
            if nuevas.iter().all(|q| q.forma == Forma::Onda) {
                assert_eq!(nuevas.len(), 2, "una onda a cada lado");
                slaps.push(tick / 144);
            } else {
                assert!(nuevas.iter().all(|q| q.forma == Forma::Negra));
                let rosas = nuevas.iter().filter(|q| q.rosa).count();
                negras.push((golpe, nuevas.len(), rosas));
            }
        }
        (negras, slaps)
    }

    #[test]
    fn el_contrabajista_tira_negras_en_arco_en_la_clave() {
        let mut p = con_contrabajista(Tipo::Contrabajista.vida());
        assert_eq!(p.portero().map(Enemigo::fase), Some(0));
        let (negras, slaps) = lo_que_toca(&mut p, 2);
        assert!(slaps.is_empty(), "sin slap con la vida entera");
        let compas = [(0, 3, 0), (1, 3, 0), (2, 2, 1)];
        assert_eq!(
            negras,
            [compas, compas].concat(),
            "tres, tres y dos, la ultima rosa"
        );
        // En arco: van hacia ella, a velocidad fija, y se tuercen al suelo.
        let q = *p
            .proyectiles
            .iter()
            .find(|q| q.forma == Forma::Negra)
            .expect("sin negras");
        assert!(q.vel.x < 0.0);
        let (v, bajada) = (q.vel.length(), q.vel.y);
        p.step(InputFrame::NONE);
        let q2 = p
            .proyectiles
            .iter()
            .find(|n| n.forma == Forma::Negra)
            .expect("sin negras");
        assert!((q2.vel.length() - v).abs() < 1e-2 && q2.vel.y > bajada);
    }

    #[test]
    fn el_contrabajista_da_el_slap_segun_la_vida_que_le_queda() {
        let max = Tipo::Contrabajista.vida();
        // Segunda fase: un uno de cada dos es slap; tercera, todos.
        for (vida, fase, slaps) in [(max * 2 / 3 - 1, 1, 2), (max / 3 - 1, 2, 4)] {
            let mut p = con_contrabajista(vida);
            assert_eq!(p.portero().map(Enemigo::fase), Some(fase));
            let (negras, hechos) = lo_que_toca(&mut p, 4);
            assert_eq!(hechos.len(), slaps, "fase {fase}: {hechos:?}");
            // El dos sigue siendo rosa: el parry no se va con la vida.
            assert_eq!(negras.iter().filter(|n| n.0 == 2 && n.2 == 1).count(), 4);
        }
        // La onda va a ras de acera, y se salta.
        let mut p = con_contrabajista(max / 3 - 1);
        while !p.proyectiles.iter().any(|q| q.forma == Forma::Onda) {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
        }
        let onda = p
            .proyectiles
            .iter()
            .find(|q| q.forma == Forma::Onda && q.vel.x < 0.0)
            .copied()
            .expect("sin onda");
        assert_eq!(onda.pos.y + Forma::Onda.radio(), SUELO_Y);
        for salta in [false, true] {
            let mut p = Paseo::new(calle(), 1);
            p.proyectiles.push(Proyectil {
                pos: Vec2::new(p.jugadora.pos.x + 120.0, onda.pos.y),
                ..onda
            });
            let bits = if salta { InputFrame::JUMP } else { 0 };
            let mut dado = false;
            for _ in 0..40 {
                p.step(pulsar(bits));
                dado |= p.events.player_died;
            }
            assert_eq!(dado, !salta, "saltando {salta}");
        }
    }

    #[test]
    fn el_contrabajista_cae_a_tiros_y_abre_la_arena() {
        let mut p = con_contrabajista(Tipo::Contrabajista.vida());
        let mut ticks = 0;
        let mut fases = Vec::new();
        while p.portero_vivo() {
            p.jugadora.iframes = 10;
            p.step(pulsar(DISPARA));
            ticks += 1;
            if let Some(e) = p.portero()
                && fases.last() != Some(&e.fase())
            {
                fases.push(e.fase());
            }
            assert!(ticks < 60 * 60, "no cae");
        }
        assert_eq!(fases, [0, 1, 2], "pasa por las tres fases");
        // Con la aguja y sin fallar una, unos 11 segundos: con lo que se
        // esquiva, unos quince.
        let segundos = ticks as f32 * DT;
        assert!((9.0..16.0).contains(&segundos), "{segundos} s");
        assert!(p.cierre().is_none(), "la arena sigue cerrada");
        assert!(p.ficha_visible(0), "sin su ficha");
    }

    #[test]
    fn viena_se_puede_andar_entera() {
        // Un bot que corre, dispara y salta cuando tiene un foso delante. Es
        // invulnerable, asi que no prueba que sea facil: prueba que ningun foso
        // es mas ancho que un salto y que no hay nada que tape la puerta.
        //
        // Dos cosas mas que en las otras calles: el canal del fiacre no se
        // salta, se espera a la barca en el muelle y se va en ella hasta la
        // otra orilla; y en la arena se planta y dispara de frente hasta
        // tumbar al director.
        let def = de_serie("El Vals");
        let mut p = Paseo::new(def, 3);
        let mut salto = 0;
        let mut tumbado = false;
        for _ in 0..(60 * 120) {
            p.jugadora.iframes = 5;
            let j = &p.jugadora;
            let x = j.pos.x;
            let foso_delante = j.on_ground && !p.def.hay_suelo(x + 24.0);
            // La barca, si la hay sobre el foso que tiene delante.
            let barca = p.def.plataformas.iter().enumerate().find(|(i, pl)| {
                let Some(Mueve::Vaiven { dx, .. }) = pl.mueve else {
                    return false;
                };
                let a = p.bases[*i].x;
                (a..=a + dx + pl.ancho).contains(&(x + 24.0))
            });
            let mut lado = DERECHA;
            if let Some(i) = p.sobre {
                // Montada: al morro de la barca y quieta hasta la otra orilla.
                let pl = p.def.plataformas[i];
                if !p.def.hay_suelo(x + 40.0) && x + 40.0 > pl.x + pl.ancho {
                    lado = 0;
                }
            } else if foso_delante && let Some((_, pl)) = barca {
                // En el muelle: se sube andando si esta aqui, si no espera.
                if !(pl.x <= x + 10.0 && pl.x + pl.ancho >= x + 80.0) {
                    lado = 0;
                }
            } else if foso_delante {
                salto = 20;
            }
            let mut bits = lado | DISPARA | if salto > 0 { InputFrame::JUMP } else { 0 };
            if p.cierre().is_some() {
                bits = InputFrame::FOCUS | DERECHA | DISPARA;
            }
            salto = if salto > 0 { salto - 1 } else { 0 };
            let habia = p.portero().is_some();
            p.step(pulsar(bits));
            tumbado |= habia && p.portero().is_none();
            if p.completado.is_some() {
                break;
            }
        }
        assert!(p.completado.is_some(), "atascada en x {}", p.jugadora.pos.x);
        assert!(tumbado, "el director tenia que caer a tiros");
        assert_eq!(p.fichas_cogidas(), 2, "la del camino y la del director");
        assert!(p.fichas[3], "la del director, al pasar por su tarima");
        assert_eq!(p.vidas, VIDAS, "no ha caido a ningun foso");
    }

    // --- Viena ---

    #[test]
    fn la_hoja_baja_girando_y_se_queda_en_la_acera() {
        let mut p = Paseo::new(uno(Tipo::Hoja, 600.0, 250.0), 1);
        p.step(InputFrame::NONE);
        let salida = p.enemigos[0].pos;
        let (mut ida, mut vuelta) = (false, false);
        let mut antes = salida;
        // Un compas: una vuelta entera, asi que va y vuelve en x.
        for _ in 0..(3 * 21) {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            let e = &p.enemigos[0];
            ida |= e.pos.x > antes.x;
            vuelta |= e.pos.x < antes.x;
            antes = e.pos;
        }
        assert!(ida && vuelta, "no gira");
        assert!(antes.y > salida.y + 20.0, "no baja: {antes}");
        assert!(antes.x < salida.x - 40.0, "no deriva: {antes}");
        // Hasta la acera, y alli se va sin contar como caida.
        let mut caidos = 0;
        for _ in 0..600 {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            caidos += p.caidos;
            if let Some(e) = p.enemigos.first() {
                assert!(e.pos.y < SUELO_Y, "atraviesa la acera");
            }
        }
        assert!(p.enemigos.is_empty(), "no llega a la acera");
        assert_eq!(caidos, 0);
        // Y las rosas despiertan rosas, que es lo que se parria.
        let rosa = con(vec![Aparicion {
            y: 300.0,
            rosa: true,
            ..aparece(Tipo::Hoja, 400.0)
        }]);
        let mut p = Paseo::new(rosa, 1);
        p.step(InputFrame::NONE);
        assert!(p.enemigos[0].rosa);
    }

    #[test]
    fn el_husar_sale_por_detras_y_alcanza_a_quien_corre() {
        let mut def = uno(Tipo::Husar, 1300.0, 0.0);
        (def.largo, def.puerta, def.suelo) = (6000.0, 5800.0, vec![(0.0, 6000.0)]);
        let mut p = Paseo::new(def.clone(), 1);
        let mut salio = None;
        for _ in 0..1500 {
            p.step(pulsar(DERECHA));
            if salio.is_none()
                && let Some(e) = p.enemigos.first()
            {
                salio = Some((e.pos.x, p.jugadora.pos.x, e.dir));
            }
            if p.events.player_died {
                break;
            }
        }
        let (ex, jx, dir) = salio.expect("no ha salido");
        assert!(
            ex < jx - VISTA_W * ENCUADRE,
            "fuera de cuadro: {ex} vs {jx}"
        );
        assert_eq!(dir, 1.0, "viene hacia ella");
        assert!(p.events.player_died, "corriendo no se le escapa");

        // Dandose la vuelta y disparando, cae antes de llegar.
        let mut p = Paseo::new(def, 1);
        while p.enemigos.is_empty() {
            p.step(pulsar(DERECHA));
        }
        let mut cayo = false;
        for _ in 0..120 {
            p.step(pulsar(InputFrame::LEFT | InputFrame::FOCUS | DISPARA));
            cayo |= p.caidos > 0;
        }
        assert!(cayo, "el husar tenia que caer");
        assert_eq!(p.vidas, VIDAS, "sin dejarlo llegar");
    }

    #[test]
    fn la_barca_del_fiacre_cruza_el_canal_de_orilla_a_orilla() {
        let def = de_serie("El Vals");
        let pl = *def
            .plataformas
            .iter()
            .find(|pl| pl.mueve.is_some())
            .expect("falta el fiacre");
        let Some(Mueve::Vaiven {
            dx, dy, periodo, ..
        }) = pl.mueve
        else {
            panic!("el fiacre va y viene");
        };
        // El canal que cruza: de muelle a muelle, mas que un salto con dash.
        let (a, b) = def
            .suelo
            .windows(2)
            .map(|par| (par[0].1, par[1].0))
            .find(|&(a, b)| (a..b).contains(&(pl.x + pl.ancho)))
            .expect("la barca sale de un muelle");
        assert!(b - a > 600.0, "este canal se salta: {}", b - a);
        assert_eq!((dy, pl.y), (0.0, SUELO_Y), "a ras de muelle");
        // Y se mete bajo cada muelle, que es lo que da tiempo a subir y bajar.
        assert!(
            pl.x <= a - 40.0 && pl.x + pl.ancho > a + 100.0,
            "no toca su muelle"
        );
        assert!(
            pl.x + dx < b - 100.0 && pl.x + dx + pl.ancho >= b + 40.0,
            "ni el otro"
        );

        // Se sube andando, se va quieta y se baja andando al otro lado.
        let mut p = Paseo::new(def, 1);
        let y = p.def.altura(a) - PIES;
        p.jugadora.pos = Vec2::new(a - 30.0, y);
        p.jugadora.prev_pos = p.jugadora.pos;
        for _ in 0..20 {
            p.jugadora.iframes = 10;
            p.step(pulsar(DERECHA));
        }
        assert!(p.sobre.is_some(), "no se ha subido");
        while p.tick < u64::from(periodo / 2) {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            assert!(p.jugadora.on_ground, "se ha caido de la barca");
        }
        for _ in 0..60 {
            p.jugadora.iframes = 10;
            p.step(pulsar(DERECHA));
        }
        let j = &p.jugadora;
        assert!(j.pos.x > b && j.on_ground, "en {}", j.pos);
        assert_eq!(p.vidas, VIDAS);
    }

    /// El director en una arena de pruebas que empieza en 1000.
    fn con_director() -> PaseoDef {
        let mut def = con(vec![Aparicion {
            y: 458.0,
            portero: true,
            ..aparece(Tipo::Director, 1750.0)
        }]);
        def.porteria = Some(Porteria {
            desde: 1000.0,
            ficha: None,
        });
        def.puerta = 2500.0;
        def
    }

    /// Anda hasta que se cierra la arena, invulnerable.
    fn hasta_la_arena(def: PaseoDef) -> Paseo {
        let mut p = Paseo::new(def, 1);
        while p.cierre().is_none() {
            p.jugadora.iframes = 10;
            p.step(pulsar(DERECHA));
            assert!(p.proyectiles.is_empty(), "toca antes de cerrarse");
            assert!(p.tick < 600, "la arena no se cierra");
        }
        p
    }

    /// Un tick con lo que vuela borrado: lo que quede es lo que ha nacido.
    fn lo_nuevo(p: &mut Paseo) -> Vec<Proyectil> {
        p.jugadora.iframes = 10;
        p.proyectiles.clear();
        p.step(InputFrame::NONE);
        p.proyectiles.clone()
    }

    #[test]
    fn el_director_toca_a_tres_tiempos_y_la_rosa_va_en_el_tres() {
        let mut p = hasta_la_arena(con_director());
        let (mut espirales, mut rosas) = (0, 0);
        for _ in 0..(3 * 21 * 4) {
            let tick = p.tick;
            let nuevas = lo_nuevo(&mut p);
            if nuevas.is_empty() {
                continue;
            }
            let (golpe, dentro, _) = batuta(tick, 21, 0);
            assert_eq!(dentro, 0, "fuera de tiempo en {tick}");
            if golpe == 2 {
                assert!(nuevas.len() == 1 && nuevas[0].rosa, "el tres: {nuevas:?}");
                // A ella: hacia la izquierda.
                assert!(nuevas[0].vel.x < 0.0);
                rosas += 1;
            } else {
                assert_eq!(nuevas.len(), BRAZOS_ESPIRAL[0]);
                assert!(nuevas.iter().all(|q| !q.rosa && q.forma == Forma::Batuta));
                espirales += 1;
            }
        }
        assert_eq!((espirales, rosas), (8, 4), "cuatro compases");
        // Y la rosa se parria.
        let rosa = Proyectil {
            pos: p.jugadora.pos + Vec2::new(20.0, -10.0),
            vel: Vec2::ZERO,
            rosa: true,
            forma: Forma::Batuta,
        };
        p.proyectiles = vec![rosa];
        p.jugadora.iframes = 0;
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
    }

    #[test]
    fn el_director_cambia_de_fase_por_tercios_y_avisa_el_remolino() {
        let mut p = hasta_la_arena(con_director());
        let max = p.portero().expect("despierto").vida_max();
        for (vida, fase) in [(max, 0), (max * 2 / 3, 1), (max / 3, 2), (1, 2)] {
            p.enemigos[0].vida = vida;
            assert_eq!(p.enemigos[0].fase(), fase, "con {vida} de {max}");
        }
        // En la segunda fase, un compas quieto y luego el anillo.
        p.enemigos[0].vida = max / 2;
        let compas = 3 * 21;
        let mut ultimo_tiro = p.tick;
        let mut anillo = None;
        for _ in 0..(compas * 9) {
            let tick = p.tick;
            let nuevas = lo_nuevo(&mut p);
            if nuevas.iter().any(|q| q.forma == Forma::Remolino) {
                assert_eq!(nuevas.len(), NOTAS_REMOLINO);
                assert!(tick - ultimo_tiro > compas, "sin aviso en {tick}");
                anillo.get_or_insert(nuevas);
            }
            if !p.proyectiles.is_empty() {
                ultimo_tiro = tick;
            }
        }
        let anillo = anillo.expect("no ha soltado el remolino");
        // Las del remolino giran y aceleran hasta el tope, y luego van rectas.
        // La que sale mas de lado, que es la que mas dura en la vista.
        let mut q = *anillo
            .iter()
            .max_by(|a, b| a.vel.x.abs().total_cmp(&b.vel.x.abs()))
            .expect("un anillo tiene notas");
        let v0 = q.vel;
        let mut p = Paseo::new(calle(), 1);
        p.proyectiles = vec![Proyectil {
            pos: Vec2::new(500.0, 270.0),
            ..q
        }];
        correr(&mut p, 0, 30);
        q = p.proyectiles[0];
        assert!(q.vel.length() > v0.length() * 1.3, "no acelera");
        assert!(v0.normalize().dot(q.vel.normalize()) < 0.9, "no gira");
        correr(&mut p, 0, 60);
        let recta = p.proyectiles[0].vel;
        assert!(recta.length() >= TOPE_REMOLINO);
        p.step(InputFrame::NONE);
        assert_eq!(p.proyectiles[0].vel, recta, "pasado el tope, recto");
    }

    #[test]
    fn el_director_cae_a_tiros_en_su_tiempo_y_abre_la_arena() {
        // Plantada a la entrada de la arena, disparando de frente: con la
        // aguja tiene que durar entre 12 y 25 segundos.
        let mut p = hasta_la_arena(con_director());
        let empieza = p.tick;
        let mut vida = p.portero().expect("despierto").vida;
        while p.portero_vivo() {
            p.jugadora.iframes = 10;
            p.step(pulsar(InputFrame::FOCUS | DERECHA | DISPARA));
            if let Some(e) = p.portero() {
                assert!(e.vida <= vida, "se cura");
                vida = e.vida;
            }
            assert!(
                p.tick - empieza < 60 * 25,
                "dura mas de 25 s: le queda {vida}"
            );
        }
        let segundos = (p.tick - empieza) as f32 * DT;
        assert!(segundos > 9.0, "dura poco: {segundos} s");
        assert!(p.cierre().is_none(), "tumbado, se abre");
        while !p.terminado() {
            p.step(pulsar(DERECHA));
            assert!(p.tick < 60 * 60, "no llega a la puerta");
        }
        assert!(p.completado.is_some());
    }

    // --- El arrabal ---

    #[test]
    fn el_arrabal_se_puede_andar_entero() {
        // El bot de Viena, que corre, dispara de frente y salta ante cada
        // foso, con tres cosas mas: ante la sudestada espera al tranvia y se
        // sube, en el tranvia se queda quieto hasta la otra orilla, y en la
        // arena se pone de cara al guapo. A los malevos los pasa de largo.
        let mut p = Paseo::new(de_serie("El Tango"), 3);
        let mut salto = 0;
        let mut tranvia = false;
        let mut tumbado = false;
        for _ in 0..(60 * 150) {
            p.jugadora.iframes = 5;
            let (x, suelo) = (p.jugadora.pos.x, p.jugadora.on_ground);
            let sobre = p
                .sobre
                .map(|i| p.def.plataformas[i])
                .filter(|pl| pl.mueve.is_some() && suelo);
            let orilla = p.def.suelo.iter().map(|t| t.0).find(|&a| a > x);
            let mut lado = DERECHA;
            if let Some(pl) = sobre {
                // Quieto en el techo, hasta que el tranvia llega a la orilla.
                tranvia = true;
                if orilla.is_none_or(|a| a - (pl.x + pl.ancho) > 20.0) {
                    lado = 0;
                }
            } else if let Some(g) = p.cierre().and(p.portero()) {
                let dx = g.pos.x - x;
                lado = match dx {
                    d if d > 40.0 => DERECHA,
                    d if d < -40.0 => InputFrame::LEFT,
                    _ => 0,
                };
            } else if suelo && !p.def.hay_suelo(x + 24.0) {
                // Un foso que se salta, o el tranvia ya en la parada.
                let corto = orilla.is_some_and(|a| a - x < 220.0);
                let tranvia =
                    p.def.plataformas.iter().any(|pl| {
                        pl.mueve.is_some() && pl.x < x + 40.0 && pl.x + pl.ancho > x + 200.0
                    });
                if corto || tranvia {
                    salto = 20;
                } else {
                    lado = 0;
                }
            }
            tumbado |= p.def.porteria.is_some() && !p.portero_vivo();
            let bits = lado | DISPARA | if salto > 0 { InputFrame::JUMP } else { 0 };
            salto = if salto > 0 { salto - 1 } else { 0 };
            p.step(pulsar(bits));
            if p.completado.is_some() {
                break;
            }
        }
        assert!(p.completado.is_some(), "atascada en x {}", p.jugadora.pos.x);
        assert!(tranvia, "ha cruzado la sudestada sin el tranvia");
        assert!(tumbado, "el guapo tenia que caer");
        assert_eq!(p.vidas, VIDAS, "ha caido al Riachuelo");
    }

    #[test]
    fn el_compadrito_amaga_se_lanza_frenando_y_se_planta() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                tipo: Tipo::Compadrito,
                x: 400.0,
                y: 0.0,
                rosa: false,
                portero: false,
            }]),
            1,
        );
        // Se despierta y arranca en el mismo tick, que es un tiempo (el 0).
        p.step(InputFrame::NONE);
        assert_eq!(p.enemigos[0].reloj, CICLO_COMPADRITO, "no ha arrancado");
        let mut xs = vec![p.enemigos[0].pos.x];
        for _ in 1..CICLO_COMPADRITO {
            p.step(InputFrame::NONE);
            xs.push(p.enemigos[0].pos.x);
        }
        let pasos: Vec<f32> = xs.windows(2).map(|w| w[0] - w[1]).collect();
        let (amago, resto) = pasos.split_at(AMAGO_COMPADRITO as usize - 1);
        let (estocada, plantado) = resto.split_at(ESTOCADA_COMPADRITO as usize);
        assert!(
            amago.iter().all(|&d| d == 0.0),
            "el amago es quieto: {amago:?}"
        );
        // Hacia ella, y cada paso mas corto que el anterior: frena.
        assert!(estocada[0] > 13.0, "sale a tope: {}", estocada[0]);
        for w in estocada.windows(2) {
            assert!(w[1] < w[0] && w[1] > 0.0, "no frena: {estocada:?}");
        }
        let largo: f32 = estocada.iter().sum();
        assert!(largo > 200.0, "la estocada se queda corta: {largo}");
        assert!(
            plantado.iter().all(|&d| d == 0.0),
            "no se planta: {plantado:?}"
        );
        assert_eq!(p.vidas, VIDAS, "no llega a tocarla desde 300");
    }

    #[test]
    fn la_rosa_frena_se_queda_colgada_y_cae() {
        let mut p = Paseo::new(calle(), 1);
        p.enemigos.push(Enemigo {
            tipo: Tipo::Rosa,
            pos: Vec2::new(700.0, 380.0),
            vida: 1,
            dir: -1.0,
            nacio: 0,
            rosa: false,
            golpe: 0,
            reloj: 0,
            lanzados: 0,
            base_y: 380.0,
            avance: 0.0,
            gesto: 0.0,
            vel: Vec2::new(-ROSA_V, 0.0),
            portero: false,
        });
        let mut antes = 700.0;
        let mut paso = f32::MAX;
        for _ in 0..VUELO_ROSA {
            p.step(InputFrame::NONE);
            let x = p.enemigos[0].pos.x;
            assert!(antes - x < paso && x < antes, "no frena");
            (paso, antes) = (antes - x, x);
        }
        assert!(700.0 - antes > 180.0, "vuela poco: {}", 700.0 - antes);
        let colgada = p.enemigos[0].pos;
        for _ in 0..COLGADA_ROSA {
            p.step(InputFrame::NONE);
            assert_eq!(p.enemigos[0].pos, colgada, "tenia que quedarse colgada");
        }
        correr(&mut p, 0, 20);
        assert!(p.enemigos[0].pos.y > colgada.y, "y luego cae");
        correr(&mut p, 0, 240);
        assert!(p.enemigos.is_empty(), "una rosa caida se va");
        assert_eq!(p.caidos, 0);
    }

    #[test]
    fn la_florista_tira_rosas_hacia_ti_y_una_de_cada_tres_es_rosa() {
        let mut def = con(vec![Aparicion {
            tipo: Tipo::Florista,
            x: 500.0,
            y: 360.0,
            rosa: false,
            portero: false,
        }]);
        def.plataformas = vec![Plataforma {
            x: 440.0,
            y: 360.0,
            ancho: 120.0,
            mueve: None,
        }];
        let mut p = Paseo::new(def, 1);
        let (mut vistas, mut rosas) = (0, 0);
        for _ in 0..(CADENCIA_FLORISTA * 7) {
            p.jugadora.iframes = 10;
            let antes = p.enemigos.first().map_or(0, |f| f.lanzados);
            p.step(InputFrame::NONE);
            if p.enemigos[0].lanzados > antes {
                let r = p.enemigos.last().expect("acaba de tirar");
                assert_eq!(r.tipo, Tipo::Rosa);
                assert!(r.vel.x < 0.0 && r.vel.y > 0.0, "hacia ella, abajo");
                vistas += 1;
                rosas += u32::from(r.rosa);
            }
        }
        assert!(vistas >= 6, "tira con cadencia: {vistas}");
        assert_eq!(rosas, vistas / 3);
    }

    #[test]
    fn la_gaviota_remonta_pica_frenando_y_se_queda_colgada() {
        let mut p = Paseo::new(uno(Tipo::Gaviota, 400.0, 250.0), 1);
        // Despierta y arranca en el tick 0, que es un tiempo.
        p.step(InputFrame::NONE);
        assert_eq!(p.enemigos[0].reloj, CICLO_GAVIOTA - 1, "no ha arrancado");
        let mut pos = vec![p.enemigos[0].pos];
        for _ in 1..CICLO_GAVIOTA {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            pos.push(p.enemigos[0].pos);
        }
        let pasos: Vec<f32> = pos.windows(2).map(|w| w[0].distance(w[1])).collect();
        let r = REMONTE_GAVIOTA as usize - 1;
        let (remonte, resto) = pasos.split_at(r);
        let (picado, colgada) = resto.split_at(PICADO_GAVIOTA as usize);
        // Ya estaba a su altura: el remonte es el aviso, sin moverse.
        assert!(remonte.iter().all(|&d| d == 0.0), "{remonte:?}");
        assert!(picado[0] > 15.0, "sale a tope: {}", picado[0]);
        for w in picado.windows(2) {
            assert!(w[1] < w[0] && w[1] > 0.0, "no frena: {picado:?}");
        }
        // Estaba a mas de `PICADO_MAX`: llega justo hasta ahi, hacia ella.
        let largo: f32 = picado.iter().sum();
        assert!((largo - PICADO_MAX).abs() < 1.0, "pica {largo}");
        assert!(colgada.iter().all(|&d| d == 0.0), "no se queda colgada");
        let g = p.enemigos[0].pos;
        assert!(g.y > 250.0 && g.x < 400.0, "ha picado hacia otro lado: {g}");
    }

    #[test]
    fn la_gaviota_rosa_se_parria() {
        let mut p = Paseo::new(
            con(vec![Aparicion {
                rosa: true,
                ..aparece(Tipo::Gaviota, 130.0)
            }]),
            1,
        );
        p.def.enemigos[0].y = 420.0;
        p.jugadora.iframes = 10;
        p.step(InputFrame::NONE);
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
        assert!(p.enemigos.is_empty());
        assert_eq!(p.vidas, VIDAS);
    }

    fn malevo(p: &Paseo) -> Option<&Enemigo> {
        p.enemigos.iter().find(|e| e.tipo == Tipo::Malevo)
    }

    #[test]
    fn el_poncho_del_malevo_para_lo_de_frente_y_no_la_espalda() {
        let mut def = uno(Tipo::Malevo, 400.0, 0.0);
        def.tiempo = 30;
        let mut p = Paseo::new(def, 1);
        let mut sacudido = false;
        for _ in 0..60 {
            p.jugadora.iframes = 10;
            p.step(pulsar(DISPARA));
            sacudido |= malevo(&p).is_some_and(|m| m.gesto > 0.0);
        }
        let m = malevo(&p).expect("el malevo sigue ahi");
        assert_eq!(m.vida, Tipo::Malevo.vida(), "de frente no se le hace nada");
        assert!(sacudido, "el poncho tenia que sacudirse");
        // A su espalda, plantada y mirandole: no se vuelve hasta el compas.
        p.jugadora.pos.x = 560.0;
        let espalda = InputFrame::FOCUS | InputFrame::LEFT | DISPARA;
        for _ in 0..30 {
            p.jugadora.iframes = 10;
            p.step(pulsar(espalda));
        }
        let m = malevo(&p).expect("en medio segundo no cae de espaldas");
        assert!(m.vida < Tipo::Malevo.vida() - 4, "vida {}", m.vida);
    }

    #[test]
    fn el_malevo_tira_en_el_tercer_tiempo_y_la_primera_es_de_parry() {
        let mut def = uno(Tipo::Malevo, 400.0, 0.0);
        def.tiempo = 30;
        let mut p = Paseo::new(def, 1);
        let mut tiradas = Vec::new();
        for _ in 0..(120 * 4) {
            p.jugadora.iframes = 10;
            let (tick, antes) = (p.tick, p.enemigos.len());
            p.step(InputFrame::NONE);
            if let Some(r) = p.enemigos.last().filter(|_| p.enemigos.len() > antes)
                && r.tipo == Tipo::Rosa
            {
                assert!(r.vel.x < 0.0, "hacia ella");
                tiradas.push((tick % 120, r.rosa));
            }
        }
        // Dos tiempos despues del uno del compas, una de parry y otra no.
        assert_eq!(
            tiradas,
            [(59, true), (59, false), (59, true), (59, false)],
            "{tiradas:?}"
        );
    }

    #[test]
    fn la_rosa_devuelta_tumba_al_malevo_de_frente() {
        let mut p = Paseo::new(uno(Tipo::Malevo, 400.0, 0.0), 1);
        p.step(InputFrame::NONE);
        p.proyectiles.push(Proyectil {
            pos: p.jugadora.pos + Vec2::new(20.0, -10.0),
            vel: Vec2::ZERO,
            rosa: true,
            forma: Forma::Plato,
        });
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
        let d = p
            .disparos
            .iter()
            .find(|d| d.rosa)
            .expect("sale la devuelta");
        assert!(d.vel.x > 0.0, "hacia el malevo");
        let mut cayo = false;
        for _ in 0..40 {
            p.step(InputFrame::NONE);
            cayo |= p.caidos > 0;
        }
        assert!(cayo && malevo(&p).is_none(), "la devuelta lo tumba");
    }

    /// Una calle cortada por la sudestada: el tranvia de orilla a orilla y un
    /// compadrito colgado del cable en medio.
    fn con_tranvia() -> PaseoDef {
        let mut def = uno(Tipo::Compadrito, 650.0, 300.0);
        def.suelo = vec![(0.0, 200.0), (1400.0, 3200.0)];
        def.plataformas = vec![Plataforma {
            x: 210.0,
            y: 380.0,
            ancho: 320.0,
            mueve: Some(Mueve::Vaiven {
                dx: 860.0,
                dy: 0.0,
                periodo: 600,
                fase: 0.0,
            }),
        }];
        def
    }

    #[test]
    fn el_compadrito_se_tira_del_cable_al_tranvia_y_viaja_en_el() {
        let mut p = Paseo::new(con_tranvia(), 1);
        // La jugadora, ya en el techo del tranvia.
        p.jugadora.pos = Vec2::new(300.0, 380.0 - PIES);
        let (mut cayo, mut viaja) = (false, Vec::new());
        for _ in 0..300 {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            let e = &p.enemigos[0];
            cayo |= e.vel.y > 0.0;
            if cayo && e.vel == Vec2::ZERO {
                assert_eq!(e.pos.y + e.tipo.alto(), 380.0, "no esta en el techo");
                viaja.push(e.pos.x);
            }
        }
        assert!(cayo, "no se ha soltado del cable");
        assert!(viaja.len() > 60, "no ha caido en el tranvia");
        // Plantado tras caer, y aun asi avanza: lo lleva el tranvia.
        let plantado = PLANTADO_COMPADRITO as usize - 2;
        assert!(viaja[plantado] - viaja[0] > 30.0, "{viaja:?}");
        assert_eq!(p.vidas, VIDAS, "ella tambien sigue en el tranvia");
        // Y no se baja: la estocada se clava en el borde del techo.
        let pl = p.def.plataformas[0];
        let x = p.enemigos[0].pos.x;
        assert!(x >= pl.x && x <= pl.x + pl.ancho, "se ha tirado: {x}");
    }

    #[test]
    fn el_colgado_no_se_suelta_sin_tranvia_debajo() {
        let mut def = con_tranvia();
        def.plataformas.clear();
        let mut p = Paseo::new(def, 1);
        p.jugadora.pos.x = 450.0;
        correr(&mut p, 0, 240);
        let e = &p.enemigos[0];
        assert_eq!(e.vel, Vec2::ZERO);
        assert_eq!(e.pos.y + e.tipo.alto(), 300.0, "sigue colgado");
    }

    /// Una calle con el guapo en su arena: de 1000 a 1960.
    fn con_guapo() -> PaseoDef {
        let mut def = con_portero();
        def.enemigos[0].tipo = Tipo::Guapo;
        def.tiempo = 30;
        def
    }

    /// Anda hasta que se cierra la arena.
    fn en_la_arena() -> Paseo {
        let mut p = Paseo::new(con_guapo(), 1);
        while p.cierre().is_none() {
            p.jugadora.iframes = 5;
            p.step(pulsar(DERECHA));
            let g = p.portero().map_or(1800.0, |g| g.pos.x);
            assert_eq!(g, 1800.0, "se mueve antes de cerrarse la arena");
            assert!(p.tick < 600, "la arena no se cierra");
        }
        p
    }

    /// Los pasos del guapo durante `n` ticks quieta, con lo que avanza en
    /// cada uno.
    fn pasos_del_guapo(p: &mut Paseo, n: usize) -> Vec<(Paso, f32)> {
        let mut pasos = Vec::new();
        for _ in 0..n {
            p.jugadora.iframes = 10;
            let g = p.portero().expect("el guapo");
            let (x, (paso, _)) = (g.pos.x, g.paso(30).expect("es el guapo"));
            p.step(InputFrame::NONE);
            let g = p.portero().expect("el guapo");
            pasos.push((paso, (g.pos.x - x).abs()));
        }
        pasos
    }

    #[test]
    fn el_guapo_amaga_y_se_lanza_frenando_en_seco() {
        let mut p = en_la_arena();
        let pasos = pasos_del_guapo(&mut p, 240);
        let estocada: Vec<f32> = pasos
            .iter()
            .filter(|(paso, _)| *paso == Paso::Estocada)
            .map(|(_, d)| *d)
            .collect();
        assert_eq!(estocada.len(), 30, "una estocada por compas de estocada");
        assert!(estocada[0] > 20.0, "sale a tope: {}", estocada[0]);
        for w in estocada.windows(2) {
            assert!(w[1] < w[0], "no frena: {estocada:?}");
        }
        let largo: f32 = estocada.iter().sum();
        assert!(largo > 300.0, "la estocada se queda corta: {largo}");
        // Todo lo demas, quieto: amaga, se planta y tira sin moverse.
        assert!(
            pasos
                .iter()
                .all(|(paso, d)| *paso == Paso::Estocada || *d == 0.0)
        );
        assert!(!pasos.iter().any(|(paso, _)| *paso == Paso::Finta));
        // Y tira tres rosas a la vez, la de abajo de parry.
        let rosas: Vec<&Enemigo> = p.enemigos.iter().filter(|e| e.tipo == Tipo::Rosa).collect();
        assert_eq!(rosas.len(), 3, "un abanico de tres");
        assert_eq!(rosas.iter().filter(|r| r.rosa).count(), 1);
    }

    #[test]
    fn el_guapo_cambia_de_jugadas_con_la_vida() {
        // Con dos tercios: finta antes de la estocada, corta, y dos abanicos
        // de cinco por compas de rosas.
        let mut p = en_la_arena();
        p.enemigos[0].vida = VIDA_GUAPO * 2 / 3 - 1;
        assert_eq!(p.enemigos[0].fase(), 1);
        let pasos = pasos_del_guapo(&mut p, 240);
        let finta: f32 = pasos
            .iter()
            .filter(|(paso, _)| *paso == Paso::Finta)
            .map(|(_, d)| *d)
            .sum();
        assert!(finta > 20.0 && finta < 80.0, "la finta: {finta}");
        pasos_del_guapo(&mut p, 30);
        let rosas = p.enemigos.iter().filter(|e| e.tipo == Tipo::Rosa).count();
        assert_eq!(rosas, 10, "dos abanicos de cinco");

        // Con un tercio: dos estocadas seguidas.
        let mut p = en_la_arena();
        p.enemigos[0].vida = VIDA_GUAPO / 3 - 1;
        assert_eq!(p.enemigos[0].fase(), 2);
        let pasos = pasos_del_guapo(&mut p, 120);
        let estocadas = pasos
            .iter()
            .filter(|(paso, _)| *paso == Paso::Estocada)
            .count();
        assert_eq!(estocadas, 60, "dos tiempos de estocada en un compas");
    }

    #[test]
    fn el_guapo_cae_a_tiros_y_abre_la_arena() {
        let mut p = en_la_arena();
        let antes = p.portero().expect("el guapo").vida;
        // Plantada y de cara a el, disparando.
        let fuego = InputFrame::FOCUS | DERECHA | DISPARA;
        for _ in 0..30 {
            p.jugadora.iframes = 10;
            p.step(pulsar(fuego));
        }
        let g = p.portero().expect("medio segundo no basta");
        assert!(g.vida < antes, "los disparos no le hacen nada");
        assert_eq!(p.cierre(), Some(1000.0), "con el en pie, cerrada");
        p.enemigos[0].vida = 1;
        let mut cayo = false;
        for _ in 0..120 {
            p.jugadora.iframes = 10;
            p.step(pulsar(fuego));
            cayo |= p.caidos > 0 && p.portero().is_none();
        }
        assert!(cayo, "el guapo tenia que caer");
        assert!(
            p.cierre().is_none() && p.ficha_visible(0),
            "la arena se abre"
        );
    }

    // --- Montmartre ---

    fn montmartre() -> PaseoDef {
        PaseoDef::de_serie()
            .into_iter()
            .find(|d| d.jefe == "El Cancan")
            .expect("falta Montmartre")
    }

    /// Una calle de pruebas con una escalera en medio.
    fn cuesta() -> PaseoDef {
        PaseoDef {
            escaleras: vec![Escalera {
                x: 400.0,
                ancho: 200.0,
                sube: 120.0,
            }],
            ..calle()
        }
    }

    fn aparece(tipo: Tipo, x: f32) -> Aparicion {
        Aparicion {
            tipo,
            x,
            y: 0.0,
            rosa: false,
            portero: false,
        }
    }

    #[test]
    fn la_escalera_se_sube_y_se_baja_andando_sin_despegarse() {
        // Andando, los pies van pegados a la rampa en los dos sentidos: ni se
        // queda en el aire al bajar ni choca con ella al subir.
        let mut p = Paseo::new(cuesta(), 1);
        for bits in [DERECHA, InputFrame::LEFT] {
            for _ in 0..150 {
                p.step(pulsar(bits));
                let x = p.jugadora.pos.x;
                assert!(p.jugadora.on_ground, "en el aire en x {x}");
                assert!((p.jugadora.pos.y + PIES - p.def.altura(x)).abs() < 0.01);
            }
            if bits == DERECHA {
                assert_eq!(p.jugadora.pos.y + PIES, SUELO_Y - 120.0, "no ha subido");
            }
        }
        assert_eq!(p.jugadora.pos.y + PIES, SUELO_Y, "no ha vuelto a bajar");
        assert_eq!(p.vidas, VIDAS);
    }

    #[test]
    fn la_corista_baja_la_escalera_y_su_volante_corre_a_ras_de_acera() {
        // La corista arriba de la escalera y la jugadora abajo, quieta. Tiene
        // que bajar pisando los peldanos, patear y mandarle un volante que
        // corre pegado a la acera y le da.
        let mut def = cuesta();
        def.enemigos = vec![aparece(Tipo::Corista, 700.0)];
        let mut p = Paseo::new(def, 1);
        p.jugadora.pos.x = 300.0;
        let mut volante = false;
        for _ in 0..600 {
            p.step(InputFrame::NONE);
            if let Some(e) = p.enemigos.first() {
                assert_eq!(
                    e.pos.y,
                    p.def.altura(e.pos.x) - e.tipo.alto(),
                    "flota o se hunde"
                );
            }
            for f in &p.fugaces {
                assert_eq!(f.clase, Clase::Volante);
                assert!(f.ttl <= VIDA_VOLANTE);
                assert_eq!(f.pos.y, p.def.altura(f.pos.x) - 10.0, "no va a ras");
                volante = true;
            }
            if p.events.player_died {
                break;
            }
        }
        assert!(volante, "no ha pateado nunca");
        assert_eq!(p.vidas, VIDAS - 1, "el volante tenia que llegarle");
    }

    #[test]
    fn la_pierna_de_la_corista_cuelga_parada_y_sube_en_la_patada() {
        let mut p = Paseo::new(con(vec![aparece(Tipo::Corista, 600.0)]), 1);
        correr(&mut p, 0, 1);
        let e = &mut p.enemigos[0];
        e.gesto = 0.0;
        let (cadera, colgada) = e.pierna().expect("una corista tiene pierna");
        assert!(colgada.y > cadera.y, "parada, la pierna cuelga");
        e.gesto = 1.0;
        let (cadera, arriba) = e.pierna().expect("una corista tiene pierna");
        assert!(arriba.y < cadera.y - PIERNA * 0.9, "la patada no sube");
    }

    #[test]
    fn el_corcho_revienta_en_burbujas_que_se_apagan_enseguida() {
        let mut p = Paseo::new(con(vec![aparece(Tipo::Botella, 450.0)]), 1);
        let mut burbujas_max = 0;
        let mut corcho = false;
        for _ in 0..240 {
            // Invulnerable: un golpe se lleva lo que toca y no se contaria.
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            corcho |= p.fugaces.iter().any(|f| f.clase == Clase::Corcho);
            let n = p
                .fugaces
                .iter()
                .filter(|f| f.clase == Clase::Burbuja)
                .count();
            burbujas_max = burbujas_max.max(n);
            assert!(
                p.fugaces.iter().all(|f| f.ttl <= 2.0),
                "nada vive mas de 2 s"
            );
        }
        assert!(corcho, "no ha descorchado");
        assert_eq!(burbujas_max, BURBUJAS, "el corcho no ha reventado");
        // Y se apagan: sin la botella, en un segundo y medio no queda nada.
        p.enemigos.clear();
        correr(&mut p, 0, 90);
        assert!(p.fugaces.is_empty());
    }

    #[test]
    fn un_corcho_rosa_se_parria() {
        let mut p = Paseo::new(calle(), 1);
        p.fugaces.push(Fugaz {
            clase: Clase::Corcho,
            pos: p.jugadora.pos + Vec2::new(20.0, -30.0),
            vel: Vec2::ZERO,
            ttl: 1.0,
            rosa: true,
        });
        p.step(pulsar(InputFrame::PARRY));
        assert_eq!(p.events.parried, 1);
        assert!(p.fugaces.is_empty(), "parriado no revienta");
    }

    #[test]
    fn el_pintor_deja_tres_charcos_que_se_secan() {
        // El pintor en un balcon y la jugadora debajo, invulnerable. Se mira
        // hasta antes de que caigan las gotas de la segunda sacudida.
        let mut def = calle();
        def.enemigos = vec![Aparicion {
            y: 330.0,
            ..aparece(Tipo::Pintor, 500.0)
        }];
        let mut p = Paseo::new(def, 1);
        let mut charcos = 0;
        for _ in 0..150 {
            p.jugadora.iframes = 10;
            p.step(InputFrame::NONE);
            let manchas: Vec<&Fugaz> = p
                .fugaces
                .iter()
                .filter(|f| f.clase == Clase::Mancha)
                .collect();
            for m in &manchas {
                assert_eq!(m.pos.y, SUELO_Y - 4.0, "el charco va en la acera");
                assert_eq!(m.vel, Vec2::ZERO);
            }
            charcos = charcos.max(manchas.len());
        }
        assert_eq!(charcos, 3, "un abanico son tres gotas");
        p.enemigos.clear();
        correr(&mut p, 0, (VIDA_MANCHA / DT) as usize + 2);
        assert!(p.fugaces.is_empty(), "los charcos no se secan");
    }

    #[test]
    fn el_confeti_brinca_y_revienta_en_papelillos_que_se_apagan() {
        // A la altura del pecho y la jugadora quieta e invulnerable: viene a
        // brincos, prende la mecha al tenerla cerca y revienta sin contar
        // como caido. El rosa suelta un papelillo rosa.
        for rosa in [false, true] {
            let mut p = Paseo::new(
                con(vec![Aparicion {
                    y: SUELO_Y - 36.0,
                    rosa,
                    ..aparece(Tipo::Confeti, 600.0)
                }]),
                1,
            );
            let (mut alto, mut bajo) = (f32::MAX, f32::MIN);
            let mut reventado = None;
            for t in 0..400 {
                p.jugadora.iframes = 10;
                p.step(InputFrame::NONE);
                assert_eq!(p.caidos, 0, "nadie lo ha tumbado");
                match p.enemigos.first() {
                    Some(e) => (alto, bajo) = (alto.min(e.pos.y), bajo.max(e.pos.y)),
                    None => {
                        reventado = Some(t);
                        break;
                    }
                }
            }
            assert!(reventado.is_some(), "no ha reventado");
            assert!(
                bajo - alto > BRINCO_CONFETI * 0.9,
                "no brinca: {alto} {bajo}"
            );
            let papelillos: Vec<&Fugaz> = p
                .fugaces
                .iter()
                .filter(|f| f.clase == Clase::Papelillo)
                .collect();
            assert_eq!(papelillos.len(), PAPELILLOS);
            assert_eq!(
                papelillos.iter().filter(|f| f.rosa).count(),
                usize::from(rosa)
            );
            assert!(papelillos.iter().all(|f| f.ttl < 2.0));
            // Revienta antes de llegar: a la distancia de la mecha.
            let x = papelillos[0].pos.x - p.jugadora.pos.x;
            assert!(x > 100.0 && x < MECHA_CONFETI_X, "revienta en {x}");
            correr(&mut p, 0, (VIDA_PAPELILLO / DT) as usize + 1);
            assert!(p.fugaces.is_empty(), "los papelillos no se apagan");
        }
    }

    #[test]
    fn el_apache_se_esfuma_y_sale_a_tu_otro_lado() {
        let mut def = con(vec![aparece(Tipo::Apache, 700.0)]);
        def.salida = 400.0;
        let mut p = Paseo::new(def, 1);
        // En el primer tiempo ya la tiene a tiro: humareda donde estaba, y el
        // a su izquierda, por el otro lado.
        p.step(InputFrame::NONE);
        let e = &p.enemigos[0];
        assert!(!e.presente(), "tenia que esfumarse");
        assert_eq!(e.pos.x, 400.0 - LADO_APACHE);
        assert_eq!(p.fugaces[0].clase, Clase::Humo);
        assert_eq!(p.fugaces[0].pos.x, 700.0);
        // Esfumado no se le dispara, y la humareda no hace dano.
        let pos = e.pos;
        p.disparos.push(Disparo {
            pos,
            vel: Vec2::ZERO,
            ttl: 0.5,
            rosa: false,
        });
        p.fugaces[0].pos = p.jugadora.pos;
        p.step(InputFrame::NONE);
        assert_eq!(
            p.enemigos[0].vida,
            Tipo::Apache.vida(),
            "le han dado esfumado"
        );
        assert_eq!(p.vidas, VIDAS, "la humareda pega");
        p.disparos.clear();
        // Sale al compas siguiente, de cara a ella, y si no se mueve el
        // navajazo le llega.
        correr(&mut p, 0, OCULTO_APACHE as usize - 1);
        let e = &p.enemigos[0];
        assert!(e.presente() && e.dir == 1.0, "no ha salido de cara");
        let mut dado = false;
        for _ in 0..GOLPE_APACHE {
            p.step(InputFrame::NONE);
            dado |= p.events.player_died;
        }
        assert!(dado, "el navajazo no llega a quien se queda quieta");

        // Y alejandose al ver la sombra, no.
        let mut def = con(vec![aparece(Tipo::Apache, 700.0)]);
        def.salida = 400.0;
        let mut p = Paseo::new(def, 1);
        p.step(InputFrame::NONE);
        correr(&mut p, DERECHA, CICLO_APACHE as usize);
        assert_eq!(p.vidas, VIDAS, "no se esquiva corriendo");
    }

    #[test]
    fn la_tabla_del_molino_lleva_por_arriba_al_otro_lado() {
        // Un vacio de 400 con el molino en medio, y la jugadora sobre la
        // tabla de la izquierda, a ras de acera.
        let mut def = calle();
        def.suelo = vec![(0.0, 400.0), (840.0, 3200.0)];
        def.plataformas = [0.0, 0.25, 0.5, 0.75]
            .map(|fase| Plataforma {
                x: 580.0,
                y: SUELO_Y,
                ancho: 80.0,
                mueve: Some(Mueve::Gira {
                    radio: 150.0,
                    periodo: 480,
                    fase,
                }),
            })
            .to_vec();
        def.salida = 470.0;
        let mut p = Paseo::new(def, 1);
        // Se la suelta un poco por encima, que la tabla ya esta subiendo.
        p.jugadora.pos.y -= 6.0;
        p.jugadora.on_ground = false;
        correr(&mut p, 0, 8);
        // Media vuelta sin tocar nada: sube, pasa por arriba y baja.
        let mut cima = f32::MAX;
        for _ in 8..240 {
            p.step(InputFrame::NONE);
            assert!(p.jugadora.on_ground, "se ha caido del aspa");
            cima = cima.min(p.jugadora.pos.y + PIES);
        }
        assert!((cima - (SUELO_Y - 150.0)).abs() < 2.0, "cima {cima}");
        assert!(
            (p.jugadora.pos.x - 770.0).abs() < 3.0,
            "x {}",
            p.jugadora.pos.x
        );
        // Y de un salto, a la otra acera.
        correr(&mut p, DERECHA | InputFrame::JUMP, 60);
        assert!(p.jugadora.on_ground && p.jugadora.pos.x > 840.0);
        assert_eq!(p.vidas, VIDAS);
    }

    /// Una pista con Monsieur Loyal: arena en 1000 y su ficha.
    fn con_loyal() -> PaseoDef {
        let mut def = con(vec![Aparicion {
            portero: true,
            ..aparece(Tipo::Loyal, 1700.0)
        }]);
        def.tiempo = 24;
        def.porteria = Some(Porteria {
            desde: 1000.0,
            ficha: Some(0),
        });
        def.fichas = vec![(1500.0, 435.0)];
        def.puerta = 2500.0;
        def
    }

    /// Anda hasta que se cierra la pista, invulnerable.
    fn hasta_la_pista(p: &mut Paseo) {
        while p.cierre().is_none() {
            p.jugadora.iframes = 5;
            p.step(pulsar(DERECHA));
            assert!(
                !p.fugaces.iter().any(|f| f.clase == Clase::Paloma),
                "empieza antes de cerrar"
            );
            assert!(p.tick < 600, "la pista no se cierra");
        }
    }

    #[test]
    fn monsieur_loyal_cambia_de_numeros_con_la_vida() {
        // Quieta e invulnerable en la pista, una vuelta entera de numeros en
        // cada fase: cuantas palomas por chistera, y si saca el baston.
        for (fase, vida) in [(0, 192), (1, 96), (2, 48)] {
            let mut p = Paseo::new(con_loyal(), 1);
            hasta_la_pista(&mut p);
            p.enemigos[0].vida = vida;
            assert_eq!(p.enemigos[0].fase(), fase);
            let (mut barridos, mut aros, mut chisteras) = (0, 0, Vec::new());
            for _ in 0..(6 * NUMERO_LOYAL[fase]) {
                p.jugadora.iframes = 5;
                p.step(InputFrame::NONE);
                let recien = |clase: Clase, vida: f32| {
                    p.fugaces
                        .iter()
                        .filter(|f| f.clase == clase && (f.ttl - (vida - DT)).abs() < 1e-4)
                        .collect::<Vec<_>>()
                };
                let palomas = recien(Clase::Paloma, VIDA_PALOMA);
                if !palomas.is_empty() {
                    assert_eq!(palomas.iter().filter(|f| f.rosa).count(), 1, "una rosa");
                    chisteras.push(palomas.len());
                }
                aros += recien(Clase::Aro, VIDA_ARO).len();
                barridos += recien(Clase::Barrido, VIDA_BARRIDO).len();
                assert!(p.fugaces.iter().all(|f| f.ttl < 2.0), "nada vive 2 s");
            }
            assert!(!chisteras.is_empty() && aros > 0, "fase {fase}");
            assert!(chisteras.iter().all(|&n| n == 3 + fase), "{chisteras:?}");
            assert_eq!(barridos > 0, fase > 0, "el baston, solo tras la primera");
        }
    }

    #[test]
    fn tumbar_a_monsieur_loyal_abre_la_pista() {
        // Plantada en la pista y disparando de frente con la aguja: cae en
        // el tiempo de un portero, y al caer se abre la pista y sale la ficha.
        let mut p = Paseo::new(con_loyal(), 1);
        hasta_la_pista(&mut p);
        assert_eq!(p.portero().map(|e| e.vida_max()), Some(192));
        let desde = p.tick;
        while p.portero_vivo() {
            p.jugadora.iframes = 5;
            p.step(pulsar(DISPARA));
            let e = p.portero();
            assert!(
                e.is_none_or(|e| (1060.0..=1900.0).contains(&e.pos.x)),
                "se sale"
            );
            assert!(p.tick - desde < 60 * 45, "no cae");
        }
        let segundos = (p.tick - desde) as f32 * DT;
        assert!((10.0..20.0).contains(&segundos), "dura {segundos} s");
        assert!(p.cierre().is_none() && p.ficha_visible(0));
        correr(&mut p, DERECHA, 300);
        assert!(p.completado.is_some());
    }

    #[test]
    fn montmartre_se_puede_subir_entera() {
        // El bot de Viena en la cuesta: corre, dispara y salta ante cada foso.
        // Las escaleras se suben andando. Dos cosas mas: ante el vacio del
        // molino espera a que asome una tabla, salta a ella, se deja llevar y
        // salta a la otra acera; y en la pista se vuelve hacia Monsieur Loyal
        // y le dispara de frente hasta tumbarlo.
        let mut p = Paseo::new(montmartre(), 3);
        let (mut salto, mut tabla) = (0, None);
        for _ in 0..(60 * 150) {
            p.jugadora.iframes = 5;
            let j = &p.jugadora;
            let (x, pies) = (j.pos.x, j.pos.y + PIES);
            let en_tabla = j.on_ground
                && p.sobre
                    .is_some_and(|i| p.def.plataformas[i].mueve.is_some());
            // Un vacio que no se salta: el siguiente tramo, lejos.
            let vacio = !p.def.hay_suelo(x + 24.0)
                && p.def
                    .suelo
                    .iter()
                    .find(|&&(a, _)| a > x)
                    .is_some_and(|&(a, _)| a - x > 250.0);
            let mut lado = DERECHA;
            if j.on_ground {
                tabla = None;
            }
            if en_tabla {
                // Quieta en la tabla hasta tener la otra acera a tiro.
                lado = 0;
                let x2 = x + 120.0;
                if p.def.hay_suelo(x2) && (p.def.altura(x2) - pies).abs() < 40.0 {
                    (salto, lado) = (22, DERECHA);
                }
            } else if j.on_ground && vacio {
                // Al borde, hasta que una tabla suba por delante a ras de pie.
                lado = 0;
                tabla = p.def.plataformas.iter().position(|pl| {
                    pl.mueve.is_some()
                        && pl.x > x + 30.0
                        && pl.x < x + 110.0
                        && (pl.y - pies).abs() < 30.0
                });
                if tabla.is_some() {
                    (salto, lado) = (22, DERECHA);
                }
            } else if j.on_ground && !p.def.hay_suelo(x + 24.0) {
                salto = 20;
            }
            if let Some(pl) = tabla.map(|i| p.def.plataformas[i]) {
                // En el aire hacia la tabla: a su centro, no mas alla.
                let dx = pl.x + pl.ancho * 0.5 - x;
                lado = if dx > 6.0 { DERECHA } else { 0 };
            }
            if let Some(e) = p.portero().filter(|_| p.cierre().is_some()) {
                lado = if e.pos.x < x {
                    InputFrame::LEFT
                } else {
                    DERECHA
                };
            }
            let bits = lado | DISPARA | if salto > 0 { InputFrame::JUMP } else { 0 };
            salto = if salto > 0 { salto - 1 } else { 0 };
            p.step(pulsar(bits));
            if p.completado.is_some() {
                break;
            }
        }
        assert!(p.completado.is_some(), "atascada en x {}", p.jugadora.pos.x);
        assert_eq!(p.vidas, VIDAS, "no ha caido a ningun foso");
        assert!(p.jugadora.pos.y < SUELO_Y - 400.0, "no ha llegado arriba");
        assert!(
            !p.portero_vivo() && p.ficha_visible(5),
            "Loyal sigue en pie"
        );
    }

    // --- Las fichas y el equipo ---

    use crate::equipo::Amuleto;

    fn con_fichas(fichas: Vec<(f32, f32)>) -> PaseoDef {
        PaseoDef { fichas, ..calle() }
    }

    fn equipado(def: PaseoDef, tiro: Tiro, amuleto: Amuleto) -> Paseo {
        Paseo::con_equipo(def, 1, Equipo { tiro, amuleto })
    }

    #[test]
    fn las_fichas_de_serie_estan_donde_se_puede_llegar() {
        let mut total = 0;
        for def in PaseoDef::de_serie() {
            let n = def.fichas.len();
            assert!((3..=6).contains(&n), "{}: {n} fichas", def.nombre);
            // La del portero es de propina: no cuenta para la modista.
            total += n - usize::from(def.porteria.and_then(|po| po.ficha).is_some());
            for &(x, y) in &def.fichas {
                assert!(
                    x > def.salida && x < def.puerta,
                    "{}: ficha en x {x}",
                    def.nombre
                );
                // Por encima de donde se da por caida: si no, cogerla mata.
                assert!(
                    y < def.altura(x) + CAIDA - 60.0,
                    "{}: ficha hundida en {x}",
                    def.nombre
                );
            }
        }
        // Las de los porteros van aparte: sobran para la modista, no faltan.
        assert!(
            total >= 16,
            "los precios de la modista cuentan con 16: {total}"
        );
    }

    #[test]
    fn una_ficha_se_coge_al_pasar_y_una_sola_vez() {
        let mut p = Paseo::new(
            con_fichas(vec![(400.0, SUELO_Y - PIES - 5.0), (400.0, 100.0)]),
            1,
        );
        let mut cogidas = 0;
        for _ in 0..120 {
            p.step(pulsar(DERECHA));
            cogidas += p.events.fichas;
        }
        assert!(p.jugadora.pos.x > 450.0, "ha pasado de largo");
        assert_eq!(cogidas, 1, "la de la acera, una vez; la del cielo, no");
        assert_eq!(p.fichas, vec![true, false]);
        assert_eq!(p.fichas_cogidas(), 1);
    }

    #[test]
    fn una_ficha_de_otra_vez_no_esta_en_la_calle() {
        // La app marca las que ya se tienen antes de empezar.
        let mut p = Paseo::new(con_fichas(vec![(400.0, SUELO_Y - PIES)]), 1);
        p.fichas[0] = true;
        let mut cogidas = 0;
        for _ in 0..120 {
            p.step(pulsar(DERECHA));
            cogidas += p.events.fichas;
        }
        assert_eq!(cogidas, 0);
    }

    #[test]
    fn el_bot_coge_la_ficha_del_camino_en_cada_paseo() {
        // La primera ficha que esta a la altura del cuerpo sobre la acera es
        // la "del camino": andar la calle tiene que darla sin buscarla.
        for def in PaseoDef::de_serie() {
            let camino = def
                .fichas
                .iter()
                .position(|&(x, y)| (def.altura(x) - PIES - y).abs() < 10.0)
                .unwrap_or_else(|| panic!("{} no tiene ficha en el camino", def.nombre));
            let nombre = def.nombre.clone();
            let p = bot_anda(def);
            assert!(
                p.fichas[camino],
                "{nombre}: el bot no ha cogido la del camino"
            );
        }
    }

    #[test]
    fn recargar_no_devuelve_las_fichas_a_la_calle() {
        let mut p = Paseo::new(con_fichas(vec![(400.0, SUELO_Y - PIES)]), 1);
        correr(&mut p, DERECHA, 120);
        assert_eq!(p.fichas, vec![true]);
        let def = p.def.clone();
        p.recargar(def);
        assert_eq!(p.fichas, vec![true]);
    }

    #[test]
    fn el_abanico_sale_de_tres_en_tres_abierto() {
        let mut p = equipado(calle(), Tiro::Abanico, Amuleto::Ninguno);
        p.step(pulsar(DISPARA));
        assert_eq!(p.disparos.len(), 3);
        let mut ys: Vec<f32> = p.disparos.iter().map(|d| d.vel.y).collect();
        ys.sort_by(f32::total_cmp);
        assert!(ys[0] < 0.0 && ys[1] == 0.0 && ys[2] > 0.0, "{ys:?}");
        assert!(
            p.disparos.iter().all(|d| d.vel.x > 0.0),
            "todos hacia delante"
        );
    }

    /// Ticks disparando hasta tumbar a un camarero (8 de vida) plantado a `x`,
    /// o `None` si no cae en dos segundos.
    fn ticks_para_tumbar(tiro: Tiro, x: f32) -> Option<usize> {
        let mut p = equipado(uno(Tipo::Camarero, x, 0.0), tiro, Amuleto::Ninguno);
        for t in 0..120 {
            p.jugadora.iframes = 5;
            p.step(pulsar(DISPARA));
            if p.caidos > 0 {
                return Some(t);
            }
        }
        None
    }

    #[test]
    fn la_castanuela_tumba_antes_pero_de_cerca() {
        let aguja = ticks_para_tumbar(Tiro::Aguja, 320.0).expect("la aguja tumba");
        let castanuela = ticks_para_tumbar(Tiro::Castanuela, 320.0).expect("de cerca tumba");
        assert!(castanuela < aguja, "{castanuela} vs {aguja}");
        assert!(ticks_para_tumbar(Tiro::Aguja, 600.0).is_some());
        assert_eq!(
            ticks_para_tumbar(Tiro::Castanuela, 600.0),
            None,
            "de lejos no llega"
        );
    }

    #[test]
    fn la_serpentina_busca_a_lo_que_vuela_y_dispara_la_mitad() {
        let canotier = uno(Tipo::Canotier, 450.0, 330.0);
        for (tiro, busca) in [(Tiro::Aguja, false), (Tiro::Serpentina, true)] {
            let mut p = equipado(canotier.clone(), tiro, Amuleto::Ninguno);
            let mut disparos = 0;
            for _ in 0..16 {
                p.step(pulsar(DISPARA));
                disparos += u32::from(p.events.player_shot);
            }
            assert_eq!(p.disparos.iter().any(|d| d.vel.y < 0.0), busca, "{tiro:?}");
            assert_eq!(disparos, if busca { 2 } else { 4 }, "{tiro:?}");
        }
    }

    #[test]
    fn el_relicario_y_el_guante_en_la_calle() {
        assert_eq!(
            equipado(calle(), Tiro::Aguja, Amuleto::Relicario).vidas,
            VIDAS + 1
        );
        for (amuleto, ticks) in [(Amuleto::Ninguno, 7), (Amuleto::Guante, 12)] {
            let mut p = equipado(calle(), Tiro::Aguja, amuleto);
            p.step(pulsar(InputFrame::PARRY));
            let mut abierto = 0;
            while p.jugadora.is_parrying() {
                abierto += 1;
                p.step(InputFrame::NONE);
            }
            assert_eq!(abierto, ticks, "{amuleto:?}");
        }
    }

    #[test]
    fn el_corse_estrecha_el_cuerpo_que_recibe() {
        // Una perla (radio 7) a 16 del centro: dentro de 10 + 7, fuera de 6 + 7.
        for (amuleto, golpe) in [(Amuleto::Ninguno, true), (Amuleto::Corse, false)] {
            let mut p = equipado(calle(), Tiro::Aguja, amuleto);
            p.proyectiles.push(Proyectil {
                pos: p.jugadora.pos + Vec2::new(16.0, 0.0),
                vel: Vec2::ZERO,
                rosa: false,
                forma: Forma::Perla,
            });
            p.step(InputFrame::NONE);
            assert_eq!(p.events.player_died, golpe, "{amuleto:?}");
        }
    }

    /// Una calle cortada por un foso largo, con una plataforma que va y viene
    /// de un lado al otro: el fiacre. Se sale ya encima, sobre el foso.
    fn con_fiacre() -> PaseoDef {
        let mut def = calle();
        def.suelo = vec![(0.0, 250.0), (1000.0, 3200.0)];
        def.plataformas = vec![Plataforma {
            x: 300.0,
            y: 470.0,
            ancho: 120.0,
            mueve: Some(Mueve::Vaiven {
                dx: 640.0,
                dy: 0.0,
                periodo: 240,
                fase: 0.0,
            }),
        }];
        def.salida = 350.0;
        def
    }

    #[test]
    fn la_plataforma_que_se_mueve_lleva_a_quien_la_pisa() {
        let mut p = Paseo::new(con_fiacre(), 1);
        // Media vuelta sin tocar nada: el fiacre cruza el foso con ella encima.
        for _ in 0..120 {
            p.step(InputFrame::NONE);
            assert!(p.jugadora.on_ground, "se ha caido del fiacre");
        }
        assert!(
            (p.jugadora.pos.x - 990.0).abs() < 2.0,
            "x {}",
            p.jugadora.pos.x
        );
        assert_eq!(p.vidas, VIDAS);
        // Y se baja andando al otro lado.
        correr(&mut p, DERECHA, 30);
        assert!(p.jugadora.pos.x > 1000.0 && p.jugadora.on_ground);
    }

    #[test]
    fn la_plataforma_sale_del_tick_y_no_de_la_historia() {
        let mut a = Paseo::new(con_fiacre(), 1);
        correr(&mut a, DERECHA | InputFrame::JUMP, 77);
        let b = Paseo::new(con_fiacre(), 9);
        let t = a.tick;
        let esperada = 300.0
            + Mueve::Vaiven {
                dx: 640.0,
                dy: 0.0,
                periodo: 240,
                fase: 0.0,
            }
            .desvio(t)
            .x;
        assert!((a.def.plataformas[0].x - esperada).abs() < 1e-3);
        assert_eq!(
            b.def.plataformas[0].x, 300.0,
            "en el tick 0, donde dice el RON"
        );
    }

    #[test]
    fn la_cabina_de_la_noria_da_la_vuelta_sin_volcar() {
        let m = Mueve::Gira {
            radio: 100.0,
            periodo: 400,
            fase: 0.0,
        };
        assert!((m.desvio(0) - Vec2::new(100.0, 0.0)).length() < 1e-3);
        assert!((m.desvio(100) - Vec2::new(0.0, 100.0)).length() < 1e-2);
        assert!((m.desvio(400) - m.desvio(0)).length() < 1e-3);
    }

    /// Una calle con portero: arena en 1000, el portero dentro y su ficha.
    fn con_portero() -> PaseoDef {
        let mut def = con(vec![Aparicion {
            portero: true,
            ..aparece(Tipo::Camarero, 1800.0)
        }]);
        def.porteria = Some(Porteria {
            desde: 1000.0,
            ficha: Some(0),
        });
        def.fichas = vec![(1500.0, 435.0)];
        def.puerta = 2500.0;
        def
    }

    #[test]
    fn al_portero_no_se_le_da_hasta_que_se_cierra_la_arena() {
        // Quieta a la entrada, con el portero a tiro pero la arena aun
        // abierta: los disparos le pasan y el super no le toca.
        let mut p = Paseo::new(con_portero(), 1);
        p.a_la_porteria();
        // Hasta tenerlo despierto y a tiro, sin llegar a cerrar la arena.
        while p.jugadora.pos.x < 1300.0 {
            p.step(pulsar(DERECHA));
        }
        p.jugadora.meter = 1.0;
        for _ in 0..240 {
            p.jugadora.iframes = 5;
            p.step(pulsar(DISPARA | InputFrame::SUPER));
        }
        assert!(p.cierre().is_none(), "sin andar no se cierra");
        let e = p
            .portero()
            .expect("no esta: o no desperto o se le tumbo de lejos");
        assert_eq!(e.vida, e.vida_max(), "se le ha hecho dano de lejos");
    }

    #[test]
    fn la_tecla_de_pruebas_lleva_a_la_puerta_del_portero() {
        let mut p = Paseo::new(con_portero(), 1);
        p.a_la_porteria();
        assert!(p.jugadora.pos.x > 1000.0 && p.portero_vivo());
        for _ in 0..90 {
            p.jugadora.iframes = 5;
            p.step(pulsar(DERECHA));
        }
        assert_eq!(p.cierre(), Some(1000.0), "y la arena se cierra al andar");
    }

    #[test]
    fn la_arena_del_portero_no_deja_salir_hasta_tumbarlo() {
        let mut p = Paseo::new(con_portero(), 1);
        while p.cierre().is_none() {
            p.jugadora.iframes = 5;
            p.step(pulsar(DERECHA));
            assert!(p.tick < 600, "la arena no se cierra");
        }
        // La camara llega justo al borde: la jugadora va a su encuadre.
        let x = p.jugadora.pos.x;
        assert!((x - (1000.0 + VISTA_W * ENCUADRE)).abs() < 10.0, "x {x}");
        // Ni hacia delante ni hacia atras, y su ficha aun no esta.
        for bits in [DERECHA, InputFrame::LEFT] {
            for _ in 0..240 {
                p.jugadora.iframes = 5;
                p.step(pulsar(bits));
                let x = p.jugadora.pos.x;
                assert!((1000.0..=1000.0 + VISTA_W).contains(&x), "x {x}");
            }
        }
        assert!(!p.ficha_visible(0), "la ficha del portero aun no sale");
        assert!(p.portero().is_some());

        // Tumbado, se abre por delante pero no por detras.
        p.enemigos.iter_mut().for_each(|e| e.vida = 0);
        p.step(InputFrame::NONE);
        assert!(p.cierre().is_none() && p.ficha_visible(0));
        assert!(p.events.boss_down, "el portero cae como un jefe");
        correr(&mut p, InputFrame::LEFT, 120);
        assert!(p.jugadora.pos.x >= 1000.0, "lo de detras no vuelve");
        while !p.terminado() {
            p.jugadora.iframes = 5;
            p.step(pulsar(DERECHA));
            assert!(p.tick < 2000, "no llega a la puerta");
        }
        assert!(p.completado.is_some());
    }
}
