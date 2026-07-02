use crate::sexpr::Span;

#[derive(Debug, Clone)]
pub struct GridPos {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct GridLayout {
    pub cells: Vec<GridPos>,
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

    // (line, char_col) per key
    let mut positions: Vec<(usize, usize)> = key_spans
        .iter()
        .map(|s| byte_to_line_col(source, &line_starts, s.start))
        .collect();

    // normalize rows to start at 0
    let min_row = positions.iter().map(|(r, _)| *r).min().unwrap_or(0);
    for pos in positions.iter_mut() {
        pos.0 -= min_row;
    }

    // group by row; within each row, sort by char_col and assign consecutive col indices
    let mut cells_by_index: Vec<Option<GridPos>> = vec![None; key_spans.len()];
    let mut row_buckets: std::collections::BTreeMap<usize, Vec<(usize, usize)>> =
        std::collections::BTreeMap::new();
    for (i, (r, c)) in positions.iter().enumerate() {
        row_buckets.entry(*r).or_default().push((*c, i));
    }

    let mut n_cols = 0usize;
    for (_, mut bucket) in row_buckets {
        bucket.sort_by_key(|(c, _)| *c);
        n_cols = n_cols.max(bucket.len());
        for (col, &(_, idx)) in bucket.iter().enumerate() {
            cells_by_index[idx] = Some(GridPos {
                row: positions[idx].0,
                col,
            });
        }
    }
    let cells: Vec<GridPos> = cells_by_index.into_iter().map(Option::unwrap).collect();

    let n_rows = cells
        .iter()
        .map(|c| c.row)
        .max()
        .map(|r| r + 1)
        .unwrap_or(0);

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
