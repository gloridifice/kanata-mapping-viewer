use crate::sexpr::Span;

#[derive(Debug, Clone)]
pub struct GridCell {
    pub row: usize,
    pub col: usize,
    pub colspan: usize,
}

#[derive(Debug, Clone)]
pub struct GridLayout {
    pub cells: Vec<GridCell>,
    pub n_rows: usize,
    pub n_cols: usize,
}

pub fn compute_layout(source: &str, key_spans: &[Span]) -> GridLayout {
    if key_spans.is_empty() {
        return GridLayout {
            cells: vec![],
            n_rows: 0,
            n_cols: 0,
        };
    }

    let line_starts = compute_line_starts(source);

    // absolute (line, char_col) per key
    let mut positions: Vec<(usize, usize)> = key_spans
        .iter()
        .map(|s| byte_to_line_col(source, &line_starts, s.start))
        .collect();

    // normalize: subtract min row and min col
    let min_row = positions.iter().map(|(r, _)| *r).min().unwrap_or(0);
    let min_col = positions.iter().map(|(_, c)| *c).min().unwrap_or(0);
    for pos in positions.iter_mut() {
        pos.0 -= min_row;
        pos.1 -= min_col;
    }

    // group by row to compute colspan = next key's col - this col (last in row = 1)
    let mut cells_by_index: Vec<Option<GridCell>> = vec![None; key_spans.len()];
    let mut row_buckets: std::collections::BTreeMap<usize, Vec<(usize, usize)>> =
        std::collections::BTreeMap::new();
    for (i, (r, c)) in positions.iter().enumerate() {
        row_buckets.entry(*r).or_default().push((*c, i));
    }

    for (_, mut bucket) in row_buckets {
        bucket.sort_by_key(|(c, _)| *c);
        for j in 0..bucket.len() {
            let (col, idx) = bucket[j];
            let colspan = if j + 1 < bucket.len() {
                bucket[j + 1].0.saturating_sub(col).max(1)
            } else {
                1
            };
            cells_by_index[idx] = Some(GridCell {
                row: positions[idx].0,
                col,
                colspan,
            });
        }
    }
    let cells: Vec<GridCell> = cells_by_index.into_iter().map(Option::unwrap).collect();

    let n_rows = cells
        .iter()
        .map(|c| c.row)
        .max()
        .map(|r| r + 1)
        .unwrap_or(0);
    let n_cols = cells.iter().map(|c| c.col + c.colspan).max().unwrap_or(0);

    GridLayout {
        cells,
        n_rows,
        n_cols,
    }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn byte_to_line_col(source: &str, line_starts: &[usize], byte_pos: usize) -> (usize, usize) {
    // find line: last line_start <= byte_pos
    let line_idx = match line_starts.binary_search(&byte_pos) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };
    let line_start = line_starts[line_idx];
    // char column = number of chars in source[line_start..byte_pos]
    let col = source[line_start..byte_pos].chars().count();
    (line_idx, col)
}
