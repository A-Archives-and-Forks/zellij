use ansi_term::ANSIStrings;
use unicode_width::UnicodeWidthStr;

use crate::{LinePart, ARROW_SEPARATOR};
use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::*;
use zellij_tile_utils::style;

fn get_current_title_len(current_title: &[LinePart]) -> usize {
    current_title.iter().map(|p| p.len).sum()
}

// move elements from before_active and after_active into tabs_to_render while they fit in cols
// adds collapsed_tabs to the left and right if there's left over tabs that don't fit
fn populate_tabs_in_tab_line(
    tabs_before_active: &mut Vec<LinePart>,
    tabs_after_active: &mut Vec<LinePart>,
    tabs_to_render: &mut Vec<LinePart>,
    cols: usize,
    palette: Styling,
    capabilities: PluginCapabilities,
) {
    let mut middle_size = get_current_title_len(tabs_to_render);

    let mut total_left = 0;
    let mut total_right = 0;
    loop {
        let left_count = tabs_before_active.len();
        let right_count = tabs_after_active.len();

        // left_more_tab_index is the tab to the left of the leftmost visible tab
        let left_more_tab_index = left_count.saturating_sub(1);
        let collapsed_left = left_more_message(
            left_count,
            palette,
            tab_separator(capabilities),
            left_more_tab_index,
        );
        // right_more_tab_index is the tab to the right of the rightmost visible tab
        let right_more_tab_index = left_count + tabs_to_render.len();
        let collapsed_right = right_more_message(
            right_count,
            palette,
            tab_separator(capabilities),
            right_more_tab_index,
        );

        let total_size = collapsed_left.len + middle_size + collapsed_right.len;

        if total_size > cols {
            // break and dont add collapsed tabs to tabs_to_render, they will not fit
            break;
        }

        let left = if let Some(tab) = tabs_before_active.last() {
            tab.len
        } else {
            usize::MAX
        };

        let right = if let Some(tab) = tabs_after_active.first() {
            tab.len
        } else {
            usize::MAX
        };

        // total size is shortened if the next tab to be added is the last one, as that will remove the collapsed tab
        let size_by_adding_left =
            left.saturating_add(total_size)
                .saturating_sub(if left_count == 1 {
                    collapsed_left.len
                } else {
                    0
                });
        let size_by_adding_right =
            right
                .saturating_add(total_size)
                .saturating_sub(if right_count == 1 {
                    collapsed_right.len
                } else {
                    0
                });

        let left_fits = size_by_adding_left <= cols;
        let right_fits = size_by_adding_right <= cols;
        // active tab is kept in the middle by adding to the side that
        // has less width, or if the tab on the other side doesn't fit
        if (total_left <= total_right || !right_fits) && left_fits {
            // add left tab
            let tab = tabs_before_active.pop().unwrap();
            middle_size += tab.len;
            total_left += tab.len;
            tabs_to_render.insert(0, tab);
        } else if right_fits {
            // add right tab
            let tab = tabs_after_active.remove(0);
            middle_size += tab.len;
            total_right += tab.len;
            tabs_to_render.push(tab);
        } else {
            // there's either no space to add more tabs or no more tabs to add, so we're done
            tabs_to_render.insert(0, collapsed_left);
            tabs_to_render.push(collapsed_right);
            break;
        }
    }
}

fn left_more_message(
    tab_count_to_the_left: usize,
    palette: Styling,
    separator: &str,
    tab_index: usize,
) -> LinePart {
    if tab_count_to_the_left == 0 {
        return LinePart::default();
    }
    let more_text = if tab_count_to_the_left < 10000 {
        format!(" ← +{} ", tab_count_to_the_left)
    } else {
        " ← +many ".to_string()
    };
    // 238
    // chars length plus separator length on both sides
    let more_text_len = more_text.width() + 2 * separator.width();
    let (text_color, sep_color) = (
        palette.ribbon_unselected.base,
        palette.text_unselected.background,
    );
    let plus_ribbon_bg = palette.text_selected.emphasis_0;
    let left_separator = style!(sep_color, plus_ribbon_bg).paint(separator);
    let more_styled_text = style!(text_color, plus_ribbon_bg).bold().paint(more_text);
    let right_separator = style!(plus_ribbon_bg, sep_color).paint(separator);
    let more_styled_text =
        ANSIStrings(&[left_separator, more_styled_text, right_separator]).to_string();
    LinePart {
        part: more_styled_text,
        len: more_text_len,
        tab_index: Some(tab_index),
    }
}

