use std::collections::HashMap;
use utils::math::Vec2;

pub(crate) const MAX_SLASHES: usize = 16;
pub(crate) const MAX_FLASHES: usize = 32;
pub(crate) const MAX_DASH_GHOSTS: usize = 16;
pub(crate) const TRAIL_POINTS: usize = 12;

/// Arc DrawRing orienté selon l'AttackBox du joueur.
/// `start_angle` et `end_angle` sont en degrés, dans le repère monde.
#[derive(Clone, Copy, Default)]
struct SlashVfx {
    pos: Vec2,
    inner_r: f32,
    outer_r: f32,
    start_angle: f32,
    end_angle: f32,
    lifetime: f32,
    lt_max: f32,
    color: utils::colors::Color,
}

/// Override couleur sur une entité pendant `timer` secondes.
/// Interrogeable via `is_flashing(entity_id)` au moment du rendu de l'entité.
#[derive(Clone, Copy, Default)]
struct FlashVfx {
    entity_id: u64,
    timer: f32,
}

#[derive(Clone, Copy)]
struct TrailPoint {
    pos: Vec2,
    age: f32,
}

/// Ring buffer de `TRAIL_POINTS` positions pour le trail d'épée d'une entité.
/// `head` pointe vers le slot le plus récent (écriture circulaire).
/// `count` permet de savoir combien de points sont valides avant que le buffer soit plein.
struct SwordTrail {
    points: [TrailPoint; TRAIL_POINTS],
    head: usize,
    count: usize,
    color: utils::colors::Color,
}

impl SwordTrail {
    fn new(color: utils::colors::Color) -> Self {
        Self {
            points: [TrailPoint {
                pos: Vec2::zero(),
                age: 0.0,
            }; TRAIL_POINTS],
            head: 0,
            count: 0,
            color,
        }
    }

    /// Pousse une nouvelle position dans le ring buffer.
    fn push(&mut self, pos: Vec2) {
        self.head = (self.head + 1) % TRAIL_POINTS;
        self.points[self.head] = TrailPoint { pos, age: 0.0 };
        if self.count < TRAIL_POINTS {
            self.count += 1;
        }
    }

    /// Vieillit tous les points actifs.
    fn update(&mut self, dt: f32) {
        for i in 0..self.count {
            let idx = (self.head + TRAIL_POINTS - i) % TRAIL_POINTS;
            self.points[idx].age += dt;
        }
    }

    /// Itère les points du plus récent au plus ancien avec leur ratio d'âge [0..1].
    /// `trail_duration` est la durée totale du trail en secondes.
    fn iter_with_alpha(&self, trail_duration: f32) -> impl Iterator<Item = (Vec2, f32)> + '_ {
        (0..self.count).map(move |i| {
            let idx = (self.head + TRAIL_POINTS - i) % TRAIL_POINTS;
            let p = self.points[idx];
            let alpha = 1.0 - (p.age / trail_duration).clamp(0.0, 1.0);
            (p.pos, alpha)
        })
    }
}

/// Copie semi-transparente de la position du joueur au moment du dash.
#[derive(Clone, Copy, Default)]
struct DashGhost {
    pos: Vec2,
    lifetime: f32,
    lt_max: f32,
    color: utils::colors::Color,
}

struct Pool<T: Copy, const N: usize> {
    slots: [(bool, T); N],
}

impl<T: Copy + Default, const N: usize> Pool<T, N> {
    fn new() -> Self {
        Self {
            slots: [(false, T::default()); N],
        }
    }

    fn spawn(&mut self, item: T) {
        if let Some(slot) = self.slots.iter_mut().find(|(active, _)| !active) {
            *slot = (true, item);
        }
    }

    fn iter_active(&self) -> impl Iterator<Item = &T> {
        self.slots
            .iter()
            .filter(|(active, _)| *active)
            .map(|(_, item)| item)
    }

    fn iter_active_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.slots
            .iter_mut()
            .filter(|(active, _)| *active)
            .map(|(_, item)| item)
    }

    fn release_if<F: Fn(&T) -> bool>(&mut self, should_release: F) {
        for (active, item) in &mut self.slots {
            if *active && should_release(item) {
                *active = false;
            }
        }
    }
}

pub struct VfxManager {
    slashes: Pool<SlashVfx, MAX_SLASHES>,
    flashes: Pool<FlashVfx, MAX_FLASHES>,
    sword_trails: HashMap<u64, SwordTrail>,
    dash_ghosts: Pool<DashGhost, MAX_DASH_GHOSTS>,
}

