//! Tests that drive the settings window and check what Save & apply writes (see `harness.rs`).

use super::harness::Rig;
use super::*;

#[test]
fn smoke() {
    let mut rig = Rig::new();
    rig.send(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    let d = rig.save_diff();
    rig.assert_round_trip();
    panic!("{d:#?}");
}
