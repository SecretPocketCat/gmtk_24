//! Demo gameplay. All of these modules are only intended for demonstration
//! purposes and should be replaced with your own game logic.
//! Feel free to change the logic found here if you feel like tinkering around
//! to get a feeling for the template.

use crate::prelude::*;

pub mod fog_of_war;
pub mod goal;
pub mod level;
pub mod physics;
pub mod player;
pub mod rock;
pub mod word;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins((
        player::plugin,
        level::plugin,
        word::plugin,
        goal::plugin,
        rock::plugin,
        fog_of_war::plugin,
        physics::plugin,
    ));
}
