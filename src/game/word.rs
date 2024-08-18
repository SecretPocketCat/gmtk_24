use crate::prelude::*;
use bevy::{color::palettes::tailwind, utils::HashSet};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<TileWord>()
        .add_event::<WordAdvancedEvent>()
        .add_event::<WordFinishedEvent>()
        .add_systems(
            Update,
            (spawn_tile_words, update_ground_text_sections).run_if(in_game),
        )
        .add_systems(Update, (tween_ground_texts,).run_if(level_ready));
}

#[derive(Component, Reflect, Debug)]
pub(crate) struct TileWord {
    text: String,
    typed_char_len: usize,
    text_e: Entity,
}

#[derive(Event, Debug, Reflect)]
pub(crate) struct WordAdvancedEvent(pub Entity);

#[derive(Event, Debug, Reflect)]
pub(crate) struct WordFinishedEvent(pub Entity);

impl TileWord {
    pub(crate) fn new(text: impl Into<String>, text_e: Entity) -> Self {
        Self {
            text: text.into(),
            typed_char_len: 0,
            text_e,
        }
    }

    pub(crate) fn remaining(&self) -> String {
        self.text.chars().skip(self.typed_char_len).collect()
    }

    pub(crate) fn advance(&mut self, count: usize) {
        self.typed_char_len += count;
    }

    pub(crate) fn finished(&self) -> bool {
        self.text.len() <= self.typed_char_len
    }

    pub(crate) fn damaged(&self) -> bool {
        self.typed_char_len > 0
    }

    pub(crate) fn section(text: impl Into<String>, color: Color) -> TextSection {
        TextSection::new(
            text.into(),
            TextStyle {
                color,
                font_size: 24.0,
                ..default()
            },
        )
    }

    pub(crate) fn done_section(text: impl Into<String>, alpha: f32) -> TextSection {
        Self::section(text, tailwind::GRAY_700.with_alpha(alpha).into())
    }

    pub(crate) fn text_sections(&self, alpha: f32) -> Vec<TextSection> {
        let mut res = Vec::with_capacity(4);
        if self.damaged() {
            res.push(Self::done_section(
                self.text[..self.typed_char_len].to_string(),
                alpha,
            ));
        }
        if !self.finished() {
            res.push(Self::section(
                "|",
                tailwind::GRAY_300.with_alpha(alpha).into(),
            ));
            let next_char_i = self.typed_char_len + 1;
            res.push(Self::section(
                self.text[self.typed_char_len..next_char_i].to_string(),
                tailwind::GREEN_200.with_alpha(alpha).into(),
            ));
            res.push(Self::section(
                self.text[next_char_i..].to_string(),
                tailwind::GRAY_200.with_alpha(alpha).into(),
            ));
        }

        res
    }
}

#[derive(Component, Default)]
struct TileWordVisible;

const WORDS: &[&str] = &["bar", "baz", "test", "dig"];

fn spawn_tile_words(ground_q: Query<Entity, Added<Ground>>, mut cmd: Commands) {
    let mut rng = thread_rng();
    for e in &ground_q {
        let word = *WORDS.choose(&mut rng).expect("random word picked");
        let mut text_e = None;
        let mut e_cmd = or_continue!(cmd.get_entity(e));
        e_cmd
            .with_children(|b| {
                text_e = Some(
                    b.spawn(Text2dBundle {
                        text: Text::from_sections(vec![TileWord::done_section(word, 0.0)]),
                        transform: Transform::from_translation(Vec2::ZERO.extend(0.1))
                            .with_scale(Vec2::splat(0.25).extend(1.)),
                        ..default()
                    })
                    .id(),
                );
            })
            .try_insert(TileWord::new(word, text_e.unwrap()))
            .add_child(text_e.unwrap());
    }
}

fn update_ground_text_sections(
    mut word_advanced_evr: EventReader<WordAdvancedEvent>,
    mut word_finished_evr: EventReader<WordFinishedEvent>,
    word_q: Query<&TileWord>,
    mut text_q: Query<&mut Text>,
) {
    let mut entities: Vec<_> = word_advanced_evr.read().map(|ev| ev.0).collect();
    entities.extend(word_finished_evr.read().map(|ev| ev.0));
    for word_e in entities {
        let word = or_continue!(word_q.get(word_e));
        let mut text = or_continue!(text_q.get_mut(word.text_e));
        text.sections = word.text_sections(if word.damaged() { 1.0 } else { 0.0 });
    }
}

// fn remove_finished_words(
//     word_q: Query<&TileWord, Changed<TileWord>>,
//     mut text_q: Query<&mut Text>,
// ) {
//     for word in word_q.iter().filter(|w| w.finished()) {
//         // cmd.tween_text_alpha(word.text_e, 0.0, 110, EaseFunction::QuadraticOut);
//     }
// }

// tween text in/out as the player approaches/leaves
fn tween_ground_texts(
    player_q: Query<&GridCoords, (With<Player>, Changed<GridCoords>)>,
    word_q: Query<&TileWord>,
    visible_word_q: Query<Entity, With<TileWordVisible>>,
    level_lookup: Res<LevelEntityLookup>,
    mut cmd: Commands,
) {
    let radius = 3;
    let player_coords = or_return_quiet!(player_q.get_single());
    let visible_tile_ids: HashSet<_> = visible_word_q.iter().collect();
    let radius_tile_pairs: Vec<_> = player_coords
        .radius(radius, false)
        .iter()
        .filter_map(|c| level_lookup.get(c).map(|e| (*c, *e)))
        .collect();

    // tween out when player has moved away
    let radius_tile_ids: HashSet<_> = radius_tile_pairs.iter().map(|(_, e)| *e).collect();
    for out_tile_e in visible_tile_ids.difference(&radius_tile_ids) {
        let word = or_continue_quiet!(word_q.get(*out_tile_e));
        if let Some(mut cmd_e) = cmd.get_entity(*out_tile_e) {
            cmd_e.remove::<TileWordVisible>();
            cmd.tween_text_alpha(word.text_e, 0.0, 110, EaseFunction::QuadraticOut);
        }
    }

    // tween in when player has moved in
    for (tile_coords, tile_e) in radius_tile_pairs {
        let word = or_continue_quiet!(word_q.get(tile_e));
        let dist = tile_coords.distance(player_coords).floor();

        if let Some(mut cmd_e) = cmd.get_entity(tile_e) {
            cmd_e.try_insert(TileWordVisible);
            cmd.tween_text_alpha(
                word.text_e,
                // opacity based on distance from player
                1.0 - ((dist - 1.0) / radius as f32) * 0.8,
                110,
                EaseFunction::QuadraticOut,
            );
        }
    }
}
