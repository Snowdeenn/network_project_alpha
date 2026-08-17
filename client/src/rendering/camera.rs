use utils::protocol::{EntityKind, StateSnapshot};

// =========================================================================
// CameraShake — système trauma-based
//
// Le trauma est une valeur [0..1] qui représente l'intensité du choc.
// Le shake appliqué est trauma² (falloff quadratique) — l'effet décroît
// rapidement au début puis s'estompe doucement, ce qui est plus naturel
// qu'un falloff linéaire.
//
// Utilisation :
//   shake.add_trauma(0.6);          // sur un impact fort
//   shake.add_trauma(0.3);          // sur un impact léger
//   shake.update(dt);               // chaque frame
//   let offset = shake.offset();    // appliqué à cam.target
// =========================================================================

pub struct CameraShake {
    trauma: f32,
    /// Vitesse de décroissance du trauma par seconde
    decay: f32,
    /// Amplitude max du déplacement en pixels monde
    max_offset: f32,
    /// Seed interne pour la variation du bruit
    time: f32,
}

impl CameraShake {
    pub fn new(decay: f32, max_offset: f32) -> Self {
        Self {
            trauma: 0.0,
            decay,
            max_offset,
            time: 0.0,
        }
    }

    /// Ajoute du trauma — clampé à 1.0.
    /// Plusieurs impacts s'accumulent jusqu'à saturation.
    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    pub fn update(&mut self, dt: f32) {
        self.trauma = (self.trauma - self.decay * dt).max(0.0);
        self.time += dt;
    }

    /// Offset à appliquer sur `cam.target` ou `cam.offset`.
    /// Retourne `Vector2::zero()` quand le trauma est épuisé.
    pub fn offset(&self) -> utils::math::Vec2 {
        if self.trauma <= 0.0 {
            return utils::math::Vec2::zero();
        }

        let shake = self.trauma * self.trauma; // falloff quadratique

        // Bruit pseudo-aléatoire basé sur le temps — deux fréquences distinctes
        // pour X et Y pour éviter un mouvement directionnel
        let dx = pseudo_noise(self.time * 13.7) * self.max_offset * shake;
        let dy = pseudo_noise(self.time * 11.3 + 42.0) * self.max_offset * shake;

        utils::math::Vec2::new(dx, dy)
    }

    pub fn is_active(&self) -> bool {
        self.trauma > 0.0
    }
}

impl Default for CameraShake {
    fn default() -> Self {
        Self::new(
            1.8,  // decay — trauma épuisé en ~0.5s à pleine intensité
            18.0, // max_offset — pixels monde au zoom actuel
        )
    }
}

// Bruit pseudo-aléatoire déterministe dans [-1, 1]
// Basé sur une fonction sinus à haute fréquence — pas de dépendance externe
fn pseudo_noise(t: f32) -> f32 {
    (t.sin() * 43758.545).fract() * 2.0 - 1.0
}

pub struct Camera {
    pos: utils::math::Vec2,
    view: utils::math::Mat4,
    pub shake: CameraShake,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: utils::math::Vec2::zero(),
            view: utils::math::Mat4::identity(),
            shake: CameraShake::default(),
        }
    }
}

impl Camera {
    pub fn get_view_proj(&self, screen_w: f32, screen_h: f32) -> utils::math::Mat4 {
        let proj = utils::math::Mat4::orthographic_wgpu(0.0, screen_w, screen_h, 0.0, -1.0, 1.0);
        let view = utils::math::Mat4::translation(
            -self.pos.x + (screen_w * 0.5) + self.shake.offset().x,
            -self.pos.y + (screen_h * 0.5) + self.shake.offset().y,
            0.0,
        );
        proj.multiply(view)
    }
    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.pos = utils::math::Vec2::new(x, y);
        self.view = utils::math::Mat4::translation(-x, -y, 0.0);
    }
    pub fn pos(&self) -> utils::math::Vec2 {
        self.pos
    }
}
// =========================================================================
// Update caméra
// =========================================================================

pub fn update(cam: &mut Camera, prev: Option<&StateSnapshot>, current: &StateSnapshot, t: f32) {
    let curr_player = current
        .entities
        .iter()
        .find(|e| matches!(e.entity_kind, EntityKind::Player));

    let prev_player = prev.and_then(|p| {
        p.entities
            .iter()
            .find(|e| matches!(e.entity_kind, EntityKind::Player))
    });

    if let Some(curr) = curr_player {
        let prev_pos = prev_player.map(|p| p.position).unwrap_or(curr.position);

        let base_target = utils::math::Vec2::new(
            utils::math::lerp(prev_pos[0], curr.position[0], t),
            utils::math::lerp(prev_pos[1], curr.position[1], t),
        );
        cam.set_pos(base_target.x, base_target.y);
        cam.shake.update(t);
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trauma_clamps_at_one() {
        let mut shake = CameraShake::default();
        shake.add_trauma(0.8);
        shake.add_trauma(0.8); // total > 1.0
        assert_eq!(shake.trauma, 1.0);
    }

    #[test]
    fn trauma_decays_to_zero() {
        let mut shake = CameraShake::default();
        shake.add_trauma(1.0);
        shake.update(10.0); // largement suffisant pour décroître
        assert_eq!(shake.trauma, 0.0);
    }

    #[test]
    fn no_offset_without_trauma() {
        let shake = CameraShake::default();
        let offset = shake.offset();
        assert_eq!(offset.x, 0.0);
        assert_eq!(offset.y, 0.0);
    }

    #[test]
    fn offset_within_max_bounds() {
        let mut shake = CameraShake::default();
        shake.add_trauma(1.0);
        // Teste sur plusieurs frames simulées
        for i in 0..100 {
            shake.time = i as f32 * 0.016;
            let offset = shake.offset();
            assert!(offset.x.abs() <= shake.max_offset);
            assert!(offset.y.abs() <= shake.max_offset);
        }
    }

    #[test]
    fn is_active_while_trauma_remains() {
        let mut shake = CameraShake::default();
        shake.add_trauma(0.5);
        assert!(shake.is_active());
        shake.update(10.0);
        assert!(!shake.is_active());
    }
}
