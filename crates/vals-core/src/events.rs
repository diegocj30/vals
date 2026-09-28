//! Lo que ha pasado en un tick.
//!
//! El core no sabe que existe el audio, y no va a saberlo. Pero **si** sabe lo
//! que ha ocurrido en la simulacion, y decirlo explicitamente sale mucho mejor
//! que obligar a la capa de arriba a deducirlo comparando contadores entre
//! frames: comparando se pierden los eventos que se cancelan —dos parries en el
//! mismo frame, o morir y reaparecer— y hay que replicar reglas del juego fuera
//! del juego.
//!
//! Se rellena durante `World::step` y se vacia al principio del siguiente. **No
//! entra en el hash de estado**: es informacion derivada de lo que ya se
//! hashea, no estado propio.

/// Sucesos de un tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    pub player_shot: bool,
    /// Balas neutralizadas con el parry.
    pub parried: u32,
    /// Balas rozadas.
    pub grazed: u32,
    /// El jefe ha recibido dano.
    pub boss_hit: bool,
    pub phase_changed: bool,
    /// Ha caido un jefe.
    pub boss_down: bool,
    pub player_died: bool,
    pub super_fired: bool,
    pub victory: bool,
    pub defeat: bool,
    /// Fichas cogidas en la calle. En el combate no hay.
    pub fichas: u32,
}

impl Events {
    /// Junta los sucesos de otro tick con estos.
    ///
    /// La capa de presentacion va a un ritmo distinto que la simulacion —puede
    /// haber varios ticks en un frame—, asi que necesita acumular antes de
    /// reaccionar.
    pub fn merge(&mut self, o: &Events) {
        self.player_shot |= o.player_shot;
        self.parried += o.parried;
        self.grazed += o.grazed;
        self.boss_hit |= o.boss_hit;
        self.phase_changed |= o.phase_changed;
        self.boss_down |= o.boss_down;
        self.player_died |= o.player_died;
        self.super_fired |= o.super_fired;
        self.victory |= o.victory;
        self.defeat |= o.defeat;
        self.fichas += o.fichas;
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_evento_vacio_es_vacio() {
        assert!(Events::default().is_empty());
        assert!(
            !Events {
                player_shot: true,
                ..Default::default()
            }
            .is_empty()
        );
    }

    #[test]
    fn merge_suma_las_cuentas_y_junta_las_banderas() {
        let mut a = Events {
            parried: 2,
            grazed: 5,
            player_shot: true,
            ..Default::default()
        };
        a.merge(&Events {
            parried: 3,
            grazed: 1,
            player_died: true,
            ..Default::default()
        });
        assert_eq!(a.parried, 5);
        assert_eq!(a.grazed, 6);
        assert!(a.player_shot);
        assert!(a.player_died);
        assert!(!a.victory);
    }
}