impl VfxManager {
    pub fn new() -> Self {
        Self {
            slashes: Pool::new(),
            flashes: Pool::new(),
            sword_trails: HashMap::new(),
            dash_ghosts: Pool::new(),
        }
    }

    /// Spawn d'un arc de slash centré sur `pos`, orienté selon `angle_deg` (direction d'attaque).
    /// `half_arc` est la demi-ouverture de l'arc en degrés.
    pub fn spawn_slash(
        &mut self,
        pos: Vec2,
        angle_deg: f32,
        inner_r: f32,
        outer_r: f32,
        half_arc: f32,
        duration: f32,
        color: utils::colors::Color,
    ) {
        self.slashes.spawn(SlashVfx {
            pos,
            inner_r,
            outer_r,
            start_angle: angle_deg - half_arc,
            end_angle: angle_deg + half_arc,
            lifetime: duration,
            lt_max: duration,
            color,
        });
    }

    /// Spawn d'un flash d'impact sur l'entité `entity_id` pendant `duration` secondes.
    pub fn spawn_flash(&mut self, entity_id: u64, duration: f32) {
        // Écrase le flash existant pour la même entité si présent
        for flash in self.flashes.iter_active_mut() {
            if flash.entity_id == entity_id {
                flash.timer = duration;
                return;
            }
        }
        self.flashes.spawn(FlashVfx {
            entity_id,
            timer: duration,
        });
    }

    /// Pousse une nouvelle position dans le trail de l'entité `entity_id`.
    /// Le trail est créé automatiquement s'il n'existe pas encore.
    pub fn push_trail_point(&mut self, entity_id: u64, pos: Vec2, color: utils::colors::Color) {
        self.sword_trails
            .entry(entity_id)
            .or_insert_with(|| SwordTrail::new(color))
            .push(pos);
    }

    /// Spawn d'un ghost de dash à la position `pos`.
    pub fn spawn_dash_ghost(&mut self, pos: Vec2, duration: f32, color: utils::colors::Color) {
        self.dash_ghosts.spawn(DashGhost {
            pos,
            lifetime: duration,
            lt_max: duration,
            color,
        });
    }

    /// Retourne `true` si l'entité est actuellement en état de flash.
    /// À appeler au moment du rendu de l'entité pour appliquer l'override couleur.
    pub fn is_flashing(&self, entity_id: u64) -> bool {
        self.flashes.iter_active().any(|f| f.entity_id == entity_id)
    }

    pub fn update(&mut self, dt: f32) {
        // Slashes — diminue lifetime
        for slash in self.slashes.iter_active_mut() {
            slash.lifetime -= dt;
        }
        self.slashes.release_if(|s| s.lifetime <= 0.0);

        // Flashes — diminue timer
        for flash in self.flashes.iter_active_mut() {
            flash.timer -= dt;
        }
        self.flashes.release_if(|f| f.timer <= 0.0);

        // Sword trails — vieillit les points
        for trail in self.sword_trails.values_mut() {
            trail.update(dt);
        }

        // Dash ghosts — diminue lifetime
        for ghost in self.dash_ghosts.iter_active_mut() {
            ghost.lifetime -= dt;
        }
        self.dash_ghosts.release_if(|g| g.lifetime <= 0.0);
    }

    /// Durée totale du trail épée en secondes — les points plus âgés que ça sont invisibles.
    const TRAIL_DURATION: f32 = 0.12;

    pub fn draw<D: RaylibDraw>(&self, d: &mut RaylibMode2D<D>) {
        self.draw_sword_trails(d);
        self.draw_slashes(d);
        self.draw_dash_ghosts(d);
        // Les flashes ne sont pas dessinés ici — ils sont queryés lors du rendu des entités
    }

    fn draw_slashes<D: RaylibDraw>(&self, d: &mut RaylibMode2D<D>) {
        for slash in self.slashes.iter_active() {
            let progress = (slash.lifetime / slash.lt_max).clamp(0.0, 1.0);
            let color = slash.color.alpha(progress);
            let segments = 16;

            d.draw_ring(
                slash.pos,
                slash.inner_r,
                slash.outer_r,
                slash.start_angle,
                slash.end_angle,
                segments,
                color,
            );
        }
    }

