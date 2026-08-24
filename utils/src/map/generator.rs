use std::collections::VecDeque;

use noise::{MultiFractal, NoiseFn, Simplex};
use rand::{Rng, SeedableRng};

pub struct Generator {
    seed: u32,
    width: u32,
    height: u32,
    scale: f64,
    wall_threshold: f64,
}

impl Generator {
    pub fn new(seed: u32, width: u32, height: u32) -> Self {
        Self {
            seed,
            width,
            height,
            scale: 1.0,
            wall_threshold: 0.15,
        }
    }

    pub fn generate(&self) -> super::grid::Grid {
        let mut grid = super::grid::Grid::new(self.width, self.height, 64.0);
        self.apply_noise(&mut grid);
        let floor_after_noise = grid
            .iter()
            .filter(|c| matches!(c.kind, crate::map::cell::CellKind::Floor))
            .count();
        let wall_after_noise = grid
            .iter()
            .filter(|c| matches!(c.kind, crate::map::cell::CellKind::Wall))
            .count();
        tracing::info!(
            "Après noise — Floor: {}, Wall: {}",
            floor_after_noise,
            wall_after_noise
        );
        self.smooth(&mut grid);
        let floor_after_noise = grid
            .iter()
            .filter(|c| matches!(c.kind, crate::map::cell::CellKind::Floor))
            .count();
        let wall_after_noise = grid
            .iter()
            .filter(|c| matches!(c.kind, crate::map::cell::CellKind::Wall))
            .count();
        tracing::info!(
            "Après smooth — Floor: {}, Wall: {}",
            floor_after_noise,
            wall_after_noise
        );
        Self::flood_fill_connectivity(&mut grid);
        Self::clear_player_zone(&mut grid);
        Self::place_spawn_zone(&mut grid);
        self.place_structures(&mut grid);
        self.scatter_debris(&mut grid);
        grid
    }

    fn apply_noise(&self, grid: &mut super::grid::Grid) {
        let noise = noise::Fbm::<Simplex>::new(self.seed)
            .set_octaves(6)
            .set_frequency(0.05)
            .set_persistence(noise::Fbm::<Simplex>::DEFAULT_PERSISTENCE)
            .set_lacunarity(noise::Fbm::<Simplex>::DEFAULT_LACUNARITY);
        for (x, y, cell) in grid.iter_mut_with_pos() {
            let threshold = noise.get([x as f64 * self.scale, y as f64 * self.scale]);
            if threshold < self.wall_threshold {
                cell.kind = super::cell::CellKind::Wall;
                cell.cost = 255;
            } else {
                cell.kind = super::cell::CellKind::Floor;
                cell.cost = 1;
            }
        }
    }

    fn smooth(&self, grid: &mut super::grid::Grid) {
        let mut new_grid = super::grid::Grid::new(self.width, self.height, 64.0);
        for _ in 0..5 {
            for cell_index in 0..grid.len() {
                let cell_pos = grid.cell_pos(cell_index);
                let cell_neighbors = grid.neighbors(cell_pos.0, cell_pos.1);
                let mut wall_neighbors = 0;

                for neighbor in cell_neighbors {
                    if !grid.is_walkable(neighbor.0, neighbor.1) {
                        wall_neighbors += 1;
                    }
                }
                if wall_neighbors > 5 {
                    new_grid.set_cell(
                        cell_pos.0,
                        cell_pos.1,
                        super::cell::Cell {
                            kind: super::cell::CellKind::Wall,
                            cost: 255,
                        },
                    );
                } else {
                    new_grid.set_cell(
                        cell_pos.0,
                        cell_pos.1,
                        super::cell::Cell {
                            kind: super::cell::CellKind::Floor,
                            cost: 1,
                        },
                    );
                }
            }
            std::mem::swap(grid, &mut new_grid);
        }
    }

