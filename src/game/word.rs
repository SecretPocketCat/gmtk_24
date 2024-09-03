use crate::{assets::WordlistAssets, prelude::*};
use bevy::{color::palettes::tailwind, utils::HashSet};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<TileWord>()
        .add_event::<WordAdvancedEvent>()
        .add_event::<WordFinishedEvent>()
        .add_systems(OnExit(Screen::Loading), update_word_list)
        .add_systems(
            Update,
            update_word_list.run_if(assets_exist.and_then(resource_changed::<MovementBindings>)),
        )
        .add_systems(
            Update,
            (spawn_tile_words, update_ground_text_sections).run_if(in_game),
        )
        .add_systems(
            Update,
            (tween_ground_texts, tween_out_finished_words).run_if(level_ready),
        );
}

#[derive(Resource, Reflect, Debug, Deref, DerefMut)]
pub struct WordList {
    ground_words: Vec<String>,
    // other words for bosses etc
    // enemy_words: Vec<String>,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum WordStatus {
    Pristine,
    Damaged,
    Finished,
}

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

    pub(crate) fn status(&self) -> WordStatus {
        match self.typed_char_len {
            0 => WordStatus::Pristine,
            typed if typed >= self.text.len() => WordStatus::Finished,
            _ => WordStatus::Damaged,
        }
    }

    pub(crate) fn section(
        text: impl Into<String>,
        color: Color,
        font: Handle<Font>,
    ) -> TextSection {
        TextSection::new(
            text.into(),
            TextStyle {
                color,
                font_size: 36.0,
                font,
                ..default()
            },
        )
    }

    pub(crate) fn done_section(
        text: impl Into<String>,
        alpha: f32,
        font: Handle<Font>,
    ) -> TextSection {
        Self::section(text, tailwind::GRAY_700.with_alpha(alpha).into(), font)
    }

    pub(crate) fn text_sections(&self, alpha: f32, font: Handle<Font>) -> Vec<TextSection> {
        tile_word_text_sections(
            self.text.as_str(),
            self.typed_char_len,
            self.status(),
            alpha,
            font,
        )
    }
}

fn update_word_list(
    wordlists: Res<Assets<WordListSource>>,
    wordlist_assets: Res<WordlistAssets>,
    bindings: Res<MovementBindings>,
    mut cmd: Commands,
) {
    let blacklist = [
        &bindings.up,
        &bindings.down,
        &bindings.left,
        &bindings.right,
    ];
    let source = or_return!(wordlists.get(&wordlist_assets.en));
    let mut words: Vec<_> = source
        .0
        .iter()
        .filter(|w| w.len() >= 3 && !blacklist.iter().any(|blacklisted| w.contains(*blacklisted)))
        .cloned()
        .collect();
    words.sort_unstable_by_key(|w| w.len());
    let split_i = or_return!(words.iter().enumerate().find_map(|(i, w)| if w.len() > 4 {
        Some(i)
    } else {
        None
    }));
    let _longer_words = words.split_off(split_i);

    cmd.insert_resource(WordList {
        ground_words: words,
    });
}

fn tile_word_text_sections<'a>(
    text: impl Into<&'a str>,
    typed_len: usize,
    status: WordStatus,
    alpha: f32,
    font: Handle<Font>,
) -> Vec<TextSection> {
    let text = text.into();
    let mut res = Vec::with_capacity(4);
    if status != WordStatus::Pristine {
        res.push(TileWord::done_section(
            text[..typed_len].to_string(),
            alpha,
            font.clone_weak(),
        ));
    }
    if status != WordStatus::Finished {
        res.push(TileWord::section(
            "|",
            tailwind::GRAY_300.with_alpha(alpha).into(),
            font.clone_weak(),
        ));
        let next_char_i = typed_len + 1;
        res.push(TileWord::section(
            text[typed_len..next_char_i].to_string(),
            tailwind::GREEN_200.with_alpha(alpha).into(),
            font.clone_weak(),
        ));
        res.push(TileWord::section(
            text[next_char_i..].to_string(),
            tailwind::GRAY_200.with_alpha(alpha).into(),
            font.clone_weak(),
        ));
    }

    res
}

#[derive(Component, Default)]
struct TileWordVisible;

