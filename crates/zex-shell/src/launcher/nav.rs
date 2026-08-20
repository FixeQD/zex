//! Grid navigation math for the launcher results

/// Step the selection inside a grid: `delta = ±1` moves a cell, larger deltas move a whole row while staying in range.
pub fn grid_step(count: usize, columns: u32, sel: usize, delta: i32, wrap: bool) -> usize {
    if count == 0 {
        return 0;
    }
    let columns = columns.max(1) as usize;
    let last_row = (count - 1) / columns;
    let current = sel.min(count - 1);

    if delta.abs() <= 1 {
        let target = current as i64 + i64::from(delta);
        if wrap {
            target.rem_euclid(count as i64) as usize
        } else {
            target.clamp(0, count as i64 - 1) as usize
        }
    } else {
        let row = current / columns;
        let target_row = (row as i64 + i64::from(delta)).clamp(0, last_row as i64) as usize;
        if target_row == row {
            return current;
        }
        if delta < 0 {
            target_row * columns + current % columns
        } else {
            let last = count - target_row * columns;
            let column = (current % columns).min(last - 1);
            target_row * columns + column
        }
    }
}