    fn flood_fill_connectivity(grid: &mut super::grid::Grid) {
        /// Fonction récursive locale qui s'éloigne du centre anneau par anneau
        fn find_start_floor(
            grid: &super::grid::Grid,
            cx: u32,
            cy: u32,
            r: u32,
            max_r: u32,
        ) -> Option<(u32, u32)> {
            if r > max_r {
                return None;
            }

            let min_x = cx.saturating_sub(r);
            let max_x = (cx + r).min(grid.width() - 1);
            let min_y = cy.saturating_sub(r);
            let max_y = (cy + r).min(grid.height() - 1);

            // Parcourt le périmètre du carré à la distance `r`
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    if (x == min_x || x == max_x || y == min_y || y == max_y)
                        && grid.is_walkable(x, y)
                    {
                        return Some((x, y));
                    }
                }
            }

            find_start_floor(grid, cx, cy, r + 1, max_r)
        }

        let center_x = grid.width() / 2;
        let center_y = grid.height() / 2;
        let max_rayon = grid.width().max(grid.height());
        let start_pos = match find_start_floor(grid, center_x, center_y, 0, max_rayon) {
            Some(pos) => pos,
            None => return,
        };

        let mut queue = VecDeque::new();
        let mut visited = vec![false; grid.len()];

        let start_idx = grid.cell_index(start_pos.0, start_pos.1);
        visited[start_idx] = true;
        queue.push_back(start_pos);

        while let Some((x, y)) = queue.pop_front() {
            for (nx, ny) in grid.neighbors(x, y) {
                let idx = grid.cell_index(nx, ny);
                if !visited[idx] && grid.is_walkable(nx, ny) {
                    visited[idx] = true;
                    queue.push_back((nx, ny));
                }
            }
        }

        for (idx, cell) in grid.iter_mut().enumerate() {
            if matches!(cell.kind, super::cell::CellKind::Floor) && !visited[idx] {
                cell.kind = super::cell::CellKind::Wall;
                cell.cost = 255;
            }
        }
    }

    fn clear_player_zone(grid: &mut super::grid::Grid) {
        let center_x = grid.width() / 2;
        let center_y = grid.height() / 2;
        let max_distance = 5;

        let mut queue = VecDeque::new();
        let mut visited = vec![false; grid.len()];

        let start_idx = grid.cell_index(center_x, center_y);
        visited[start_idx] = true;
        queue.push_back((center_x, center_y, 0));

        while let Some((x, y, dist)) = queue.pop_front() {
            if let Some(cell) = grid.get_mut(x, y) {
                cell.kind = super::cell::CellKind::Floor;
            }

            if dist < max_distance {
                for (nx, ny) in grid.neighbors(x, y) {
                    let idx = grid.cell_index(nx, ny);
                    if !visited[idx] {
                        visited[idx] = true;
                        queue.push_back((nx, ny, dist + 1));
                    }
                }
            }
        }
    }

    fn place_spawn_zone(grid: &mut super::grid::Grid) {
        let mut spawnable = vec![false; grid.len()];
        let width = grid.width();
        let height = grid.height();
        let thickness = 3;

        for (x, y, cell) in grid.iter_with_pos() {
            // Vérifie si la case (x, y) se trouve dans la bande périphérique de `thickness` cases depuis le bord.
            // - Gauche / Haut : indices de `0` à `thickness - 1` (ex: 0, 1, 2)
            // - Droit / Bas   : indices de `taille - thickness` à `taille - 1` (ex pour taille 10 : 7, 8, 9)
            let is_ring = x < thickness
                || x >= width.saturating_sub(thickness)
                || y < thickness
                || y >= height.saturating_sub(thickness);

            if is_ring && matches!(cell.kind, super::cell::CellKind::Floor) {
                let idx = grid.cell_index(x, y);
                spawnable[idx] = true;
            }
        }
        for (idx, spawn) in spawnable.iter().enumerate() {
            if *spawn {
                let cell_pos = grid.cell_pos(idx);
                grid.set_cell(
                    cell_pos.0,
                    cell_pos.1,
                    super::cell::Cell {
                        kind: super::cell::CellKind::Spawn,
                        cost: 1,
                    },
                );
            }
        }
    }
    fn place_structures(&self, grid: &mut super::grid::Grid) {
        // Rayon de 12 cases pour séparer le centre des bâtiments (ex: usines 10x8)
        let radius = 12.0;
        let points = generate_poisson_points(grid.width(), grid.height(), radius, 30, self.seed);

        let templates = vec![
            Template::house_5x5(),
            Template::factory_10x8(),
            Template::bridge_3x8(),
        ];

        for (x, y) in points {
            // Sélection d'un template (ici un choix simple basé sur le point)
            let template = &templates[(x as usize + y as usize) % templates.len()];

            // Centrer le bâtiment sur le point Poisson Disk
            let origin_x = x.saturating_sub(template.width / 2);
            let origin_y = y.saturating_sub(template.height / 2);

            // Tente de placer : vérifie les limites, applique le template et teste le BFS
            try_place_template(grid, template, origin_x, origin_y);
        }
    }

    fn scatter_debris(&self, grid: &mut super::grid::Grid) {
        let noise = noise::Fbm::<noise::Simplex>::new(self.seed)
            .set_octaves(4)
            .set_frequency(0.03)
            .set_persistence(noise::Fbm::<Simplex>::DEFAULT_PERSISTENCE)
            .set_lacunarity(noise::Fbm::<Simplex>::DEFAULT_LACUNARITY);
        let water_threshold = -0.1;
        for (x, y, cell) in grid.iter_mut_with_pos() {
            let threshold = noise.get([x as f64 * self.scale, y as f64 * self.scale]);
            if threshold < water_threshold && matches!(cell.kind, super::cell::CellKind::Floor) {
                cell.kind = super::cell::CellKind::Water;
                cell.cost = 40;
            }
        }
        let debris_density = 0.08;
        for cell in grid.iter_mut() {
            let prob = rand::rngs::StdRng::seed_from_u64(self.seed as u64).gen_range(0.0..1.0);
            if prob < debris_density && matches!(cell.kind, super::cell::CellKind::Floor) {
                cell.kind = super::cell::CellKind::Debris;
            }
        }
    }
}

