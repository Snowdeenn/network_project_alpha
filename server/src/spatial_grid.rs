use crate::simulation::components::*;

pub struct SpatialGrid {
    cells: Vec<usize>,
    offsets: Vec<usize>,
    counts: Vec<usize>,
    pending: Vec<(usize, usize)>,
    cell_size: f64,
    cols: usize,
    rows: usize,
}

#[allow(dead_code)]
impl SpatialGrid {
    pub fn new(cell_size: f64, arena_w: f64, arena_h: f64) -> Self {
        let cols = (arena_w / cell_size).ceil() as usize;
        let rows = (arena_h / cell_size).ceil() as usize;
        Self {
            cells: Vec::with_capacity(cols * rows * 4),
            offsets: vec![0; cols * rows],
            counts: vec![0; cols * rows],
            pending: vec![],
            cell_size,
            cols,
            rows,
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.counts.fill(0);
        self.offsets.fill(0);
        self.cells.clear();
        self.pending.clear();
    }

    pub fn cell_index(&self, x: f64, y: f64) -> Option<usize> {
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let col = (x / self.cell_size) as usize;
        let row = (y / self.cell_size) as usize;
        if col < self.cols && row < self.rows {
            Some(row * self.cols + col)
        } else {
            None
        }
    }

    pub fn insert(&mut self, index: usize, pos: &Position, collider: &Collider) {
        if let Some((min_col, max_col, min_row, max_row)) = self.bounds(pos, collider) {
            for row in min_row..=max_row {
                for col in min_col..=max_col {
                    let cell_index = row * self.cols + col;
                    self.counts[cell_index] += 1;
                    self.pending.push((index, cell_index));
                }
            }
        }
    }

    pub fn build(&mut self) {
        let total: usize = self.counts.iter().sum();
        if total == 0 {
            return;
        }

        self.offsets[0] = 0;
        for i in 1..self.counts.len() {
            self.offsets[i] = self.offsets[i - 1] + self.counts[i - 1];
        }

        self.cells.resize(total, 0);

        let mut write_pos = self.offsets.clone();
        self.pending.iter().for_each(|&(index, cell_index)| {
            if cell_index < write_pos.len() && write_pos[cell_index] < self.cells.len() {
                self.cells[write_pos[cell_index]] = index;
                write_pos[cell_index] += 1;
            }
        });
    }

    pub fn query(&self, pos: &Position, collider: &Collider, out: &mut Vec<usize>) {
        if let Some((min_col, max_col, min_row, max_row)) = self.bounds(pos, collider) {
            for row in min_row..=max_row {
                for col in min_col..=max_col {
                    let cell_index = row * self.cols + col;
                    let start = self.offsets[cell_index];
                    let end = start + self.counts[cell_index];
                    out.extend_from_slice(&self.cells[start..end]);
                }
            }
        }
    }

    fn bounds(&self, pos: &Position, collider: &Collider) -> Option<(usize, usize, usize, usize)> {
        let max_x = pos.x + collider.w;
        let max_y = pos.y + collider.h;

        // Si l'objet est complètement en dehors de la grille
        if max_x < 0.0 || pos.x >= (self.cols as f64 * self.cell_size)
            || max_y < 0.0 || pos.y >= (self.rows as f64 * self.cell_size) {
            return None;
        }

        let min_col = (pos.x / self.cell_size).floor().max(0.0) as usize;
        let max_col = ((max_x / self.cell_size).floor() as usize).min(self.cols - 1);
        let min_row = (pos.y / self.cell_size).floor().max(0.0) as usize;
        let max_row = ((max_y / self.cell_size).floor() as usize).min(self.rows - 1);

        if min_col > max_col || min_row > max_row {
            None
        } else {
            Some((min_col, max_col, min_row, max_row))
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    const CELL_SIZE: f64 = 128.0;
    const ARENA_W: f64 = 1920.0;
    const ARENA_H: f64 = 1080.0;

    fn create_test_grid() -> SpatialGrid {
        SpatialGrid::new(CELL_SIZE, ARENA_W, ARENA_H)
    }

    // =========================================================================
    // 1. TESTS DES DIMENSIONS DE LA GRILLE
    // =========================================================================

    #[test]
    fn test_grid_dimensions_calculation() {
        let grid = create_test_grid();
        // 1920 / 128 = 15 colonnes
        // 1080 / 128 = 8.4375 -> 9 lignes (avec ceil/round)
        assert_eq!(grid.cols, 15);
        assert_eq!(grid.rows, 9);
    }

    // =========================================================================
    // 2. TESTS DE CALCUL DE BORNES (bounds / cell indexing)
    // =========================================================================

    #[test]
    fn test_bounds_inside_single_cell() {
        let grid = create_test_grid();
        let pos = Position { x: 50.0, y: 50.0 };
        let col = Collider { w: 30.0, h: 30.0 };

        // Doit être entièrement dans la cellule (colonne 0, ligne 0)
        let bounds = grid.bounds(&pos, &col);
        assert_eq!(bounds, Some((0, 0, 0, 0)));
    }

    #[test]
    fn test_bounds_straddling_multiple_cells() {
        let grid = create_test_grid();
        // Placé à cheval entre la cellule 0 et 1 en X, et 0 et 1 en Y
        let pos = Position { x: 100.0, y: 100.0 };
        let col = Collider { w: 50.0, h: 50.0 }; // s'étend jusqu'à (150, 150)

        let bounds = grid.bounds(&pos, &col);
        // Min: (0,0), Max: (1,1) -> Couvre 4 cellules
        assert_eq!(bounds, Some((0, 1, 0, 1)));
    }

    #[test]
    fn test_bounds_exact_cell_boundaries() {
        let grid = create_test_grid();
        // Exactement sur la ligne de séparation (128.0)
        let pos = Position { x: 128.0, y: 128.0 };
        let col = Collider { w: 10.0, h: 10.0 };

        let bounds = grid.bounds(&pos, &col);
        // Doit appartenir à la cellule (1, 1) et pas (0, 0)
        assert_eq!(bounds, Some((1, 1, 1, 1)));
    }

    #[test]
    fn test_bounds_exact_arena_edges() {
        let grid = create_test_grid();
        // Entité collée au bord supérieur gauche (0.0, 0.0)
        let pos = Position { x: 0.0, y: 0.0 };
        let col = Collider { w: 10.0, h: 10.0 };
        assert_eq!(grid.bounds(&pos, &col), Some((0, 0, 0, 0)));

        // Entité collée au coin inférieur droit extrême (1910.0, 1070.0)
        let pos_end = Position { x: 1910.0, y: 1070.0 };
        let col_end = Collider { w: 10.0, h: 10.0 };
        assert_eq!(grid.bounds(&pos_end, &col_end), Some((14, 14, 8, 8)));
    }

    // =========================================================================
    // 3. TESTS DES CAS LIMITES HORS-CHAMP (OUT OF BOUNDS)
    // =========================================================================

    #[test]
    fn test_bounds_completely_negative_coords() {
        let grid = create_test_grid();
        let pos = Position { x: -100.0, y: -50.0 };
        let col = Collider { w: 20.0, h: 20.0 };

        // Doit être ignoré ou clampé proprement sans crash/panic
        let bounds = grid.bounds(&pos, &col);
        assert!(bounds.is_none() || bounds == Some((0, 0, 0, 0)));
    }

    #[test]
    fn test_bounds_completely_exceeding_arena() {
        let grid = create_test_grid();
        let pos = Position { x: 3000.0, y: 2000.0 };
        let col = Collider { w: 50.0, h: 50.0 };

        let bounds = grid.bounds(&pos, &col);
        // Doit renvoyer None ou être clampé à la dernière cellule
        assert!(bounds.is_none() || bounds == Some((14, 14, 8, 8)));
    }

    #[test]
    fn test_bounds_partially_outside_negative() {
        let grid = create_test_grid();
        // Entité spawner à cheval sur le bord gauche (x = -10.0, w = 30.0 -> dépasse à 20.0)
        let pos = Position { x: -10.0, y: 50.0 };
        let col = Collider { w: 30.0, h: 20.0 };

        let bounds = grid.bounds(&pos, &col);
        // Min_col ne doit PAS overflow en usize, et doit être ramené à 0
        assert!(bounds.is_some());
        let (min_col, max_col, _, _) = bounds.unwrap();
        assert_eq!(min_col, 0);
        assert_eq!(max_col, 0);
    }

    #[test]
    fn test_bounds_partially_outside_positive() {
        let grid = create_test_grid();
        // Entité qui dépasse du bord droit de l'arène (x = 1910.0, w = 50.0 -> jusqu'à 1960.0)
        let pos = Position { x: 1910.0, y: 100.0 };
        let col = Collider { w: 50.0, h: 20.0 };

        let bounds = grid.bounds(&pos, &col);
        assert!(bounds.is_some());
        let (min_col, max_col, _, _) = bounds.unwrap();
        assert_eq!(min_col, 14);
        assert_eq!(max_col, 14); // Ne doit PAS valoir 15 (out of bounds array)
    }

    // =========================================================================
    // 4. TESTS DES COLLIDERS ANORMAUX
    // =========================================================================

    #[test]
    fn test_zero_size_collider() {
        let grid = create_test_grid();
        let pos = Position { x: 200.0, y: 200.0 };
        let col = Collider { w: 0.0, h: 0.0 };

        let bounds = grid.bounds(&pos, &col);
        assert_eq!(bounds, Some((1, 1, 1, 1)));
    }

    #[test]
    fn test_huge_collider_covering_entire_arena() {
        let grid = create_test_grid();
        let pos = Position { x: -100.0, y: -100.0 };
        let col = Collider { w: 5000.0, h: 5000.0 };

        let bounds = grid.bounds(&pos, &col);
        assert_eq!(bounds, Some((0, 14, 0, 8)));
    }

    // =========================================================================
    // 5. TESTS D'INSERTION, RECHERCHE & NETTOYAGE (QUERY & CLEAR)
    // =========================================================================

    #[test]
    fn test_insert_and_query_single_entity() {
        let mut grid = create_test_grid();
        let pos = Position { x: 100.0, y: 100.0 };
        let col = Collider { w: 20.0, h: 20.0 };
        let entity_id = 42;

        grid.clear();
        grid.insert(entity_id, &pos, &col);
        grid.build();

        let mut candidates = Vec::new();
        grid.query(&pos, &col, &mut candidates);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], 42);
    }

    #[test]
    fn test_query_empty_cell_returns_nothing() {
        let mut grid = create_test_grid();
        let pos1 = Position { x: 10.0, y: 10.0 };
        let col1 = Collider { w: 10.0, h: 10.0 };

        let pos_far = Position { x: 1500.0, y: 800.0 };
        let col_far = Collider { w: 10.0, h: 10.0 };

        grid.clear();
        grid.insert(1, &pos1, &col1);
        grid.build();

        let mut candidates = Vec::new();
        grid.query(&pos_far, &col_far, &mut candidates);

        assert!(candidates.is_empty(), "La zone éloignée ne doit trouver aucune entité");
    }

    #[test]
    fn test_clear_resets_grid_state_completely() {
        let mut grid = create_test_grid();
        let pos = Position { x: 100.0, y: 100.0 };
        let col = Collider { w: 20.0, h: 20.0 };

        // Frame 1 : On insère l'entité 99
        grid.clear();
        grid.insert(99, &pos, &col);
        grid.build();

        // Frame 2 : On vide la grille sans rien insérer
        grid.clear();
        grid.build();

        let mut candidates = Vec::new();
        grid.query(&pos, &col, &mut candidates);

        assert!(
            candidates.is_empty(),
            "Après un clear(), la grille ne doit conserver AUCUNE ancienne donnée"
        );
    }

    #[test]
    fn test_no_out_of_bounds_panic_on_query_out_of_arena() {
        let mut grid = create_test_grid();
        let pos_outside = Position { x: -500.0, y: -500.0 };
        let col = Collider { w: 100.0, h: 100.0 };

        grid.clear();
        grid.build();

        let mut candidates = Vec::new();
        // Ne doit PAS crasher/paniquer avec un IndexOutOfBounds
        grid.query(&pos_outside, &col, &mut candidates);
        assert!(candidates.is_empty());
    }

    #[test]
fn test_two_entities_same_cell_find_each_other() {
    let mut grid = create_test_grid();
    let pos_a = Position { x: 50.0, y: 50.0 };
    let pos_b = Position { x: 70.0, y: 70.0 };
    let col = Collider { w: 20.0, h: 20.0 };

    grid.clear();
    grid.insert(0, &pos_a, &col);
    grid.insert(1, &pos_b, &col);
    grid.build();

    let mut candidates = Vec::new();
    grid.query(&pos_a, &col, &mut candidates);
    assert!(candidates.contains(&1), "A doit trouver B dans la même cellule");

    candidates.clear();
    grid.query(&pos_b, &col, &mut candidates);
    assert!(candidates.contains(&0), "B doit trouver A dans la même cellule");
}

#[test]
fn test_entity_straddling_cells_found_from_both_sides() {
    let mut grid = create_test_grid();
    // Entité à cheval entre cellule 0 et 1 en X
    let pos_straddle = Position { x: 120.0, y: 50.0 };
    let col_straddle = Collider { w: 40.0, h: 20.0 }; // s'étend jusqu'à 160.0 -> cellule 1

    let pos_left = Position { x: 50.0, y: 50.0 };
    let pos_right = Position { x: 150.0, y: 50.0 };
    let col_small = Collider { w: 10.0, h: 10.0 };

    grid.clear();
    grid.insert(0, &pos_straddle, &col_straddle);
    grid.build();

    let mut candidates = Vec::new();
    grid.query(&pos_left, &col_small, &mut candidates);
    assert!(candidates.contains(&0), "L'entité à cheval doit être trouvée depuis la cellule gauche");

    candidates.clear();
    grid.query(&pos_right, &col_small, &mut candidates);
    assert!(candidates.contains(&0), "L'entité à cheval doit être trouvée depuis la cellule droite");
}

#[test]
fn test_multiple_frames_no_ghost_entities() {
    let mut grid = create_test_grid();
    let pos_a = Position { x: 50.0, y: 50.0 };
    let pos_b = Position { x: 200.0, y: 200.0 };
    let col = Collider { w: 20.0, h: 20.0 };

    // Frame 1 : entité 0
    grid.clear();
    grid.insert(0, &pos_a, &col);
    grid.build();

    // Frame 2 : entité 1 seulement
    grid.clear();
    grid.insert(1, &pos_b, &col);
    grid.build();

    let mut candidates = Vec::new();
    grid.query(&pos_b, &col, &mut candidates);

    assert!(candidates.contains(&1), "Entité 1 doit être trouvée");
    assert!(!candidates.contains(&0), "Entité 0 (frame précédente) ne doit pas apparaître");
}
}