fn right_more_message(
    tab_count_to_the_right: usize,
    palette: Styling,
    separator: &str,
    tab_index: usize,
) -> LinePart {
    if tab_count_to_the_right == 0 {
        return LinePart::default();
    };
    let more_text = if tab_count_to_the_right < 10000 {
        format!(" +{} → ", tab_count_to_the_right)
    } else {
        " +many → ".to_string()
    };
    // chars length plus separator length on both sides
    let more_text_len = more_text.width() + 2 * separator.width();

    let (text_color, sep_color) = (
        palette.ribbon_unselected.base,
        palette.text_unselected.background,
    );
    let plus_ribbon_bg = palette.text_selected.emphasis_0;
    let left_separator = style!(sep_color, plus_ribbon_bg).paint(separator);
    let more_styled_text = style!(text_color, plus_ribbon_bg).bold().paint(more_text);
    let right_separator = style!(plus_ribbon_bg, sep_color).paint(separator);
    let more_styled_text =
        ANSIStrings(&[left_separator, more_styled_text, right_separator]).to_string();
    LinePart {
        part: more_styled_text,
        len: more_text_len,
        tab_index: Some(tab_index),
    }
}

fn tab_line_prefix(
    session_name: Option<&str>,
    mode: InputMode,
    palette: Styling,
    cols: usize,
) -> Vec<LinePart> {
    let prefix_text = " Zellij ".to_string();

    let prefix_text_len = prefix_text.chars().count();
    let text_color = palette.text_unselected.base;
    let bg_color = palette.text_unselected.background;
    let locked_mode_color = palette.text_unselected.emphasis_3;
    let normal_mode_color = palette.text_unselected.emphasis_2;
    let other_modes_color = palette.text_unselected.emphasis_0;

    let prefix_styled_text = style!(text_color, bg_color).bold().paint(prefix_text);
    let mut parts = vec![LinePart {
        part: prefix_styled_text.to_string(),
        len: prefix_text_len,
        tab_index: None,
    }];
    if let Some(name) = session_name {
        let name_part = format!("({})", name);
        let name_part_len = name_part.width();
        let name_part_styled_text = style!(text_color, bg_color).bold().paint(name_part);
        if cols.saturating_sub(prefix_text_len) >= name_part_len {
            parts.push(LinePart {
                part: name_part_styled_text.to_string(),
                len: name_part_len,
                tab_index: None,
            })
        }
    }
    let mode_part = format!("{:?}", mode).to_uppercase();
    let mode_part_padded = format!(" {} ", mode_part);
    let mode_part_len = mode_part_padded.width();
    let mode_part_styled_text = if mode == InputMode::Locked {
        style!(locked_mode_color, bg_color)
            .bold()
            .paint(mode_part_padded)
    } else if mode == InputMode::Normal {
        style!(normal_mode_color, bg_color)
            .bold()
            .paint(mode_part_padded)
    } else {
        style!(other_modes_color, bg_color)
            .bold()
            .paint(mode_part_padded)
    };
    if cols.saturating_sub(prefix_text_len) >= mode_part_len {
        parts.push(LinePart {
            part: format!("{}", mode_part_styled_text),
            len: mode_part_len,
            tab_index: None,
        })
    }
    parts
}

pub fn tab_separator(capabilities: PluginCapabilities) -> &'static str {
    if !capabilities.arrow_fonts {
        ARROW_SEPARATOR
    } else {
        ""
    }
}