fn generate_poisson_points(
    width: u32,
    height: u32,
    radius: f32,
    k: u32,
    seed: u32,
) -> Vec<(u32, u32)> {
    use rand::prelude::*;
    use rand::rngs::StdRng;
    use std::f32::consts::PI;
    // Générateur déterministe basé sur ta seed
    let mut rng = StdRng::seed_from_u64(seed as u64);

    let cell_size = radius / 1.414_213_5; // radius / sqrt(2)
    let grid_w = (width as f32 / cell_size).ceil() as usize;
    let grid_h = (height as f32 / cell_size).ceil() as usize;

    let mut grid = vec![None; grid_w * grid_h];
    let mut points = Vec::new();
    let mut active = Vec::new();

    // 1. Premier point aléatoire dans la carte
    let first_point = (
        rng.gen_range(0.0..width as f32),
        rng.gen_range(0.0..height as f32),
    );
    points.push(first_point);
    active.push(0);

    let gx = (first_point.0 / cell_size) as usize;
    let gy = (first_point.1 / cell_size) as usize;
    grid[gy * grid_w + gx] = Some(0);

    // 2. Exploration
    while !active.is_empty() {
        let active_idx = rng.gen_range(0..active.len());
        let point_idx = active[active_idx];
        let point = points[point_idx];
        let mut found = false;

        for _ in 0..k {
            let angle = rng.gen_range(0.0..2.0 * PI);
            let dist = rng.gen_range(radius..2.0 * radius);
            let candidate = (point.0 + angle.cos() * dist, point.1 + angle.sin() * dist);

            if candidate.0 >= 0.0
                && candidate.0 < width as f32
                && candidate.1 >= 0.0
                && candidate.1 < height as f32
            {
                let cx = (candidate.0 / cell_size) as usize;
                let cy = (candidate.1 / cell_size) as usize;

                let mut too_close = false;
                let min_gx = cx.saturating_sub(2);
                let max_gx = (cx + 2).min(grid_w - 1);
                let min_gy = cy.saturating_sub(2);
                let max_gy = (cy + 2).min(grid_h - 1);

                'outer: for gy in min_gy..=max_gy {
                    for gx in min_gx..=max_gx {
                        if let Some(other_idx) = grid[gy * grid_w + gx] {
                            let other = points[other_idx];
                            let dx = candidate.0 - other.0;
                            let dy = candidate.1 - other.1;
                            if dx * dx + dy * dy < radius * radius {
                                too_close = true;
                                break 'outer;
                            }
                        }
                    }
                }

                if !too_close {
                    let new_idx = points.len();
                    points.push(candidate);
                    active.push(new_idx);
                    grid[cy * grid_w + cx] = Some(new_idx);
                    found = true;
                    break;
                }
            }
        }

        if !found {
            active.swap_remove(active_idx);
        }
    }

    points
        .into_iter()
        .map(|p| (p.0 as u32, p.1 as u32))
        .collect()
}

