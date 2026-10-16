//! The sample styles the menu scenes are drawn in: textures, filters, ornaments and frames.

use crate::config::{
    Bevel, Camo, CamoScheme, Case, Frame, FrameKind, Gradient, Grain, Ornament, OverlayStyle, Paint, Pixels, Scanlines, Shade, Stroke, Texture, Tint, Typography,
    Vignette,
};

/// A panel under a woodland camo.
pub(super) fn camouflaged() -> OverlayStyle {
    OverlayStyle { ornament: Ornament { camo: Some(Camo { scheme: CamoScheme::Woodland, opacity: 0.45, size: 60.0 }), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// A dark panel under every filter.
pub(super) fn filtered() -> OverlayStyle {
    OverlayStyle {
        ornament: Ornament {
            scanlines: Some(Scanlines { spacing: 4.0, opacity: 0.25, color: "#000000".into(), drift: false }),
            grain: Some(Grain { opacity: 0.2, size: 1.0, animated: true }),
            pixels: Some(Pixels { size: 6.0, opacity: 0.3 }),
            tint: Some(Tint { color: "#40ff80".into(), opacity: 0.1 }),
            ..Ornament::default()
        },
        ..OverlayStyle::default()
    }
}

/// Warm paper with ink-dark text and flat tan items.
pub(super) fn paper() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#e8dcc0", 1.0),
        items: Paint::new("#d6c6a0", 0.9),
        selected: Paint::new("#8a3b2a", 1.0),
        border: Some(Stroke { width: 2.0, color: "#6b5a3a".into(), opacity: 1.0 }),
        ornament: Ornament { paper: Some(Texture { opacity: 0.6 }), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Brushed steel with raised, bevelled buttons.
pub(super) fn metal() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#8d9299", 1.0),
        items: Paint::new("#a9aeb5", 1.0),
        selected: Paint::new("#2f5db0", 1.0),
        corners: 0.2,
        ornament: Ornament {
            brushed_metal: Some(Texture { opacity: 0.7 }),
            bevel: Some(Bevel { depth: 0.8 }),
            ..Ornament::default()
        },
        ..OverlayStyle::default()
    }
}

/// A menu style with most of what the typography and ornaments can do switched on.
pub(super) fn ornate() -> OverlayStyle {
    let gold = "#c8a040";
    OverlayStyle {
        background: Paint::new("#16181c", 0.92),
        border: Some(Stroke { width: 2.0, color: gold.into(), opacity: 1.0 }),
        gradient: Some(Gradient { color: "#3a1010".into(), opacity: 1.0, angle: 180.0, drift: true }),
        glow: Some(Shade { pulse: true, ..Shade::glow("#ffcc40") }),
        typography: Typography { case: Case::Upper, letter_spacing: 2.0, ..Typography::default() },
        ornament: Ornament {
            dividers: Some(Stroke { width: 1.0, color: gold.into(), opacity: 0.6 }),
            frame: Some(Frame { kind: FrameKind::Brackets, size: 16.0, width: 2.0, color: gold.into(), opacity: 1.0 }),
            scanlines: Some(Scanlines { spacing: 4.0, opacity: 0.35, color: "#000000".into(), drift: true }),
            vignette: Some(Vignette { color: "#000000".into(), opacity: 0.6, reach: 25.0 }),
            ..Ornament::default()
        },
        ..OverlayStyle::default()
    }
}

/// A panel in a frame of this kind, in colors that suit it.
pub(super) fn framed(kind: FrameKind) -> OverlayStyle {
    let (background, items, selected, ink) = match kind {
        FrameKind::ArtDeco => ("#101418", "#1c2228", "#b08a3a", "#d4af5a"),
        FrameKind::ArtNouveau => ("#1d2a22", "#2a3b30", "#8a6a3a", "#c9b27a"),
        FrameKind::Runic => ("#1a1c20", "#262a30", "#5a6e8a", "#a8b8c8"),
        FrameKind::Gothic => ("#141018", "#221a28", "#6a1e2a", "#b8a070"),
        FrameKind::Baroque => ("#24140e", "#341e16", "#8a2a1e", "#d8b060"),
        FrameKind::Circuit => ("#06140e", "#0c2218", "#14804a", "#40e090"),
        FrameKind::Celtic => ("#0e1a14", "#18281e", "#2e6a3a", "#c8b878"),
        FrameKind::GreekKey => ("#1c1610", "#2a2018", "#a0502a", "#d89a5a"),
        FrameKind::Egyptian => ("#0c1a2e", "#16263e", "#1f7a7a", "#d8b45a"),
        FrameKind::Floral => ("#241a22", "#33252f", "#8a3a5a", "#e0a8b8"),
        FrameKind::Riveted | FrameKind::Brackets | FrameKind::Chamfer => ("#2a2c2e", "#3a3c3e", "#8a4a1a", "#9a9ea2"),
    };
    let width = if kind == FrameKind::Riveted { 2.0 } else { 1.5 };
    // Marks that run along the edges want square corners to sit on; the curving ones fit a round panel.
    let corners = if matches!(kind, FrameKind::ArtNouveau | FrameKind::Baroque | FrameKind::Circuit | FrameKind::Floral) { OverlayStyle::default().corners } else { 0.0 };
    OverlayStyle {
        background: Paint::new(background, 0.95),
        items: Paint::new(items, 0.9),
        selected: Paint::new(selected, 1.0),
        corners,
        ornament: Ornament { frame: Some(Frame { kind, size: kind.natural_size(), width, color: ink.into(), opacity: 1.0 }), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// The style of a scene that shows the sample list menu in it: `menu-paper`, `frame-art-deco`
/// and the like.
pub(super) fn list_scene(name: &str) -> Option<OverlayStyle> {
    Some(match name {
        "menu-filters" => filtered(),
        "menu-camo" => camouflaged(),
        "menu-paper" => paper(),
        "menu-metal" => metal(),
        _ => {
            let slug = name.strip_prefix("frame-")?;
            framed(FrameKind::ALL.into_iter().find(|k| k.to_string().to_lowercase().replace(' ', "-") == slug)?)
        }
    })
}