pub fn tab_line(
    session_name: Option<&str>,
    mut all_tabs: Vec<LinePart>,
    active_tab_index: usize,
    cols: usize,
    palette: Styling,
    capabilities: PluginCapabilities,
    hide_session_name: bool,
    mode: InputMode,
    active_swap_layout_name: &Option<String>,
    is_swap_layout_dirty: bool,
    mode_info: &ModeInfo,
    grouped_pane_count: Option<usize>,
) -> Vec<LinePart> {
    let mut tabs_after_active = all_tabs.split_off(active_tab_index);
    let mut tabs_before_active = all_tabs;
    let active_tab = if !tabs_after_active.is_empty() {
        tabs_after_active.remove(0)
    } else {
        tabs_before_active.pop().unwrap()
    };
    let mut prefix = match hide_session_name {
        true => tab_line_prefix(None, mode, palette, cols),
        false => tab_line_prefix(session_name, mode, palette, cols),
    };
    let prefix_len = get_current_title_len(&prefix);

    // if active tab alone won't fit in cols, don't draw any tabs
    if prefix_len + active_tab.len > cols {
        return prefix;
    }

    let mut tabs_to_render = vec![active_tab];

    populate_tabs_in_tab_line(
        &mut tabs_before_active,
        &mut tabs_after_active,
        &mut tabs_to_render,
        cols.saturating_sub(prefix_len),
        palette,
        capabilities,
    );
    prefix.append(&mut tabs_to_render);

    let current_title_len = get_current_title_len(&prefix);
    if current_title_len < cols {
        let mut remaining_space = cols - current_title_len;
        let remaining_bg = palette.text_unselected.background;
        let right_side_component = match grouped_pane_count {
            Some(grouped_pane_count) => {
                render_group_controls(mode_info, grouped_pane_count, remaining_space)
            },
            None => swap_layout_status(
                remaining_space,
                active_swap_layout_name,
                is_swap_layout_dirty,
                mode,
                &palette,
                tab_separator(capabilities),
            ),
        };
        if let Some(right_side_component) = right_side_component {
            remaining_space -= right_side_component.len;
            let mut buffer = String::new();
            for _ in 0..remaining_space {
                buffer.push_str(&style!(remaining_bg, remaining_bg).paint(" ").to_string());
            }
            prefix.push(LinePart {
                part: buffer,
                len: remaining_space,
                tab_index: None,
            });
            prefix.push(right_side_component);
        }
    }

    prefix
}

fn swap_layout_status(
    max_len: usize,
    swap_layout_name: &Option<String>,
    is_swap_layout_damaged: bool,
    input_mode: InputMode,
    palette: &Styling,
    separator: &str,
) -> Option<LinePart> {
    match swap_layout_name {
        Some(swap_layout_name) => {
            let mut swap_layout_name = format!(" {} ", swap_layout_name);
            swap_layout_name.make_ascii_uppercase();
            let swap_layout_name_len = swap_layout_name.len() + 3;
            let bg = palette.text_unselected.background;
            let fg = palette.ribbon_unselected.background;
            let green = palette.ribbon_selected.background;

            let (prefix_separator, swap_layout_name, suffix_separator) =
                if input_mode == InputMode::Locked {
                    (
                        style!(bg, fg).paint(separator),
                        style!(bg, fg).italic().paint(&swap_layout_name),
                        style!(fg, bg).paint(separator),
                    )
                } else if is_swap_layout_damaged {
                    (
                        style!(bg, fg).paint(separator),
                        style!(bg, fg).bold().paint(&swap_layout_name),
                        style!(fg, bg).paint(separator),
                    )
                } else {
                    (
                        style!(bg, green).paint(separator),
                        style!(bg, green).bold().paint(&swap_layout_name),
                        style!(green, bg).paint(separator),
                    )
                };
            let swap_layout_indicator = format!(
                "{}{}{}",
                prefix_separator, swap_layout_name, suffix_separator
            );
            let (part, full_len) = (format!("{}", swap_layout_indicator), swap_layout_name_len);
            let short_len = swap_layout_name_len + 1; // 1 is the space between
            if full_len <= max_len {
                Some(LinePart {
                    part,
                    len: full_len,
                    tab_index: None,
                })
            } else if short_len <= max_len && input_mode != InputMode::Locked {
                Some(LinePart {
                    part: swap_layout_indicator,
                    len: short_len,
                    tab_index: None,
                })
            } else {
                None
            }
        },
        None => None,
    }
}

