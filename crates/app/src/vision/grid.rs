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

/// A detected cell assigned to a grid: point index, column, row.
type Assignment = (usize, i32, i32);

/// Fit up to three 3×3 grids (one per visible face) to the detected cells.
///
/// Uses deterministic RANSAC: enumerate candidate basis vectors from pairs of
/// nearby points, score each by how many points fall on integer grid positions
/// (within tolerance), keep the best-supported grid, remove its inliers, and
/// repeat. Robust to a corner view's multiple faces and to the diagonal-vs-edge
/// ambiguity that defeats simple averaging.
#[must_use]
pub fn fit_faces(boxes: &[StickerBox]) -> Vec<[Point; 9]> {
    let pts: Vec<Point> = boxes.iter().map(|&b| center(b)).collect();
    let Some(pitch) = median_nn(&pts) else {
        return Vec::new();
    };
    let mut used = vec![false; pts.len()];
    let mut faces = Vec::new();
    for _ in 0..3 {
        let avail: Vec<usize> = (0..pts.len()).filter(|&i| !used[i]).collect();
        if avail.len() < 4 {
            break;
        }
        let Some(inliers) = best_grid(&pts, &avail, pitch) else {
            break;
        };
        if inliers.len() < 4 {
            break;
        }
        let idx: Vec<(i32, i32)> = inliers.iter().map(|&(_, c, r)| (c, r)).collect();
        let ins: Vec<Point> = inliers.iter().map(|&(i, _, _)| pts[i]).collect();
        if let Some(grid) = affine_predict(&idx, &ins) {
            faces.push(grid);
        }
        for &(i, _, _) in &inliers {
            used[i] = true;
        }
    }
    faces
}

/// Find the basis (from point pairs) that puts the most points on a 3×3 grid.
fn best_grid(pts: &[Point], avail: &[usize], pitch: f32) -> Option<Vec<Assignment>> {
    let mut best: Vec<Assignment> = Vec::new();
    for &a in avail {
        for &b in avail {
            if b == a {
                continue;
            }
            let u = (pts[b].0 - pts[a].0, pts[b].1 - pts[a].1);
            let lu = u.0.hypot(u.1);
            if lu < pitch * 0.7 || lu > pitch * 1.4 {
                continue;
            }
            for &c in avail {
                if c == a || c == b {
                    continue;
                }
                let v = (pts[c].0 - pts[a].0, pts[c].1 - pts[a].1);
                let lv = v.0.hypot(v.1);
                if lv < pitch * 0.7 || lv > pitch * 1.4 {
                    continue;
                }
                let det = u.0 * v.1 - u.1 * v.0;
                if det.abs() < pitch * pitch * 0.3 {
                    continue; // u, v too parallel
                }
                if let Some(cand) = score_basis(pts, avail, pts[a], u, v, det, pitch) {
                    if cand.len() > best.len() {
                        best = cand;
                    }
                }
            }
        }
    }
    (!best.is_empty()).then_some(best)
}

/// Assign every available point to `(col, row)` under the basis and keep those
/// with small reprojection error; `None` if they don't fit a 3×3 window.
fn score_basis(
    pts: &[Point],
    avail: &[usize],
    origin: Point,
    u: Point,
    v: Point,
    det: f32,
    pitch: f32,
) -> Option<Vec<Assignment>> {
    let mut cand: Vec<Assignment> = Vec::new();
    for &p in avail {
        let (dx, dy) = (pts[p].0 - origin.0, pts[p].1 - origin.1);
        let col = ((v.1 * dx - v.0 * dy) / det).round();
        let row = ((-u.1 * dx + u.0 * dy) / det).round();
        let reproj = (
            origin.0 + col * u.0 + row * v.0,
            origin.1 + col * u.1 + row * v.1,
        );
        if dist(reproj, pts[p]) < pitch * 0.35 {
            cand.push((p, col as i32, row as i32));
        }
    }
    // Keep the densest 3×3 window of indices (handles extra points that lie on
    // the same extended lattice, e.g. a second co-aligned face).
    let mut best_win: Vec<Assignment> = Vec::new();
    for &(_, c0, r0) in &cand {
        let win: Vec<Assignment> = cand
            .iter()
            .copied()
            .filter(|&(_, c, r)| (c0..c0 + 3).contains(&c) && (r0..r0 + 3).contains(&r))
            .collect();
        if win.len() > best_win.len() {
            best_win = win;
        }
    }
    let min_c = best_win.iter().map(|c| c.1).min()?;
    let min_r = best_win.iter().map(|c| c.2).min()?;
    for c in &mut best_win {
        c.1 -= min_c;
        c.2 -= min_r;
    }
    (best_win.len() >= 4).then_some(best_win)
}

