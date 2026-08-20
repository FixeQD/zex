//! Launcher widgets and navigation maths, without a display server

use zex_shell::launcher::LAUNCHER_CSS_SCSS;
use zex_shell::launcher::{GRID_COLUMNS, PINNED_PER_ROW, grid_step};

#[test]
fn stylesheet_compiles_with_grass() {
    let css = zex_core::theme::css::compile(LAUNCHER_CSS_SCSS).expect("launcher.scss compiles");
    assert!(css.contains(".launcher-panel"));
}

#[test]
fn grid_columns_and_pinned_row_match_plan() {
    assert_eq!(GRID_COLUMNS, 5);
    assert_eq!(PINNED_PER_ROW, 8);
}

#[test]
fn grid_step_moves_within_range_without_wrap() {
    let count = 23;
    assert_eq!(grid_step(count, GRID_COLUMNS, 0, 1, false), 1);
    assert_eq!(grid_step(count, GRID_COLUMNS, 22, 1, false), 22);
    assert_eq!(grid_step(count, GRID_COLUMNS, 0, -1, false), 0);
    assert_eq!(grid_step(count, GRID_COLUMNS, 5, -1, false), 4);
}

#[test]
fn grid_step_large_delta_moves_whole_rows() {
    // Down 5 rows from the first row lands on the last row, same column
    // where the column hangs (23 = 4 rows of 5 + 3), clamped inward
    assert_eq!(grid_step(23, GRID_COLUMNS, 4, 5, false), 22);
    // Moving down past the last row clamps in place
    assert_eq!(grid_step(23, GRID_COLUMNS, 21, 5, false), 21);
    // Up 5 rows keeps the column
    assert_eq!(grid_step(23, GRID_COLUMNS, 12, -5, false), 2);
    // Down from a hanging cell keeps the row
    assert_eq!(grid_step(23, GRID_COLUMNS, 22, -5, false), 2);
}

#[test]
fn grid_step_wraps_around_the_ends() {
    assert_eq!(grid_step(23, GRID_COLUMNS, 22, 1, true), 0);
    assert_eq!(grid_step(23, GRID_COLUMNS, 0, -1, true), 22);
}

#[test]
fn grid_step_ignores_empty_catalogs() {
    assert_eq!(grid_step(0, GRID_COLUMNS, 0, 1, true), 0);
}

#[test]
fn grid_step_with_one_column_is_linear() {
    let count = 4;
    assert_eq!(grid_step(count, 1, 0, 1, false), 1);
    assert_eq!(grid_step(count, 1, 3, 1, false), 3);
    assert_eq!(grid_step(count, 1, 3, 1, true), 0);
}