fn spawn_tile_words(
    ground_q: Query<Entity, Added<Ground>>,
    mut cmd: Commands,
    wordlist: Res<WordList>,
    fonts: Res<FontAssets>,
) {
    let mut rng = thread_rng();
    for e in &ground_q {
        let word = &wordlist
            .ground_words
            .choose(&mut rng)
            .expect("random word picked");
        let mut text_e = None;
        let mut e_cmd = or_continue!(cmd.get_entity(e));
        e_cmd
            .with_children(|b| {
                text_e = Some(
                    b.spawn((
                        Text2dBundle {
                            text: Text::from_sections(tile_word_text_sections(
                                word.as_str(),
                                0,
                                WordStatus::Pristine,
                                0.0,
                                fonts.tile.clone_weak(),
                            )),
                            transform: Transform::from_translation(Vec2::ZERO.extend(0.1))
                                .with_scale(Vec2::splat(0.25).extend(1.)),
                            ..default()
                        },
                        HIGH_RES_RENDER_LAYER,
                    ))
                    .id(),
                );
            })
            .try_insert(TileWord::new(word.to_string(), text_e.unwrap()))
            .add_child(text_e.unwrap());
    }
}

fn update_ground_text_sections(
    mut word_advanced_evr: EventReader<WordAdvancedEvent>,
    mut word_finished_evr: EventReader<WordFinishedEvent>,
    word_q: Query<&TileWord>,
    mut text_q: Query<&mut Text>,
    fonts: Res<FontAssets>,
) {
    let mut entities: Vec<_> = word_advanced_evr.read().map(|ev| ev.0).collect();
    entities.extend(word_finished_evr.read().map(|ev| ev.0));
    for word_e in entities {
        let word = or_continue!(word_q.get(word_e));
        let mut text = or_continue!(text_q.get_mut(word.text_e));
        text.sections = word.text_sections(1.0, fonts.tile.clone_weak());
    }
}

fn tween_out_finished_words(
    mut word_finished_evr: EventReader<WordFinishedEvent>,
    word_q: Query<&TileWord, Changed<TileWord>>,
    mut cmd: Commands,
) {
    for ev in word_finished_evr.read() {
        let word = or_continue!(word_q.get(ev.0));
        let mut cmd_e = or_continue!(cmd.get_entity(ev.0));
        cmd_e.try_insert(DespawnOnTweenCompleted::Itself);
        cmd.tween_tile_color(ev.0, Color::NONE, 150, EaseFunction::QuadraticIn);
        cmd.tween_text_alpha(word.text_e, 0.0, 110, EaseFunction::QuadraticIn);
    }
}

// tween text in/out as the player approaches/leaves
fn tween_ground_texts(
    player_q: Query<&GridCoords, (With<Player>, Changed<GridCoords>)>,
    word_q: Query<&TileWord>,
    visible_word_q: Query<Entity, With<TileWordVisible>>,
    level_lookup: Res<LevelEntityLookup>,
    mut cmd: Commands,
) {
    let player_coords = or_return_quiet!(player_q.get_single());
    let visible_tile_ids: HashSet<_> = visible_word_q.iter().collect();
    let radius_tile_ids: HashSet<_> = player_coords
        .neighbours()
        .iter()
        .filter_map(|c| level_lookup.get(c))
        .copied()
        .collect();

    // tween out when player has moved away
    for out_tile_e in visible_tile_ids.difference(&radius_tile_ids) {
        let word = or_continue_quiet!(word_q.get(*out_tile_e));
        if let Some(mut cmd_e) = cmd.get_entity(*out_tile_e) {
            cmd_e.remove::<TileWordVisible>();
            cmd.tween_text_alpha(word.text_e, 0.0, 110, EaseFunction::QuadraticOut);
        }
    }

    // tween in when player has moved in
    for tile_e in radius_tile_ids {
        let word = or_continue_quiet!(word_q.get(tile_e));
        if let Some(mut cmd_e) = cmd.get_entity(tile_e) {
            cmd_e.try_insert(TileWordVisible);
            cmd.tween_text_alpha(word.text_e, 1.0, 110, EaseFunction::QuadraticOut);
        }
    }
}
