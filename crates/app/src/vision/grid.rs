//! Fit a 3×3 cell grid to a set of detected sticker centers.
//!
//! Per-frame sticker detection is partial (only some cells of a face survive as
//! clean blobs), so we recover the full face by fitting a grid: estimate the two
//! basis vectors of the lattice, snap the detected cells to integer `(col, row)`
//! indices, least-squares fit the grid origin + basis, then predict all nine
//! cell centers — including the ones detection missed. The predicted centers
//! drive color sampling.

use super::detect::StickerBox;

/// A point in frame pixel coordinates.
pub type Point = (f32, f32);

fn center(b: StickerBox) -> Point {
    ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0)
}

fn dist(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Fit a 3×3 grid to sticker centers and return the nine predicted cell centers
/// in row-major order, or `None` if the points don't form a plausible grid.
#[must_use]
pub fn fit_grid(boxes: &[StickerBox]) -> Option<[Point; 9]> {
    if boxes.len() < 4 {
        return None;
    }
    let pts: Vec<Point> = boxes.iter().map(|&b| center(b)).collect();

    let pitch = median_nn(&pts)?;

    // Basis vectors from adjacency vectors (~one pitch long), canonicalized to
    // the right half-plane and split into "horizontal-ish" u and "vertical" v.
    let mut adj = Vec::new();
    for (i, &p) in pts.iter().enumerate() {
        for (j, &q) in pts.iter().enumerate() {
            if i == j {
                continue;
            }
            let d = (q.0 - p.0, q.1 - p.1);
            let len = d.0.hypot(d.1);
            if len > pitch * 0.6 && len < pitch * 1.5 {
                // Canonicalize to one half-plane so opposite directions of the
                // same edge don't cancel (verticals have dx == 0).
                let flip = d.0 < 0.0 || (d.0.abs() < 1e-3 && d.1 < 0.0);
                adj.push(if flip { (-d.0, -d.1) } else { d });
            }
        }
    }
    let us: Vec<Point> = adj
        .iter()
        .copied()
        .filter(|d| d.0.abs() >= d.1.abs())
        .collect();
    let vs: Vec<Point> = adj
        .iter()
        .copied()
        .filter(|d| d.0.abs() < d.1.abs())
        .collect();
    let u = mean(&us)?;
    let v = mean(&vs)?;

    // Assign integer (col, row) indices via the inverse basis, from a reference.
    let det = u.0 * v.1 - u.1 * v.0;
    if det.abs() < 1.0 {
        return None;
    }
    let reference = pts[0];
    let mut idx: Vec<(i32, i32)> = pts
        .iter()
        .map(|&p| {
            let (dx, dy) = (p.0 - reference.0, p.1 - reference.1);
            let col = (v.1 * dx - v.0 * dy) / det;
            let row = (-u.1 * dx + u.0 * dy) / det;
            (col.round() as i32, row.round() as i32)
        })
        .collect();
    let min_c = idx.iter().map(|c| c.0).min()?;
    let min_r = idx.iter().map(|c| c.1).min()?;
    for c in &mut idx {
        c.0 -= min_c;
        c.1 -= min_r;
    }
    if idx
        .iter()
        .any(|&(c, r)| !(0..=2).contains(&c) || !(0..=2).contains(&r))
    {
        return None;
    }

    // Least-squares fit origin + basis for x and y independently, then predict.
    let (ox, ux, vx) = lstsq(&idx, pts.iter().map(|p| p.0))?;
    let (oy, uy, vy) = lstsq(&idx, pts.iter().map(|p| p.1))?;
    let mut out = [(0.0, 0.0); 9];
    for row in 0..3 {
        for col in 0..3 {
            let (c, r) = (col as f32, row as f32);
            out[row * 3 + col] = (ox + c * ux + r * vx, oy + c * uy + r * vy);
        }
    }
    Some(out)
}

/// Median nearest-neighbor distance among points (the cell pitch).
fn median_nn(pts: &[Point]) -> Option<f32> {
    let mut nn: Vec<f32> = pts
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            pts.iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, &q)| dist(p, q))
                .fold(f32::MAX, f32::min)
        })
        .collect();
    nn.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let m = nn[nn.len() / 2];
    (m > 1.0).then_some(m)
}

