use std::collections::VecDeque;

use noise::{MultiFractal, NoiseFn, Simplex};

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
            wall_threshold: 0.3,
        }
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
            } else {
                cell.kind = super::cell::CellKind::Floor;
            }
        }
    }

    fn smooth(grid: &mut super::grid::Grid) {
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

                let cell = grid.get_mut(cell_pos.0, cell_pos.1).unwrap(); // A changer unwrap temp
                if wall_neighbors > 5 {
                    cell.kind = super::cell::CellKind::Wall;
                } else {
                    cell.kind = super::cell::CellKind::Floor;
                }
            }
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
            }
        }
    }

    fn clear_player_zone(grid: &mut super::grid::Grid) {

    }
}