fn render_group_controls(
    help: &ModeInfo,
    grouped_pane_count: usize,
    max_len: usize,
) -> Option<LinePart> {
    let currently_marking_group = help.currently_marking_pane_group.unwrap_or(false);
    let keymap = help.get_mode_keybinds();
    let (common_modifiers, multiple_select_key, pane_group_toggle_key, group_mark_toggle_key) = {
        let multiple_select_key = multiple_select_key(&keymap);
        let pane_group_toggle_key = single_action_key(&keymap, &[Action::TogglePaneInGroup]);
        let group_mark_toggle_key = single_action_key(&keymap, &[Action::ToggleGroupMarking]);
        let common_modifiers = get_common_modifiers(
            vec![
                multiple_select_key.iter().next(),
                pane_group_toggle_key.iter().next(),
                group_mark_toggle_key.iter().next(),
            ]
            .into_iter()
            .filter_map(|k| k)
            .collect(),
        );
        let multiple_select_key: Vec<KeyWithModifier> = multiple_select_key
            .iter()
            .map(|k| k.strip_common_modifiers(&common_modifiers))
            .collect();
        let pane_group_toggle_key: Vec<KeyWithModifier> = pane_group_toggle_key
            .iter()
            .map(|k| k.strip_common_modifiers(&common_modifiers))
            .collect();
        let group_mark_toggle_key: Vec<KeyWithModifier> = group_mark_toggle_key
            .iter()
            .map(|k| k.strip_common_modifiers(&common_modifiers))
            .collect();
        (
            common_modifiers,
            multiple_select_key,
            pane_group_toggle_key,
            group_mark_toggle_key,
        )
    };
    let multiple_select_key = multiple_select_key
        .iter()
        .next()
        .map(|key| format!("{}", key))
        .unwrap_or("UNBOUND".to_owned());
    let pane_group_toggle_key = pane_group_toggle_key
        .iter()
        .next()
        .map(|key| format!("{}", key))
        .unwrap_or("UNBOUND".to_owned());
    let group_mark_toggle_key = group_mark_toggle_key
        .iter()
        .next()
        .map(|key| format!("{}", key))
        .unwrap_or("UNBOUND".to_owned());
    let background = help.style.colors.text_unselected.background;
    let foreground = help.style.colors.text_unselected.base;
    let superkey_prefix_style = style!(foreground, background).bold();
    let common_modifier_text = if common_modifiers.is_empty() {
        "".to_owned()
    } else {
        format!(
            "{} + ",
            common_modifiers
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("-")
        )
    };

    // full
    let full_selected_panes_text = if common_modifier_text.is_empty() {
        format!("{} SELECTED PANES", grouped_pane_count)
    } else {
        format!("{} SELECTED PANES |", grouped_pane_count)
    };
    let full_group_actions_text = format!("<{}> Group Actions", &multiple_select_key);
    let full_toggle_group_text = format!("<{}> Toggle Group", &pane_group_toggle_key);
    let full_group_mark_toggle_text = format!("<{}> Follow Focus", &group_mark_toggle_key);
    let ribbon_paddings_len = 12;
    let full_controls_line_len = full_selected_panes_text.chars().count()
        + 1
        + common_modifier_text.chars().count()
        + full_group_actions_text.chars().count()
        + full_toggle_group_text.chars().count()
        + full_group_mark_toggle_text.chars().count()
        + ribbon_paddings_len
        + 1; // 1 for the end padding

    // medium
    let medium_selected_panes_text = if common_modifier_text.is_empty() {
        format!("{} SELECTED", grouped_pane_count)
    } else {
        format!("{} SELECTED |", grouped_pane_count)
    };
    let medium_group_actions_text = format!("<{}> Actions", &multiple_select_key);
    let medium_toggle_group_text = format!("<{}> Toggle", &pane_group_toggle_key);
    let medium_group_mark_toggle_text = format!("<{}> Follow", &group_mark_toggle_key);
    let ribbon_paddings_len = 12;
    let medium_controls_line_len = medium_selected_panes_text.chars().count()
        + 1
        + common_modifier_text.chars().count()
        + medium_group_actions_text.chars().count()
        + medium_toggle_group_text.chars().count()
        + medium_group_mark_toggle_text.chars().count()
        + ribbon_paddings_len
        + 1; // 1 for the end padding

    // short
    let short_selected_panes_text = if common_modifier_text.is_empty() {
        format!("{} SELECTED", grouped_pane_count)
    } else {
        format!("{} SELECTED |", grouped_pane_count)
    };
    let short_group_actions_text = format!("<{}>", &multiple_select_key);
    let short_toggle_group_text = format!("<{}>", &pane_group_toggle_key);
    let short_group_mark_toggle_text = format!("<{}>", &group_mark_toggle_key);
    let color_emphasis_range_end = if common_modifier_text.is_empty() {
        0
    } else {
        2
    };
    let ribbon_paddings_len = 12;
    let short_controls_line_len = short_selected_panes_text.chars().count()
        + 1
        + common_modifier_text.chars().count()
        + short_group_actions_text.chars().count()
        + short_toggle_group_text.chars().count()
        + short_group_mark_toggle_text.chars().count()
        + ribbon_paddings_len
        + 1; // 1 for the end padding

    let (
        selected_panes_text,
        group_actions_text,
        toggle_group_text,
        group_mark_toggle_text,
        controls_line_len,
    ) = if max_len >= full_controls_line_len {
        (
            full_selected_panes_text,
            full_group_actions_text,
            full_toggle_group_text,
            full_group_mark_toggle_text,
            full_controls_line_len,
        )
    } else if max_len >= medium_controls_line_len {
        (
            medium_selected_panes_text,
            medium_group_actions_text,
            medium_toggle_group_text,
            medium_group_mark_toggle_text,
            medium_controls_line_len,
        )
    } else if max_len >= short_controls_line_len {
        (
            short_selected_panes_text,
            short_group_actions_text,
            short_toggle_group_text,
            short_group_mark_toggle_text,
            short_controls_line_len,
        )
    } else {
        return None;
    };
    let selected_panes = serialize_text(
        &Text::new(&selected_panes_text)
            .color_range(
                3,
                ..selected_panes_text
                    .chars()
                    .count()
                    .saturating_sub(color_emphasis_range_end),
            )
            .opaque(),
    );
    let group_actions_ribbon = serialize_ribbon(
        &Text::new(&group_actions_text).color_range(0, 1..=multiple_select_key.chars().count()),
    );
    let toggle_group_ribbon = serialize_ribbon(
        &Text::new(&toggle_group_text).color_range(0, 1..=pane_group_toggle_key.chars().count()),
    );
    let mut group_mark_toggle_ribbon = Text::new(&group_mark_toggle_text)
        .color_range(0, 1..=group_mark_toggle_key.chars().count());
    if currently_marking_group {
        group_mark_toggle_ribbon = group_mark_toggle_ribbon.selected();
    }
    let group_mark_toggle_ribbon = serialize_ribbon(&group_mark_toggle_ribbon);
    let controls_line = if common_modifiers.is_empty() {
        format!(
            "{} {}{}{}",
            selected_panes, group_actions_ribbon, toggle_group_ribbon, group_mark_toggle_ribbon
        )
    } else {
        let common_modifier =
            serialize_text(&Text::new(&common_modifier_text).color_range(0, ..).opaque());
        format!(
            "{} {}{}{}{}",
            selected_panes,
            common_modifier,
            group_actions_ribbon,
            toggle_group_ribbon,
            group_mark_toggle_ribbon
        )
    };
    let remaining_space = max_len.saturating_sub(controls_line_len);
    let mut padding = String::new();
    let mut padding_len = 0;
    for _ in 0..remaining_space {
        padding.push_str(&ANSIStrings(&[superkey_prefix_style.paint(" ")]).to_string());
        padding_len += 1;
    }
    Some(LinePart {
        part: format!("{}{}", padding, controls_line),
        len: controls_line_len + padding_len,
        tab_index: None,
    })
}

