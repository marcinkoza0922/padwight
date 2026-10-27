//! The marks a frame draws at a panel's four corners, as SVG and without any drawing library.
//! Each kind's mark is drawn once, for the top-left corner in a unit square (the corner at 0,0,
//! its arms running right along the top edge and down the left one), and mirrored onto the
//! other corners.

use std::fmt::Write;

use crate::config::{Frame, FrameKind};

/// One stroke or fill of a mark, in the unit square. Paths take absolute `M`, `L`, `C` and `Z`
/// commands with `x,y` points.
enum Part {
    Line(&'static str),
    Fill(&'static str),
    /// A circle's outline: its center and radius.
    Ring(f32, f32, f32),
    /// A filled circle.
    Dot(f32, f32, f32),
    /// A stretch of a line where it passes over another: drawn again on top, with the lines
    /// under it cut away either side, so they show a gap.
    Over(&'static str),
}

use Part::{Dot, Fill, Line, Over, Ring};

/// A kind's mark: `half` runs along the top arm and is mirrored across the diagonal onto the
/// left one, `whole` is drawn as it is.
struct Design {
    half: &'static [Part],
    whole: &'static [Part],
    /// Keeps the strokes inside the mark's box. Brackets and chamfers sit on the panel's edge,
    /// half of their line cut off by it.
    inset: bool,
    round: bool,
}

const BRACKETS: Design = Design { half: &[], whole: &[Line("M1,0 L0,0 L0,1")], inset: false, round: false };

const CHAMFER: Design = Design { half: &[], whole: &[Line("M1,0 L0,1")], inset: false, round: true };

const ART_DECO: Design = Design {
        half: &[
            Line("M1,0 L0.36,0 L0.36,0.18 L0.18,0.18"),
            Line("M0.5,0.1 L0.94,0.1"),
            Line("M0.5,0.2 L0.74,0.2"),
            Line("M0.18,0.18 L0.45,0.252"),
            Line("M0.18,0.18 L0.457,0.34"),
        ],
        whole: &[Line("M0.18,0.18 L0.4,0.4"), Fill("M0.45,0.39 L0.51,0.45 L0.45,0.51 L0.39,0.45 Z")],
        inset: true,
        round: false,
};

const ART_NOUVEAU: Design = Design {
        half: &[
            Line("M1,0.04 C0.6,0.04 0.22,0.02 0.12,0.12"),
            Line("M0.62,0.045 C0.5,0.12 0.48,0.3 0.62,0.3 C0.72,0.3 0.72,0.18 0.63,0.19"),
            Line("M0.88,0.04 C0.84,0.1 0.86,0.16 0.93,0.16"),
        ],
        whole: &[Fill("M0.18,0.18 C0.3,0.2 0.42,0.3 0.48,0.48 C0.3,0.42 0.2,0.3 0.18,0.18 Z"), Dot(0.53, 0.53, 0.03)],
        inset: true,
        round: true,
};

const RUNIC: Design = Design {
        half: &[
            Line("M1,0 L0,0"),
            Line("M1,0.36 L0.36,0.36"),
            // Algiz, thurisaz and tiwaz.
            Line("M0.52,0.3 L0.52,0.06"),
            Line("M0.45,0.06 L0.52,0.14 L0.59,0.06"),
            Line("M0.68,0.06 L0.68,0.3"),
            Line("M0.68,0.11 L0.76,0.18 L0.68,0.25"),
            Line("M0.86,0.3 L0.86,0.06"),
            Line("M0.79,0.13 L0.86,0.06 L0.93,0.13"),
        ],
        whole: &[Line("M0.18,0.06 L0.3,0.18 L0.18,0.3 L0.06,0.18 Z")],
        inset: true,
        round: false,
};

const GOTHIC: Design = Design {
        half: &[Line("M1,0 L0,0"), Line("M0.7,0 C0.66,0.18 0.5,0.3 0.28,0.28"), Line("M0.74,0.1 L1,0.1"), Dot(0.7, 0.0, 0.035)],
        whole: &[Ring(0.13, 0.13, 0.07), Dot(0.13, 0.13, 0.025)],
        inset: true,
        round: true,
};

const BAROQUE: Design = Design {
        half: &[
            Line("M1,0.05 L0.5,0.05 C0.32,0.05 0.26,0.2 0.36,0.26 C0.44,0.3 0.48,0.2 0.41,0.17"),
            Fill("M0.6,0.06 C0.66,0.16 0.76,0.18 0.84,0.12 C0.76,0.1 0.68,0.08 0.6,0.06 Z"),
        ],
        whole: &[Fill("M0.16,0.16 C0.26,0.18 0.32,0.26 0.3,0.3 C0.26,0.32 0.18,0.26 0.16,0.16 Z"), Dot(0.09, 0.09, 0.045)],
        inset: true,
        round: true,
};

const CIRCUIT: Design = Design {
        half: &[
            Line("M1,0.06 L0.3,0.06 L0.18,0.18"),
            Line("M0.78,0.18 L0.42,0.18 L0.3,0.3"),
            Ring(0.82, 0.18, 0.04),
            Line("M0.5,0.3 L0.64,0.3"),
            Ring(0.68, 0.3, 0.035),
        ],
        whole: &[Ring(0.42, 0.42, 0.04)],
        inset: true,
        round: true,
};

const RIVETED: Design = Design {
        half: &[Line("M1,0 L0,0"), Line("M1,0.24 L0.24,0.24"), Dot(0.5, 0.12, 0.04), Dot(0.84, 0.12, 0.04)],
        whole: &[Dot(0.12, 0.12, 0.045)],
        inset: true,
        round: false,
};

/// Two strands twisting along each edge, swapping sides through a ring at the corner, every
/// crossing alternately over and under.
const CELTIC: Design = Design {
    half: &[
        Line("M0.455,0.04 C0.511,0.04 0.554,0.2 0.61,0.2 C0.666,0.2 0.709,0.04 0.765,0.04 C0.821,0.04 0.864,0.2 0.92,0.2"),
        Line("M0.455,0.2 C0.511,0.2 0.554,0.04 0.61,0.04 C0.666,0.04 0.709,0.2 0.765,0.2 C0.821,0.2 0.864,0.04 0.92,0.04"),
        Line("M0.92,0.04 C0.964,0.04 1,0.076 1,0.12 C1,0.164 0.964,0.2 0.92,0.2"),
    ],
    // The crossings aren't mirror images: the strand that goes over along the top goes under
    // along the left, so the weave keeps alternating round the corner.
    whole: &[
        Line("M0.455,0.04 C0.2,0.04 0.2,0.2 0.2,0.455"),
        Line("M0.455,0.2 C0.2,0.2 0.04,0.2 0.04,0.455"),
        Ring(0.213, 0.213, 0.11),
        Over("M0.509,0.083 C0.525,0.106 0.54,0.134 0.556,0.157"),
        Over("M0.157,0.509 C0.134,0.525 0.106,0.54 0.083,0.556"),
        Over("M0.664,0.083 C0.68,0.106 0.695,0.134 0.711,0.157"),
        Over("M0.157,0.664 C0.134,0.68 0.106,0.695 0.083,0.711"),
        Over("M0.819,0.083 C0.835,0.106 0.85,0.134 0.866,0.157"),
        Over("M0.157,0.819 C0.134,0.835 0.106,0.85 0.083,0.866"),
        Over("M0.222,0.178 C0.215,0.2 0.21,0.225 0.207,0.252"),
        Over("M0.37,0.2 C0.337,0.201 0.307,0.202 0.279,0.204"),
        Over("M0.138,0.239 C0.119,0.249 0.103,0.261 0.089,0.278"),
        Over("M0.194,0.105 C0.237,0.097 0.28,0.115 0.304,0.151"),
        Over("M0.263,0.311 C0.224,0.331 0.178,0.326 0.144,0.299"),
    ],
    inset: true,
    round: true,
};

/// A running key between two rails along each edge, a boxed square at the corner.
const GREEK_KEY: Design = Design {
    half: &[
        Line("M1,0 L0,0"),
        Line("M1,0.24 L0.24,0.24"),
        Line("M0.28,0.2 L0.28,0.04 L0.44,0.04 L0.44,0.12 L0.36,0.12 L0.36,0.2 L0.52,0.2 L0.52,0.04 L0.68,0.04 L0.68,0.12 L0.6,0.12 L0.6,0.2 L0.76,0.2 L0.76,0.04 L0.92,0.04 L0.92,0.12 L0.84,0.12 L0.84,0.2 L1,0.2"),
    ],
    whole: &[Line("M0.06,0.06 L0.18,0.06 L0.18,0.18 L0.06,0.18 Z"), Fill("M0.1,0.1 L0.14,0.1 L0.14,0.14 L0.1,0.14 Z")],
    inset: true,
    round: false,
};

/// A frieze of lotus flowers and buds between two rails along each edge, a sun disc boxed at
/// the corner.
const EGYPTIAN: Design = Design {
    half: &[
        Line("M1,0 L0,0"),
        Line("M1,0.26 L0.26,0.26"),
        Line("M0.26,0 L0.26,0.26"),
        // Flower, bud, flower, bud, each rising from the inner rail toward the panel's edge.
        Fill("M0.4,0.26 C0.38,0.18 0.325,0.12 0.33,0.03 L0.37,0.1 L0.4,0.02 L0.43,0.1 L0.47,0.03 C0.475,0.12 0.42,0.18 0.4,0.26 Z"),
        Fill("M0.555,0.26 C0.505,0.17 0.515,0.08 0.555,0.04 C0.595,0.08 0.605,0.17 0.555,0.26 Z"),
        Fill("M0.71,0.26 C0.69,0.18 0.635,0.12 0.64,0.03 L0.68,0.1 L0.71,0.02 L0.74,0.1 L0.78,0.03 C0.785,0.12 0.73,0.18 0.71,0.26 Z"),
        Fill("M0.865,0.26 C0.815,0.17 0.825,0.08 0.865,0.04 C0.905,0.08 0.915,0.17 0.865,0.26 Z"),
    ],
    whole: &[Ring(0.13, 0.13, 0.08), Dot(0.13, 0.13, 0.04)],
    inset: true,
    round: false,
};

/// A five-petalled blossom at the corner, a leafy vine running from it along each edge to a bud.
const FLORAL: Design = Design {
    half: &[
        Line("M0.34,0.12 C0.44,0.05 0.52,0.17 0.64,0.12 C0.76,0.07 0.82,0.14 0.92,0.1"),
        Fill("M0.47,0.105 C0.47,0.205 0.57,0.205 0.57,0.205 C0.57,0.205 0.57,0.105 0.47,0.105 Z"),
        Fill("M0.66,0.11 C0.755,0.115 0.76,0.02 0.76,0.02 C0.76,0.02 0.665,0.015 0.66,0.11 Z"),
        Dot(0.94, 0.095, 0.035),
    ],
    whole: &[
        Fill("M0.235,0.235 C0.235,0.341 0.298,0.348 0.323,0.323 C0.348,0.298 0.341,0.235 0.235,0.235 Z"),
        Fill("M0.194,0.241 C0.093,0.274 0.106,0.337 0.137,0.353 C0.169,0.368 0.227,0.342 0.194,0.241 Z"),
        Fill("M0.175,0.205 C0.113,0.119 0.057,0.15 0.052,0.185 C0.046,0.22 0.09,0.267 0.175,0.205 Z"),
        Fill("M0.205,0.175 C0.267,0.09 0.22,0.046 0.185,0.052 C0.15,0.057 0.119,0.113 0.205,0.175 Z"),
        Fill("M0.241,0.194 C0.342,0.227 0.368,0.169 0.353,0.137 C0.337,0.106 0.274,0.093 0.241,0.194 Z"),
        Dot(0.21, 0.21, 0.03),
    ],
    inset: true,
    round: true,
};

fn design(kind: FrameKind) -> &'static Design {
    match kind {
        FrameKind::Brackets => &BRACKETS,
        FrameKind::Chamfer => &CHAMFER,
        FrameKind::ArtDeco => &ART_DECO,
        FrameKind::ArtNouveau => &ART_NOUVEAU,
        FrameKind::Runic => &RUNIC,
        FrameKind::Gothic => &GOTHIC,
        FrameKind::Baroque => &BAROQUE,
        FrameKind::Circuit => &CIRCUIT,
        FrameKind::Riveted => &RIVETED,
        FrameKind::Celtic => &CELTIC,
        FrameKind::GreekKey => &GREEK_KEY,
        FrameKind::Egyptian => &EGYPTIAN,
        FrameKind::Floral => &FLORAL,
    }
}

/// Takes the unit square's points to one corner's mark box, `reach` across.
struct Place {
    reach: f32,
    inset: f32,
    corner: (f32, f32),
    mirrored: bool,
}

impl Place {
    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        let (x, y) = if self.mirrored { (y, x) } else { (x, y) };
        let span = self.reach - 2.0 * self.inset;
        let along = |at: f32, corner: f32| {
            let d = self.inset + at * span;
            if corner == 0.0 { d } else { self.reach - d }
        };
        (along(x, self.corner.0), along(y, self.corner.1))
    }

    fn path(&self, d: &str) -> String {
        d.split_whitespace()
            .map(|token| {
                let Some((x, y)) = token.trim_start_matches(['M', 'L', 'C']).split_once(',') else { return token.to_string() };
                let command = &token[..token.len() - token.trim_start_matches(['M', 'L', 'C']).len()];
                let (x, y) = self.point(x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0));
                format!("{command}{x:.2},{y:.2}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn draw(&self, part: &Part, out: &mut String) {
        let ring = |x, y, r: f32| {
            let (cx, cy) = self.point(x, y);
            (cx, cy, r * (self.reach - 2.0 * self.inset))
        };
        // Writing to a String can't fail.
        let _ = match *part {
            Line(d) | Over(d) => write!(out, "<path d=\"{}\" fill=\"none\"/>", self.path(d)),
            Fill(d) => write!(out, "<path d=\"{}\" stroke=\"none\" fill=\"currentColor\"/>", self.path(d)),
            Ring(x, y, r) => {
                let (cx, cy, r) = ring(x, y, r);
                write!(out, "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" fill=\"none\"/>")
            }
            Dot(x, y, r) => {
                let (cx, cy, r) = ring(x, y, r);
                write!(out, "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{r:.2}\" stroke=\"none\" fill=\"currentColor\"/>")
            }
        };
    }
}

/// The SVG of the frame's mark at `corner`, at scale `s`, in `ink` and the frame's opacity
/// (times `fade`).
pub fn markup(f: &Frame, ink: [u8; 3], corner: (f32, f32), s: f32, fade: f32) -> String {
    let reach = f.size.max(1.0) * s;
    let width = f.width.max(0.5) * s;
    let [r, g, b] = ink;
    let ink = format!("#{r:02x}{g:02x}{b:02x}");
    let opacity = (f.opacity * fade).clamp(0.0, 1.0);
    let d = design(f.kind);
    let inset = if d.inset { width / 2.0 } else { 0.0 };
    let (mut body, mut over) = (String::new(), String::new());
    for mirrored in [false, true] {
        let place = Place { reach, inset, corner, mirrored };
        for part in d.half {
            place.draw(part, if matches!(part, Over(_)) { &mut over } else { &mut body });
        }
    }
    let place = Place { reach, inset, corner, mirrored: false };
    for part in d.whole {
        place.draw(part, if matches!(part, Over(_)) { &mut over } else { &mut body });
    }
    if !over.is_empty() {
        // The stretches that go over, three lines wide, cut the lines under them away.
        let cut = 3.0 * width;
        body = format!(
            "<mask id=\"over\" maskUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"{reach}\" height=\"{reach}\"><rect width=\"{reach}\" height=\"{reach}\" fill=\"white\"/><g stroke=\"black\" stroke-width=\"{cut}\" stroke-linecap=\"butt\">{over}</g></mask><g mask=\"url(#over)\">{body}</g>{over}"
        );
    }
    let (cap, join) = if d.round { ("round", "round") } else { ("square", "miter") };
    // One group carries the opacity, so where the strokes cross they don't darken.
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {reach} {reach}\"><g color=\"{ink}\" stroke=\"{ink}\" stroke-width=\"{width}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\" opacity=\"{opacity}\">{body}</g></svg>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_draws_well_formed_markup() {
        for kind in FrameKind::ALL {
            let frame = Frame { kind, size: 40.0, width: 2.0, color: "#ffffff".into(), opacity: 1.0 };
            let svg = markup(&frame, [0xff, 0xff, 0xff], (1.0, 1.0), 1.0, 1.0);
            assert!(resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).is_ok(), "{kind:?}: {svg}");
        }
    }

    #[test]
    fn the_far_corner_mirrors_the_near_one() {
        let place = Place { reach: 10.0, inset: 0.0, corner: (1.0, 1.0), mirrored: false };
        assert_eq!(place.path("M1,0 L0,0"), "M0.00,10.00 L10.00,10.00");
        let mirrored = Place { mirrored: true, corner: (0.0, 0.0), ..place };
        assert_eq!(mirrored.path("M1,0.5"), "M5.00,10.00");
    }
}
