//! A line on what a stick or gyro setting means, shown under the page while its row is
//! highlighted: the window has room beside each slider for this, the overlay has one line.

use super::Page;

/// What the row labelled `label` ("Curve: 2.00") means on `page`, if it needs saying.
pub(super) fn about(page: Page, label: &str) -> Option<&'static str> {
    let name = label.split(':').next().unwrap_or(label);
    match page {
        Page::Stick(..) => stick(name),
        Page::Gyro(..) => gyro(name),
        _ => None,
    }
}

fn stick(name: &str) -> Option<&'static str> {
    Some(match name {
        "Kind" => "What the stick does; each kind has its own settings below.",
        "Speed" => "How fast the pointer moves at a full push.",
        "Response" => "How speed follows the push: a preset, or Custom to set the curve below.",
        "Curve" => "1 is linear; higher keeps small pushes slow for fine aim.",
        "Acceleration" => "How much faster the pointer gets while the stick is held far out.",
        "Ramp time" => "How long the acceleration takes to build up.",
        "Outer boost" => "Extra speed near the rim, for quick turns. 0 adds none.",
        "Vertical speed" => "Up and down speed against left and right: below 1× is slower.",
        "Smoothing" => "Averages the stick over this long, for steadier, slightly later moves.",
        "Deadzone" => "How far the stick moves before anything happens, of a full push.",
        "Deadzone shape" => "Circle treats every direction alike; Cross keeps a push along one axis clean of the other.",
        "Anti-deadzone" => "Starts the output this far out, to get past the game's own deadzone.",
        "Anti-deadzone hold" => "Travel past the deadzone that stays at the anti-deadzone before rising.",
        "Rotate output" => "Turns the stick's directions by this angle.",
        "Directions press at" => "How far a push goes before the stick's directions count as pressed.",
        "Sector 1 points at" => "Where the middle of sector 1 points, clockwise from up.",
        "Starts at" => "How far out the stick picks a sector, of a full push.",
        "Stickiness" => "How far past a sector's edge the stick goes before the next one takes over.",
        _ => return None,
    })
}

fn gyro(name: &str) -> Option<&'static str> {
    Some(match name {
        "Mode" => "What turning the controller does: move the mouse, push a stick, or steer.",
        "Sensitivity" => "How far the aim moves for each degree the controller turns.",
        "Real-world sensitivity" => "Sensitivity as a share of the game's turn: at 1×, it turns as far as you do.",
        "Game's full turn" => "Mouse pixels that turn the game all the way round.",
        "Full tilt at" => "How fast a turn pushes the stick all the way.",
        "Anti-deadzone" => "Starts the stick this far out, past the game's own deadzone.",
        "Game's curve" => "The game's own stick response: 1 is linear. Match it for one-to-one turns.",
        "Full push at" => "How far the controller turns to push the stick all the way.",
        "Lock at edge" => "Turning back leaves the edge at once.",
        "Full lock at" => "How far the controller tilts to steer all the way.",
        "Turning read as" => "Which motion aims left and right: yaw turns the controller flat, roll tilts it; player space works in any grip.",
        "Axes" => "Which ways the gyro moves the aim.",
        "Horizontal speed" | "Vertical speed" => "This direction's speed against the other: 1× is the same.",
        "Response" => "How sensitivity follows turning speed: a preset, or set it below.",
        "Slow sensitivity" => "Sensitivity for slow turns, for fine aim.",
        "Fast sensitivity" => "Sensitivity for fast turns, for big moves.",
        "Slow up to" => "Turns slower than this use the slow sensitivity.",
        "Fast from" => "Turns faster than this use the fast sensitivity.",
        "Shape" => "How sensitivity moves from slow to fast: 1 is a straight line.",
        "Pause while pushing" => "The gyro stops while this stick is pushed.",
        "Ignore jitter below" => "Turns slower than this are dropped as hand shake.",
        "Smooth below" => "Turns slower than this are smoothed.",
        "Smoothing time" => "How long slow turns are averaged over.",
        "Re-measure drift when set down" => "Measures the gyro's drift again whenever the controller rests.",
        _ => return None,
    })
}
