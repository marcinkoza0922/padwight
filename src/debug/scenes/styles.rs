//! The sample styles the menu scenes are drawn in: textures, filters, ornaments and frames.

use crate::config::{
    Argyle, Bevel, Camo, CamoScheme, Case, FilterOn, Frame, FrameKind, Gradient, Grain, Ornament, OverlayStyle, Paint, Pixels, Plaid, Scanlines, Shade, StripeDirection, Stripes,
    Stroke, Texture, Tint, Typography, Vignette, default_argyle_colors, default_plaid_colors, default_stripes_colors,
};

/// A panel under a woodland camo.
pub(super) fn camouflaged() -> OverlayStyle {
    OverlayStyle { ornament: Ornament { camo: Some(Camo { scheme: CamoScheme::Woodland, opacity: 0.45, size: 60.0, on: FilterOn::Both }), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// A panel under a red tartan.
pub(super) fn plaid() -> OverlayStyle {
    OverlayStyle { ornament: Ornament { plaid: Some(Plaid { colors: default_plaid_colors(), opacity: 0.45, size: 80.0, on: FilterOn::Both }), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// A panel under a navy and camel argyle.
pub(super) fn argyle() -> OverlayStyle {
    OverlayStyle { ornament: Ornament { argyle: Some(Argyle { colors: default_argyle_colors(), opacity: 0.45, size: 40.0, on: FilterOn::Both }), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// A panel under slanted navy and cream stripes.
pub(super) fn striped() -> OverlayStyle {
    let stripes = Stripes { colors: default_stripes_colors(), direction: StripeDirection::Diagonal, opacity: 0.45, size: 16.0, on: FilterOn::Both };
    OverlayStyle { ornament: Ornament { stripes: Some(stripes), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// Plaid on the items alone, and wood on the background alone.
pub(super) fn plaid_items() -> OverlayStyle {
    let plaid = Plaid { colors: default_plaid_colors(), opacity: 0.45, size: 80.0, on: FilterOn::Items };
    OverlayStyle { ornament: Ornament { plaid: Some(plaid), wood: Some(Texture::new(0.8)), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// Stripes on the background alone, under the items.
pub(super) fn striped_background() -> OverlayStyle {
    let stripes = Stripes { colors: default_stripes_colors(), direction: StripeDirection::Diagonal, opacity: 0.45, size: 16.0, on: FilterOn::Background };
    OverlayStyle { ornament: Ornament { stripes: Some(stripes), ..Ornament::default() }, ..OverlayStyle::default() }
}

/// A dark panel under every filter.
pub(super) fn filtered() -> OverlayStyle {
    OverlayStyle {
        ornament: Ornament {
            scanlines: Some(Scanlines { spacing: 4.0, opacity: 0.25, color: "#000000".into(), drift: false, on: FilterOn::Both }),
            grain: Some(Grain { opacity: 0.2, size: 1.0, animated: true, on: FilterOn::Both }),
            pixels: Some(Pixels { size: 6.0, opacity: 0.3, shimmer: true, on: FilterOn::Both }),
            tint: Some(Tint { color: "#40ff80".into(), opacity: 0.1, on: FilterOn::Both }),
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
        ornament: Ornament { paper: Some(Texture::new(0.6)), ..Ornament::default() },
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
            brushed_metal: Some(Texture::new(0.7)),
            bevel: Some(Bevel { depth: 0.8 }),
            ..Ornament::default()
        },
        ..OverlayStyle::default()
    }
}

/// Poured concrete with bevelled buttons.
pub(super) fn concrete() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#7b7b78", 1.0),
        items: Paint::new("#8f8f8b", 0.95),
        selected: Paint::new("#b5532a", 1.0),
        ornament: Ornament { concrete: Some(Texture::new(0.8)), bevel: Some(Bevel { depth: 0.5 }), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Woven cloth, in natural linen.
pub(super) fn cloth() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#b8ad94", 1.0),
        items: Paint::new("#a39880", 0.9),
        selected: Paint::new("#6a4a7a", 1.0),
        ornament: Ornament { cloth: Some(Texture::new(0.7)), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Denim, with orange stitching for the selected item.
pub(super) fn denim() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#34465e", 1.0),
        items: Paint::new("#43577a", 0.9),
        selected: Paint::new("#c8913a", 1.0),
        ornament: Ornament { denim: Some(Texture::new(0.8)), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Crimson silk, with gold for the selected item.
pub(super) fn silk() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#7a1f2b", 1.0),
        items: Paint::new("#8e2a37", 0.85),
        selected: Paint::new("#c8a040", 1.0),
        ornament: Ornament { silk: Some(Texture::new(0.7)), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Brown pebbled leather with bevelled buttons.
pub(super) fn leather() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#5a3422", 1.0),
        items: Paint::new("#6e4430", 0.9),
        selected: Paint::new("#b08a3a", 1.0),
        ornament: Ornament { leather: Some(Texture::new(0.6)), bevel: Some(Bevel { depth: 0.4 }), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Steel tread plate with bevelled buttons.
pub(super) fn steel() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#6f747a", 1.0),
        items: Paint::new("#878c93", 0.95),
        selected: Paint::new("#c0392b", 1.0),
        corners: 0.1,
        ornament: Ornament { steel: Some(Texture::new(0.8)), bevel: Some(Bevel { depth: 0.7 }), ..Ornament::default() },
        ..OverlayStyle::default()
    }
}

/// Wooden planks with bevelled buttons.
pub(super) fn wood() -> OverlayStyle {
    OverlayStyle {
        background: Paint::new("#8a5a32", 1.0),
        items: Paint::new("#9c6c40", 0.9),
        selected: Paint::new("#2f5a3a", 1.0),
        ornament: Ornament { wood: Some(Texture::new(0.8)), bevel: Some(Bevel { depth: 0.4 }), ..Ornament::default() },
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
            scanlines: Some(Scanlines { spacing: 4.0, opacity: 0.35, color: "#000000".into(), drift: true, on: FilterOn::Both }),
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
        "menu-plaid" => plaid(),
        "menu-argyle" => argyle(),
        "menu-stripes" => striped(),
        "menu-plaid-items" => plaid_items(),
        "menu-stripes-background" => striped_background(),
        "menu-paper" => paper(),
        "menu-metal" => metal(),
        "menu-concrete" => concrete(),
        "menu-cloth" => cloth(),
        "menu-denim" => denim(),
        "menu-silk" => silk(),
        "menu-leather" => leather(),
        "menu-steel" => steel(),
        "menu-wood" => wood(),
        _ => {
            let slug = name.strip_prefix("frame-")?;
            framed(FrameKind::ALL.into_iter().find(|k| k.to_string().to_lowercase().replace(' ', "-") == slug)?)
        }
    })
}