fn multiple_select_key(keymap: &[(KeyWithModifier, Vec<Action>)]) -> Vec<KeyWithModifier> {
    let mut matching = keymap.iter().find_map(|(key, acvec)| {
        let has_match = acvec
            .iter()
            .find(|a| a.launches_plugin("zellij:multiple-select")) // TODO: make this an alias
            .is_some();
        if has_match {
            Some(key.clone())
        } else {
            None
        }
    });
    if let Some(matching) = matching.take() {
        vec![matching]
    } else {
        vec![]
    }
}

fn single_action_key(
    keymap: &[(KeyWithModifier, Vec<Action>)],
    action: &[Action],
) -> Vec<KeyWithModifier> {
    let mut matching = keymap.iter().find_map(|(key, acvec)| {
        if acvec.iter().next() == action.iter().next() {
            Some(key.clone())
        } else {
            None
        }
    });
    if let Some(matching) = matching.take() {
        vec![matching]
    } else {
        vec![]
    }
}

fn get_common_modifiers(mut keyvec: Vec<&KeyWithModifier>) -> Vec<KeyModifier> {
    if keyvec.is_empty() {
        return vec![];
    }
    let mut common_modifiers = keyvec.pop().unwrap().key_modifiers.clone();
    for key in keyvec {
        common_modifiers = common_modifiers
            .intersection(&key.key_modifiers)
            .cloned()
            .collect();
    }
    common_modifiers.into_iter().collect()
}