    fn draw_sword_trails<D: RaylibDraw>(&self, d: &mut RaylibMode2D<D>) {
        for trail in self.sword_trails.values() {
            let mut prev: Option<(Vec2, f32)> = None;

            for (pos, alpha) in trail.iter_with_alpha(Self::TRAIL_DURATION) {
                if let Some((prev_pos, prev_alpha)) = prev {
                    let color = trail.color.alpha(alpha.min(prev_alpha));
                    // Épaisseur qui diminue avec l'âge
                    let thickness = 4.0 * alpha;
                    d.draw_line_ex(prev_pos, pos, thickness, color);
                }
                prev = Some((pos, alpha));
            }
        }
    }

    fn draw_dash_ghosts<D: RaylibDraw>(&self, d: &mut RaylibMode2D<D>) {
        for ghost in self.dash_ghosts.iter_active() {
            let progress = (ghost.lifetime / ghost.lt_max).clamp(0.0, 1.0);
            let color = ghost.color.alpha(progress * 0.5);
            // Placeholder — sera remplacé par le sprite du joueur avec tint
            d.draw_rectangle(
                ghost.pos.x as i32 - 20,
                ghost.pos.y as i32 - 20,
                40,
                40,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests_vfx {
    use super::*;

    fn v(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
    }

    // -------------------------------------------------------------------------
    // Pool
    // -------------------------------------------------------------------------

    #[test]
    fn pool_spawn_active() {
        #[derive(Clone, Copy, Default)]
        struct Dummy(f32);

        let mut pool: Pool<Dummy, 4> = Pool::new();
        pool.spawn(Dummy(1.0));
        pool.spawn(Dummy(2.0));

        assert_eq!(pool.iter_active().count(), 2);
    }

    #[test]
    fn pool_full_silently_drops() {
        #[derive(Clone, Copy, Default)]
        struct Dummy;

        let mut pool: Pool<Dummy, 2> = Pool::new();
        pool.spawn(Dummy);
        pool.spawn(Dummy);
        pool.spawn(Dummy); // doit être ignoré silencieusement

        assert_eq!(pool.iter_active().count(), 2);
    }

    #[test]
    fn pool_release_if() {
        #[derive(Clone, Copy, Default)]
        struct Timed(f32);

        let mut pool: Pool<Timed, 4> = Pool::new();
        pool.spawn(Timed(0.5));
        pool.spawn(Timed(-0.1)); // déjà expiré
        pool.spawn(Timed(1.0));

        pool.release_if(|t| t.0 <= 0.0);

        assert_eq!(pool.iter_active().count(), 2);
    }

    #[test]
    fn pool_slot_reuse_after_release() {
        #[derive(Clone, Copy, Default)]
        struct Dummy;

        let mut pool: Pool<Dummy, 2> = Pool::new();
        pool.spawn(Dummy);
        pool.spawn(Dummy);
        pool.release_if(|_| true); // libère tout

        pool.spawn(Dummy); // doit réutiliser un slot

        assert_eq!(pool.iter_active().count(), 1);
    }

    // -------------------------------------------------------------------------
    // Flash
    // -------------------------------------------------------------------------

    #[test]
    fn flash_is_active_after_spawn() {
        let mut vfx = VfxManager::new();
        vfx.spawn_flash(42, 0.1);
        assert!(vfx.is_flashing(42));
    }

    #[test]
    fn flash_inactive_for_other_entity() {
        let mut vfx = VfxManager::new();
        vfx.spawn_flash(42, 0.1);
        assert!(!vfx.is_flashing(99));
    }

    #[test]
    fn flash_expires_after_update() {
        let mut vfx = VfxManager::new();
        vfx.spawn_flash(1, 0.05);
        vfx.update(0.1); // dépasse la durée
        assert!(!vfx.is_flashing(1));
    }

    #[test]
    fn flash_reset_on_same_entity() {
        let mut vfx = VfxManager::new();
        vfx.spawn_flash(7, 0.05);
        vfx.update(0.03); // consomme une partie du timer
        vfx.spawn_flash(7, 0.1); // doit reset le timer
        vfx.update(0.07); // ne devrait pas expirer encore
        assert!(vfx.is_flashing(7));
    }

    // -------------------------------------------------------------------------
    // Slash
    // -------------------------------------------------------------------------

    #[test]
    fn slash_active_after_spawn() {
        let mut vfx = VfxManager::new();
        vfx.spawn_slash(
            v(0.0, 0.0),
            90.0,
            10.0,
            30.0,
            45.0,
            0.15,
            utils::colors::Color::WHITE,
        );
        assert_eq!(vfx.slashes.iter_active().count(), 1);
    }

    #[test]
    fn slash_expires_after_update() {
        let mut vfx = VfxManager::new();
        vfx.spawn_slash(
            v(0.0, 0.0),
            0.0,
            5.0,
            20.0,
            30.0,
            0.1,
            utils::colors::Color::WHITE,
        );
        vfx.update(0.2);
        assert_eq!(vfx.slashes.iter_active().count(), 0);
    }

    #[test]
    fn slash_still_active_mid_lifetime() {
        let mut vfx = VfxManager::new();
        vfx.spawn_slash(
            v(0.0, 0.0),
            0.0,
            5.0,
            20.0,
            30.0,
            0.3,
            utils::colors::Color::WHITE,
        );
        vfx.update(0.1);
        assert_eq!(vfx.slashes.iter_active().count(), 1);
    }

    // -------------------------------------------------------------------------
    // Sword trail — SwordTrail directement
    // -------------------------------------------------------------------------

    #[test]
    fn trail_push_count() {
        let mut trail = SwordTrail::new(utils::colors::Color::WHITE);
        trail.push(v(0.0, 0.0));
        trail.push(v(1.0, 0.0));
        trail.push(v(2.0, 0.0));
        assert_eq!(trail.count, 3);
    }

    #[test]
    fn trail_count_caps_at_max() {
        let mut trail = SwordTrail::new(utils::colors::Color::WHITE);
        for i in 0..(TRAIL_POINTS + 5) {
            trail.push(v(i as f32, 0.0));
        }
        assert_eq!(trail.count, TRAIL_POINTS);
    }

    #[test]
    fn trail_ring_overwrites_oldest() {
        let mut trail = SwordTrail::new(utils::colors::Color::WHITE);
        for i in 0..TRAIL_POINTS {
            trail.push(v(i as f32, 0.0));
        }
        trail.push(v(999.0, 0.0)); // écrase le plus ancien

        // Le premier point iter (le plus récent) doit être 999.0
        let first = trail.iter_with_alpha(1.0).next().unwrap().0;
        assert_eq!(first.x, 999.0);
    }

    #[test]
    fn trail_alpha_decreases_with_age() {
        let mut trail = SwordTrail::new(utils::colors::Color::WHITE);
        trail.push(v(0.0, 0.0));
        trail.push(v(1.0, 0.0));
        trail.update(0.05); // vieillit tous les points

        let alphas: Vec<f32> = trail.iter_with_alpha(0.12).map(|(_, a)| a).collect();
        // Le plus récent doit avoir la plus haute alpha
        assert!(alphas[0] >= alphas[1]);
    }

    #[test]
    fn trail_via_vfx_manager_created_on_first_push() {
        let mut vfx = VfxManager::new();
        assert!(vfx.sword_trails.is_empty());
        vfx.push_trail_point(42, v(0.0, 0.0), utils::colors::Color::WHITE);
        assert_eq!(vfx.sword_trails.len(), 1);
    }

    // -------------------------------------------------------------------------
    // Dash ghost
    // -------------------------------------------------------------------------

    #[test]
    fn dash_ghost_active_after_spawn() {
        let mut vfx = VfxManager::new();
        vfx.spawn_dash_ghost(v(10.0, 20.0), 0.2, utils::colors::Color::BLUE);
        assert_eq!(vfx.dash_ghosts.iter_active().count(), 1);
    }

    #[test]
    fn dash_ghost_expires_after_update() {
        let mut vfx = VfxManager::new();
        vfx.spawn_dash_ghost(v(0.0, 0.0), 0.1, utils::colors::Color::BLUE);
        vfx.update(0.2);
        assert_eq!(vfx.dash_ghosts.iter_active().count(), 0);
    }

    // -------------------------------------------------------------------------
    // Update global
    // -------------------------------------------------------------------------

    #[test]
    fn update_clears_expired_across_all_pools() {
        let mut vfx = VfxManager::new();
        vfx.spawn_flash(1, 0.05);
        vfx.spawn_slash(
            v(0.0, 0.0),
            0.0,
            5.0,
            15.0,
            30.0,
            0.05,
            utils::colors::Color::WHITE,
        );
        vfx.spawn_dash_ghost(v(0.0, 0.0), 0.05, utils::colors::Color::RED);

        vfx.update(0.1);

        assert!(!vfx.is_flashing(1));
        assert_eq!(vfx.slashes.iter_active().count(), 0);
        assert_eq!(vfx.dash_ghosts.iter_active().count(), 0);
    }
}