fn try_place_template(
    grid: &mut super::grid::Grid,
    template: &Template,
    origin_x: u32,
    origin_y: u32,
) -> bool {
    use std::collections::HashSet;

    if origin_x + template.width >= grid.width() || origin_y + template.height >= grid.height() {
        return false;
    }
    let mut perimeter_floors = Vec::new();
    let min_x = origin_x.saturating_sub(1);
    let max_x = (origin_x + template.width).min(grid.width() - 1);
    let min_y = origin_y.saturating_sub(1);
    let max_y = (origin_y + template.height).min(grid.height() - 1);

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let is_on_border = x == min_x || x == max_x || y == min_y || y == max_y;
            if is_on_border && grid.is_walkable(x, y) {
                perimeter_floors.push((x, y));
            }
        }
    }
    if perimeter_floors.len() <= 1 {
        apply_template(grid, template, origin_x, origin_y);
        return true;
    }
    let backup = apply_template_with_backup(grid, template, origin_x, origin_y);

    let start_pos = perimeter_floors[0];
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    visited.insert(start_pos);
    queue.push_back(start_pos);

    while let Some((x, y)) = queue.pop_front() {
        for (nx, ny) in grid.neighbors(x, y) {
            if grid.is_walkable(nx, ny) && !visited.contains(&(nx, ny)) {
                visited.insert((nx, ny));
                queue.push_back((nx, ny));
            }
        }
    }
    let is_valid = perimeter_floors.iter().all(|pos| visited.contains(pos));

    if !is_valid {
        // Restauration de l'état précédent si le bâtiment isole une zone
        restore_grid(grid, backup);
    }

    is_valid
}

fn restore_grid(grid: &mut super::grid::Grid, backup: Vec<((u32, u32), super::cell::CellKind)>) {
    for ((x, y), old_kind) in backup {
        if let Some(cell) = grid.get_mut(x, y) {
            cell.kind = old_kind;
        }
    }
}

fn apply_template(grid: &mut super::grid::Grid, template: &Template, origin_x: u32, origin_y: u32) {
    for ((rel_x, rel_y), kind) in &template.cells {
        let x = origin_x + rel_x;
        let y = origin_y + rel_y;
        if let Some(cell) = grid.get_mut(x, y) {
            cell.kind = *kind;
        }
    }
}

