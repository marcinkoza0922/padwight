//! The outlines a filter's picture is cut to.

/// What a filter's layer is cut to.
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    /// A box with corners this many pixels round.
    Rounded(f32),
    /// A radial menu's wedge: the part of the ring between radii `inner` and `outer` around
    /// `center` (from the layer's top left corner), from `start` to `end` degrees clockwise from
    /// the top, less half of a `gap` pixels wide line along each edge, where the next wedge starts.
    Arc { center: (f32, f32), inner: f32, outer: f32, start: f32, end: f32, gap: f32 },
}

impl Shape {
    /// Tells the shapes apart in the kept pictures.
    pub fn key(self) -> u64 {
        let bits: Vec<u32> = match self {
            Shape::Rounded(round) => vec![round.to_bits()],
            Shape::Arc { center, inner, outer, start, end, gap } => [center.0, center.1, inner, outer, start, end, gap].map(f32::to_bits).to_vec(),
        };
        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(&bits, &mut hasher);
        std::hash::Hasher::finish(&hasher)
    }

    /// The `w` by `h` pixels with what's outside the shape made clear.
    pub fn cut(self, pixels: Vec<u8>, w: usize, h: usize) -> Vec<u8> {
        match self {
            Shape::Rounded(round) => rounded(pixels, w, h, round),
            Shape::Arc { .. } => arc(pixels, w, self),
        }
    }
}

/// The pixels with what's outside the wedge made clear, the edges softened.
fn arc(mut pixels: Vec<u8>, w: usize, shape: Shape) -> Vec<u8> {
    let Shape::Arc { center: (cx, cy), inner, outer, start, end, gap } = shape else { return pixels };
    let span = end - start;
    // How far a point `d` from the middle lies from an edge `degrees` round from it, either way.
    let off_edge = |d: f32, degrees: f32| if degrees < 90.0 { d * degrees.to_radians().sin() } else { d };
    for (i, alpha) in pixels.iter_mut().skip(3).step_by(4).enumerate() {
        let (dx, dy) = ((i % w) as f32 + 0.5 - cx, (i / w) as f32 + 0.5 - cy);
        let d = dx.hypot(dy);
        let mut cover = (d - inner).min(outer - d) + 0.5;
        if span < 360.0 {
            let along = (dx.atan2(-dy).to_degrees() - start).rem_euclid(360.0);
            cover = if along > span { 0.0 } else { cover.min(off_edge(d, along).min(off_edge(d, span - along)) - gap / 2.0 + 0.5) };
        }
        *alpha = (f32::from(*alpha) * cover.clamp(0.0, 1.0)) as u8;
    }
    pixels
}

/// The pixels with what's outside corners `round` pixels round made clear, the edge softened.
fn rounded(mut pixels: Vec<u8>, w: usize, h: usize, round: f32) -> Vec<u8> {
    let r = round.min(w.min(h) as f32 / 2.0);
    if r < 1.0 {
        return pixels;
    }
    let reach = r.ceil() as usize;
    for y in (0..reach).chain(h.saturating_sub(reach).max(reach)..h) {
        for x in (0..reach).chain(w.saturating_sub(reach).max(reach)..w) {
            // How far the pixel's middle lies outside the corner's circle.
            let dx = (r - (x as f32 + 0.5)).max(x as f32 + 0.5 - (w as f32 - r));
            let dy = (r - (y as f32 + 0.5)).max(y as f32 + 0.5 - (h as f32 - r));
            if dx <= 0.0 || dy <= 0.0 {
                continue;
            }
            let cover = (r + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let alpha = &mut pixels[(y * w + x) * 4 + 3];
            *alpha = (f32::from(*alpha) * cover) as u8;
        }
    }
    pixels
}
