use std::collections::VecDeque;

use crate::{map::grid::Grid, math::Vec2};

pub struct FlowField {
    cost_field: Vec<u32>,
    direction_field: Vec<crate::math::Vec2>,
    target: (u32, u32), // Ou Vec2 a voir
}

impl FlowField {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            cost_field: vec![u32::MAX; (width * height) as usize],
            direction_field: vec![Vec2::zero(); (width * height) as usize],
            target: (u32::MAX, u32::MAX), // Force needs_update à renvoyer true le premier coup
        }
    }

    pub fn compute(&mut self, grid: &Grid, target: Vec2) {
        let (tx, ty) = grid.world_to_grid(target);
        
        // INDISPENSABLE : enregistrer la nouvelle case cible calculée
        self.target = (tx, ty);

        self.compute_cost(grid, (tx, ty));
        self.compute_direction(grid);
    }

    fn compute_cost(&mut self, grid: &Grid, target: (u32, u32)) {
        self.cost_field.fill(u32::MAX);
        let mut visited = vec![false; grid.len()];
        let mut queue = VecDeque::new();

        let (tx, ty) = target;
        let start_index = grid.cell_index(tx, ty);

        self.cost_field[start_index] = 0;
        visited[start_index] = true;
        queue.push_back((tx, ty));

        while let Some((x, y)) = queue.pop_front() {
            let current_index = grid.cell_index(x, y);
            let current_cost = self.cost_field[current_index];

            for (nx, ny) in grid.neighbors(x, y) {
                let neighbors_index = grid.cell_index(nx, ny);
                let neighbor_cell = grid.get(nx, ny).unwrap();

                if !visited[neighbors_index] && grid.is_walkable(nx, ny) {
                    //Chaque pas coûte 1 de base + le coût éventuel du terrain
                    let step_cost = 1 + neighbor_cell.cost as u32;
                    self.cost_field[neighbors_index] = current_cost + step_cost;
                    visited[neighbors_index] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }

    fn compute_direction(&mut self, grid: &Grid) {
        for (x, y, _cell) in grid.iter_with_pos() {
            let current_index = grid.cell_index(x, y);

            // Si la case est un mur, sa direction reste nulle
            if !grid.is_walkable(x, y) {
                self.direction_field[current_index] = Vec2::zero();
                continue;
            }

            let mut min_cost = self.cost_field[current_index];
            let mut min_cost_neighbors = current_index;

            for (nx, ny) in grid.neighbors(x, y) {
                let n_idx = grid.cell_index(nx, ny);
                let n_cost = self.cost_field[n_idx];

                if n_cost < min_cost {
                    min_cost = n_cost;
                    min_cost_neighbors = n_idx;
                }
            }

            let (nx, ny) = grid.cell_pos(min_cost_neighbors);
            let dx = nx as f32 - x as f32;
            let dy = ny as f32 - y as f32;
            let dir = Vec2::new(dx, dy);

            self.direction_field[current_index] = if dir.length() > 0.0 {
                dir.normalize()
            } else {
                Vec2::zero()
            };
        }
    }

    pub fn get_direction(&self, grid: &Grid, pos: Vec2) -> Vec2 {
        let (x, y) = grid.world_to_grid(pos);
        let cell_idx = grid.cell_index(x, y);
        self.direction_field[cell_idx]
    }

    pub fn needs_update(&self, grid: &Grid, new_target: Vec2) -> bool {
        let (new_tx, new_ty) = grid.world_to_grid(new_target);
        self.target != (new_tx, new_ty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::cell::*;
    use crate::math::Vec2;

    // Helper pour générer une grille de test vide
    fn create_test_grid(width: u32, height: u32) -> Grid {
        let grid = Grid::new(width, height, 10.0);
        grid
    }

    #[test]
    fn test_needs_update() {
        let mut field = FlowField::new(10, 10);
        let grid = create_test_grid(10, 10);

         // Position monde dans la case (2, 2) avec cell_size = 10.0
         let target_a = Vec2::new(25.0, 25.0);
         field.compute(&grid, target_a);

         // Petit déplacement dans la MÊME case (2, 2)
         let target_b = Vec2::new(22.0, 28.0);
         assert!(!field.needs_update(&grid, target_b));

         // Déplacement dans une AUTRE case (3, 2)
         let target_c = Vec2::new(35.0, 25.0);
         assert!(field.needs_update(&grid, target_c));
     }

    #[test]
    fn test_cost_field_propagation() {
        let grid = create_test_grid(5, 5);
        let mut field = FlowField::new(5, 5);

        // Cible au centre (2, 2)
        let target = Vec2::new(25.0, 25.0);
        field.compute(&grid, target);

        let target_idx = grid.cell_index(2, 2);
        let neighbor_idx = grid.cell_index(2, 1);
        let far_idx = grid.cell_index(0, 0);

        // La cible est à 0
        assert_eq!(field.cost_field[target_idx], 0);
        // Le voisin est plus cher que la cible
        assert!(field.cost_field[neighbor_idx] > field.cost_field[target_idx]);
        // Une case plus loin est encore plus chère
        assert!(field.cost_field[far_idx] > field.cost_field[neighbor_idx]);
    }

    #[test]
    fn test_direction_pointing_to_target() {
        let grid = create_test_grid(5, 5);
        let mut field = FlowField::new(5, 5);

        // Cible en (2, 2)
        field.compute(&grid, Vec2::new(25.0, 25.0));

        // Ennemi en (1, 2) -> à gauche de la cible
        // Le vecteur de direction doit pointer vers la droite (+X)
        let dir = field.get_direction(&grid, Vec2::new(15.0, 25.0));
        assert!(dir.x > 0.0);
        assert_eq!(dir.y, 0.0);
    }

    #[test]
    fn test_wall_avoidance() {
        let mut grid = create_test_grid(5, 5);

        // On place un mur en (2, 1) juste au-dessus de la cible en (2, 2)
        let wall_idx = grid.cell_index(2, 1);
        grid.set_cell(
            2,
            1,
            Cell {
                kind: CellKind::Wall,
                cost: 255,
            },
        );

        let mut field = FlowField::new(5, 5);
        field.compute(&grid, Vec2::new(25.0, 25.0));

        // Le coût du mur doit rester au maximum
        assert_eq!(field.cost_field[wall_idx], u32::MAX);

        // Une case au-dessus du mur (2, 0) ne doit PAS pointer vers le mur
        let dir = field.get_direction(&grid, Vec2::new(25.0, 5.0));
        assert_ne!(dir, Vec2::new(0.0, 1.0)); // Ne doit pas pousser vers le bas (dans le mur)
    }
}