/// Applique un template et renvoie un backup de l'état précédent des cases modifiées
fn apply_template_with_backup(
    grid: &mut super::grid::Grid,
    template: &Template,
    origin_x: u32,
    origin_y: u32,
) -> Vec<((u32, u32), super::cell::CellKind)> {
    let mut backup = Vec::with_capacity(template.cells.len());

    for ((rel_x, rel_y), new_kind) in &template.cells {
        let x = origin_x + rel_x;
        let y = origin_y + rel_y;

        if let Some(cell) = grid.get_mut(x, y) {
            // On sauvegarde la position absolue et l'ancien type de case
            backup.push(((x, y), cell.kind));
            // On applique la modification
            cell.kind = *new_kind;
        }
    }

    backup
}

struct Template {
    pub width: u32,
    pub height: u32,
    // Emplacements relatifs des murs et sols du template
    pub cells: Vec<((u32, u32), super::cell::CellKind)>,
}

impl Template {
    pub fn house_5x5() -> Self {
        let mut cells = Vec::new();

        // Murs du périmètre
        for x in 0..5u32 {
            for y in 0..5u32 {
                let is_perimeter = x == 0 || x == 4 || y == 0 || y == 4;
                let is_door = x == 2 && y == 4; // entrée bas-centre

                if is_perimeter && !is_door {
                    cells.push(((x, y), super::cell::CellKind::Wall));
                }
                // intérieur reste Floor implicitement
            }
        }

        Self {
            width: 5,
            height: 5,
            cells,
        }
    }
    pub fn factory_10x8() -> Self {
        let mut cells = Vec::new();

        for x in 0..10u32 {
            for y in 0..8u32 {
                let is_perimeter = x == 0 || x == 9 || y == 0 || y == 7;

                // Entrées bas : x=2 et x=7
                let is_door = y == 7 && (x == 2 || x == 7);

                // Séparation interne à y=3, avec ouverture x=3..=5
                let is_divider = y == 3 && !(x >= 3 && x <= 5);

                if (is_perimeter && !is_door) || is_divider {
                    cells.push(((x, y), super::cell::CellKind::Wall));
                }
            }
        }

        Self {
            width: 10,
            height: 8,
            cells,
        }
    }
    pub fn bridge_3x8() -> Self {
        let mut cells = Vec::new();

        for x in 0..3u32 {
            for y in 0..8u32 {
                // Murs sur les colonnes gauche et droite
                // sauf au milieu (y=3 et y=4) qui sont les côtés ouverts
                let is_side_wall = (x == 0 || x == 2) && !(y == 3 || y == 4);

                if is_side_wall {
                    cells.push(((x, y), super::cell::CellKind::Wall));
                }
            }
        }

        Self {
            width: 3,
            height: 8,
            cells,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::cell::CellKind;
    use crate::map::grid::Grid;

    // --- Utilitaires de test ---

    /// Génère une grille remplie uniquement de sols (Floor)
    fn create_empty_grid(width: u32, height: u32) -> Grid {
        let mut grid = Grid::new(width, height, 1.0);
        for cell in grid.iter_mut() {
            cell.kind = CellKind::Floor;
        }
        grid
    }

    /// Crée un template de mur plein de taille W x H
    fn create_wall_template(w: u32, h: u32) -> Template {
        let mut cells = Vec::new();
        for x in 0..w {
            for y in 0..h {
                cells.push(((x, y), CellKind::Wall));
            }
        }
        Template {
            width: w,
            height: h,
            cells,
        }
    }

    // --- Tests de sauvegarde et restauration ---

    #[test]
    fn test_backup_and_restore_integrity() {
        let mut grid = create_empty_grid(10, 10);
        let template = create_wall_template(3, 3);

        // Application avec sauvegarde
        let backup = apply_template_with_backup(&mut grid, &template, 2, 2);

        // Vérifie que les murs ont bien été appliqués
        assert_eq!(grid.get(2, 2).unwrap().kind, CellKind::Wall);
        assert_eq!(grid.get(4, 4).unwrap().kind, CellKind::Wall);

        // Restauration
        restore_grid(&mut grid, backup);

        // Vérifie que tout est redevenu du sol (Floor)
        assert_eq!(grid.get(2, 2).unwrap().kind, CellKind::Floor);
        assert_eq!(grid.get(4, 4).unwrap().kind, CellKind::Floor);
    }

    // --- Tests de placement de structures ---

    #[test]
    fn test_try_place_out_of_bounds() {
        let mut grid = create_empty_grid(10, 10);
        let template = create_wall_template(5, 5);

        // Tentative de placement débordant à droite (8 + 5 = 13 > 10)
        let placed = try_place_template(&mut grid, &template, 8, 8);
        assert!(!placed, "Le bâtiment déborde et devrait être refusé");
    }

    #[test]
    fn test_try_place_valid_in_open_space() {
        let mut grid = create_empty_grid(20, 20);
        let template = create_wall_template(4, 4);

        // Placement au milieu d'un grand terrain dégagé
        let placed = try_place_template(&mut grid, &template, 5, 5);

        assert!(placed, "Le placement en terrain dégagé doit réussir");
        assert_eq!(
            grid.get(5, 5).unwrap().kind,
            CellKind::Wall,
            "Le bâtiment doit être écrit sur la grille"
        );
    }

    #[test]
    fn test_try_place_rejects_blocking_corridor() {
        let mut grid = Grid::new(10, 5, 1.0);

        // Remplit la grille de murs
        for cell in grid.iter_mut() {
            cell.kind = CellKind::Wall;
        }

        // Crée un couloir unique de 1 case de large sur la ligne y = 2
        for x in 0..10 {
            grid.get_mut(x, 2).unwrap().kind = CellKind::Floor;
        }

        // On tente de placer un mur de 1x1 au milieu du couloir à (4, 2)
        let blocker_template = create_wall_template(1, 1);
        let placed = try_place_template(&mut grid, &blocker_template, 4, 2);

        assert!(
            !placed,
            "Le BFS doit refuser le placement car il coupe le seul couloir reliant la gauche et la droite"
        );

        // Vérifie le Rollback : la case du couloir doit être restée 'Floor'
        assert_eq!(
            grid.get(4, 2).unwrap().kind,
            CellKind::Floor,
            "La grille doit être restaurée après un échec"
        );
    }

    // --- Tests du Poisson Disk Sampling ---

    #[test]
    fn test_poisson_disk_sampling_determinism() {
        let seed = 42;
        let points1 = generate_poisson_points(100, 100, 10.0, 30, seed);
        let points2 = generate_poisson_points(100, 100, 10.0, 30, seed);

        assert_eq!(
            points1, points2,
            "À seed identique, les points générés doivent être strictement identiques"
        );
    }

    #[test]
    fn test_poisson_disk_sampling_min_distance_and_bounds() {
        let width = 80;
        let height = 80;
        let radius = 12.0;
        let points = generate_poisson_points(width, height, radius, 30, 999);

        assert!(
            !points.is_empty(),
            "La liste de points ne doit pas être vide"
        );

        // 1. Vérification des limites
        for &(x, y) in &points {
            assert!(
                x < width && y < height,
                "Le point ({}, {}) dépasse les limites de la carte ({}, {})",
                x,
                y,
                width,
                height
            );
        }

        // 2. Vérification de la distance minimale entre toutes les paires de points
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let (x1, y1) = (points[i].0 as f32, points[i].1 as f32);
                let (x2, y2) = (points[j].0 as f32, points[j].1 as f32);

                let dist_sq = (x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2);

                // Marge d'erreur minime pour les conversions f32 -> u32
                let min_dist_sq = (radius - 0.5) * (radius - 0.5);

                assert!(
                    dist_sq >= min_dist_sq,
                    "Les points {:?} et {:?} sont trop proches! Distance: {}",
                    points[i],
                    points[j],
                    dist_sq.sqrt()
                );
            }
        }
    }
}
