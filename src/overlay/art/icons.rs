//! The SVG for info icons and play states, in a color given as RGB and an alpha, without any
//! drawing library.

use crate::{
    info::{Charge, FormFactor, Icon, PadFamily},
    media::PlayState,
};


/// The SVG for an info icon, in `ink`. A controller is drawn in its kind's color, where it
/// has one; a battery's fill is green, amber or red by its level.
pub fn info_svg(icon: &Icon, ink: [u8; 3], alpha: f32) -> String {
    let hex = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    let c = hex(ink);
    let body = match icon {
        Icon::Form(FormFactor::Desktop) => format!(
            "<rect x='2' y='3' width='20' height='13' rx='2' fill='none' stroke='{c}' stroke-width='2'/>\
             <path d='M8 21 H16 M12 16 V21' fill='none' stroke='{c}' stroke-width='2' stroke-linecap='round'/>"
        ),
        Icon::Form(FormFactor::Laptop) => format!(
            "<rect x='5' y='4' width='14' height='11' rx='1.5' fill='none' stroke='{c}' stroke-width='2'/>\
             <path d='M2 18 H22 L21 20.5 H3 Z' fill='{c}'/>"
        ),
        Icon::Form(FormFactor::Handheld) => format!(
            "<rect x='2' y='6' width='20' height='12' rx='4' fill='none' stroke='{c}' stroke-width='2'/>\
             <rect x='8.5' y='9' width='7' height='6' rx='1' fill='{c}'/>"
        ),
        Icon::Controller(family) => {
            let tint = match family {
                PadFamily::Xbox => hex([0x3c, 0xa0, 0x3c]),
                PadFamily::PlayStation => hex([0x5a, 0x8c, 0xdc]),
                PadFamily::Nintendo => c.clone(),
            };
            format!(
                "<path d='M7 7 H17 C20.5 7 22.5 9.5 22.5 13.5 C22.5 17.5 20.8 18.5 19.2 18.5 \
                 C17.8 18.5 16.9 17.2 15.9 15.8 H8.1 C7.1 17.2 6.2 18.5 4.8 18.5 \
                 C3.2 18.5 1.5 17.5 1.5 13.5 C1.5 9.5 3.5 7 7 7 Z' fill='{tint}'/>"
            )
        }
        Icon::Battery(charge) => battery_svg(*charge, &c),
        Icon::Wifi(percent) => wifi_svg(*percent, &c),
    };
    format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' opacity='{alpha}'>{body}</svg>")
}

/// A battery outline filled to its level; an estimate is faded.
fn battery_svg(charge: Charge, c: &str) -> String {
    let percent = charge.percent.min(100);
    let width = 14.0 * f32::from(percent) / 100.0;
    let level = match percent {
        50.. => [0x3c, 0xa0, 0x3c],
        20..50 => [0xc8, 0xa0, 0x1e],
        _ => [0xc8, 0x3c, 0x3c],
    };
    let level = format!("#{:02x}{:02x}{:02x}", level[0], level[1], level[2]);
    let opacity = if charge.estimated { 0.6 } else { 1.0 };
    format!(
        "<rect x='1.5' y='6.5' width='18' height='11' rx='2.5' fill='none' stroke='{c}' stroke-width='2'/>\
         <rect x='21' y='10' width='2' height='4' rx='1' fill='{c}'/>\
         <rect x='4' y='9' width='{width:.2}' height='6' rx='1' fill='{level}' fill-opacity='{opacity}'/>"
    )
}

/// Wi-Fi bars, lit as the signal passes each quarter.
fn wifi_svg(percent: u8, c: &str) -> String {
    let lit: u8 = match percent {
        0 => 0,
        1..=25 => 1,
        26..=50 => 2,
        51..=75 => 3,
        _ => 4,
    };
    (0u8..4)
        .map(|i| {
            let height = 5.0 + 4.0 * f32::from(i);
            let opacity = if i < lit { 1.0 } else { 0.25 };
            format!(
                "<rect x='{:.1}' y='{:.1}' width='3.5' height='{height:.1}' rx='1' fill='{c}' fill-opacity='{opacity}'/>",
                2.0 + 5.5 * f32::from(i),
                21.0 - height
            )
        })
        .collect()
}

/// A play, pause or stop symbol in `rgb` at `alpha`, drawn so it doesn't depend on the font
/// having the glyph.
pub fn play_state_svg(state: PlayState, rgb: [u8; 3], alpha: f32) -> String {
    let [r, g, b] = rgb;
    let shape = match state {
        PlayState::Playing => "<path d='M6 3 L21 12 L6 21 Z'/>",
        PlayState::Paused => "<rect x='5' y='3' width='5' height='18' rx='1'/><rect x='14' y='3' width='5' height='18' rx='1'/>",
        PlayState::Stopped => "<rect x='5' y='5' width='14' height='14' rx='2'/>",
    };
    format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='#{r:02x}{g:02x}{b:02x}' fill-opacity='{alpha}'>{shape}</svg>")
}