/// Mean of a set of vectors.
fn mean(v: &[Point]) -> Option<Point> {
    if v.is_empty() {
        return None;
    }
    let n = v.len() as f32;
    Some((
        v.iter().map(|p| p.0).sum::<f32>() / n,
        v.iter().map(|p| p.1).sum::<f32>() / n,
    ))
}

/// Least-squares fit of `t ≈ o + col·u + row·v` over the indexed points.
/// Returns `(o, u, v)`.
fn lstsq(idx: &[(i32, i32)], targets: impl Iterator<Item = f32>) -> Option<(f32, f32, f32)> {
    let (mut s1, mut si, mut sj) = (0.0f32, 0.0f32, 0.0f32);
    let (mut sii, mut sjj, mut sij) = (0.0f32, 0.0f32, 0.0f32);
    let (mut st, mut sit, mut sjt) = (0.0f32, 0.0f32, 0.0f32);
    for (&(ci, ri), t) in idx.iter().zip(targets) {
        let (i, j) = (ci as f32, ri as f32);
        s1 += 1.0;
        si += i;
        sj += j;
        sii += i * i;
        sjj += j * j;
        sij += i * j;
        st += t;
        sit += i * t;
        sjt += j * t;
    }
    // Solve the 3×3 normal equations by Cramer's rule.
    let m = [[s1, si, sj], [si, sii, sij], [sj, sij, sjj]];
    let b = [st, sit, sjt];
    solve3(m, b)
}

/// Solve a 3×3 linear system `M x = b` via Cramer's rule; `None` if singular.
fn solve3(m: [[f32; 3]; 3], b: [f32; 3]) -> Option<(f32, f32, f32)> {
    let det = det3(m);
    if det.abs() < 1e-6 {
        return None;
    }
    let col = |k: usize| {
        let mut mk = m;
        for r in 0..3 {
            mk[r][k] = b[r];
        }
        det3(mk) / det
    };
    Some((col(0), col(1), col(2)))
}

fn det3(m: [[f32; 3]; 3]) -> f32 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clean, complete, axis-aligned grid is recovered exactly.
    #[test]
    fn recovers_full_axis_aligned_grid() {
        let mut boxes = Vec::new();
        for row in 0..3 {
            for col in 0..3 {
                let (cx, cy) = (100.0 + col as f32 * 50.0, 200.0 + row as f32 * 50.0);
                boxes.push((cx - 15.0, cy - 15.0, cx + 15.0, cy + 15.0));
            }
        }
        let grid = fit_grid(&boxes).expect("grid");
        // Center cell (index 4) should sit at the middle sticker center.
        assert!((grid[4].0 - 150.0).abs() < 2.0);
        assert!((grid[4].1 - 250.0).abs() < 2.0);
    }

    /// A partial grid (missing cells) still predicts all nine centers.
    #[test]
    fn fills_missing_cells() {
        // Only 5 of 9 cells detected, but edge-adjacent (as real clusters are)
        // so the pitch is an edge, not a diagonal.
        let present = [(0, 0), (1, 0), (2, 0), (1, 1), (1, 2)];
        let boxes: Vec<StickerBox> = present
            .iter()
            .map(|&(col, row)| {
                let (cx, cy) = (100.0 + col as f32 * 50.0, 200.0 + row as f32 * 50.0);
                (cx - 15.0, cy - 15.0, cx + 15.0, cy + 15.0)
            })
            .collect();
        let grid = fit_grid(&boxes).expect("grid");
        // Predicted center cell lands on the true middle even though it was seen.
        assert!((grid[4].0 - 150.0).abs() < 3.0);
        assert!((grid[4].1 - 250.0).abs() < 3.0);
        // Bottom-right predicted correctly.
        assert!((grid[8].0 - 200.0).abs() < 3.0);
        assert!((grid[8].1 - 300.0).abs() < 3.0);
    }

    #[test]
    fn rejects_too_few_points() {
        let boxes = vec![(0.0, 0.0, 10.0, 10.0), (50.0, 0.0, 60.0, 10.0)];
        assert!(fit_grid(&boxes).is_none());
    }
}