/// Least-squares affine fit from indexed points, then predict all nine centers
/// in canonical image row-major order (top-left first, columns increasing
/// rightward, rows increasing downward) so downstream classification sees a
/// consistent orientation regardless of the fitted basis sign/axis.
fn affine_predict(idx: &[(i32, i32)], pts: &[Point]) -> Option<[Point; 9]> {
    let (ox, ux, vx) = lstsq(idx, pts.iter().map(|p| p.0))?;
    let (oy, uy, vy) = lstsq(idx, pts.iter().map(|p| p.1))?;
    let mut o = (ox, oy);
    let mut u = (ux, uy); // column step
    let mut v = (vx, vy); // row step

    // Column axis should be the more-horizontal of the two.
    if u.0.abs() < v.0.abs() {
        std::mem::swap(&mut u, &mut v);
    }
    // Columns increase rightward; if not, reverse them (shift origin to col 2).
    if u.0 < 0.0 {
        o = (o.0 + 2.0 * u.0, o.1 + 2.0 * u.1);
        u = (-u.0, -u.1);
    }
    // Rows increase downward; if not, reverse them.
    if v.1 < 0.0 {
        o = (o.0 + 2.0 * v.0, o.1 + 2.0 * v.1);
        v = (-v.0, -v.1);
    }

    let mut out = [(0.0, 0.0); 9];
    for row in 0..3 {
        for col in 0..3 {
            let (c, r) = (col as f32, row as f32);
            out[row * 3 + col] = (o.0 + c * u.0 + r * v.0, o.1 + c * u.1 + r * v.1);
        }
    }
    Some(out)
}

/// Median nearest-neighbor distance among points (the cell pitch).
fn median_nn(pts: &[Point]) -> Option<f32> {
    if pts.len() < 2 {
        return None; // no neighbors -> no pitch
    }
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

    /// Zero or one detected cell must not panic (live frames often have few).
    #[test]
    fn handles_zero_or_one_point() {
        assert!(fit_faces(&[]).is_empty());
        assert!(fit_grid(&[]).is_none());
        let one = vec![(0.0, 0.0, 10.0, 10.0)];
        assert!(fit_faces(&one).is_empty());
        assert!(fit_grid(&one).is_none());
    }

    fn cell(cx: f32, cy: f32) -> StickerBox {
        (cx - 15.0, cy - 15.0, cx + 15.0, cy + 15.0)
    }

    /// Two separated full faces are found as two distinct grids.
    #[test]
    fn fit_faces_finds_two_faces() {
        let mut boxes = Vec::new();
        // Face A around (100,200), face B far away around (600,200).
        for (ox, oy) in [(100.0, 200.0), (600.0, 200.0)] {
            for row in 0..3 {
                for col in 0..3 {
                    boxes.push(cell(ox + col as f32 * 50.0, oy + row as f32 * 50.0));
                }
            }
        }
        let faces = fit_faces(&boxes);
        assert_eq!(faces.len(), 2, "expected two faces, got {}", faces.len());
    }

    /// A single partial face is still recovered from RANSAC.
    #[test]
    fn fit_faces_recovers_partial_face() {
        let present = [(0, 0), (1, 0), (2, 0), (1, 1), (1, 2), (0, 2)];
        let boxes: Vec<StickerBox> = present
            .iter()
            .map(|&(c, r)| cell(100.0 + c as f32 * 50.0, 200.0 + r as f32 * 50.0))
            .collect();
        let faces = fit_faces(&boxes);
        assert_eq!(faces.len(), 1);
        // Center cell predicted at the true middle.
        assert!((faces[0][4].0 - 150.0).abs() < 4.0);
        assert!((faces[0][4].1 - 250.0).abs() < 4.0);
    }
}
