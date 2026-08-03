use super::model::{SidebarContentRow, TabCard};

pub(super) fn sidebar_content_rows(
    cards: &[TabCard],
    content_height: usize,
) -> Vec<SidebarContentRow> {
    let show_heading = content_height > usize::from(!cards.is_empty());
    let mut rows = Vec::new();
    if show_heading {
        rows.push(SidebarContentRow::chrome(4, "tabs"));
    }
    rows.extend(card_viewport_rows(
        cards,
        content_height.saturating_sub(rows.len()),
    ));
    rows
}

/// Select a compact, whole-card viewport containing the active tab.
pub(super) fn card_viewport_rows(cards: &[TabCard], capacity: usize) -> Vec<SidebarContentRow> {
    if cards.is_empty() || capacity == 0 {
        return Vec::new();
    }

    let active_index = cards.iter().position(|card| card.active).unwrap_or(0);
    let active = &cards[active_index];
    if active.lines.len() >= capacity {
        return card_rows(active).take(capacity).collect();
    }

    let mut start = active_index;
    let mut end = active_index + 1;
    let mut used = active.lines.len();
    while start > 0 {
        let previous_len = cards[start - 1].lines.len();
        if used + previous_len > capacity {
            break;
        }
        start -= 1;
        used += previous_len;
    }
    while end < cards.len() {
        let next_len = cards[end].lines.len();
        if used + next_len > capacity {
            break;
        }
        used += next_len;
        end += 1;
    }

    cards[start..end].iter().flat_map(card_rows).collect()
}

fn card_rows(card: &TabCard) -> impl Iterator<Item = SidebarContentRow> + '_ {
    card.lines
        .iter()
        .map(|(kind, text)| SidebarContentRow::card(card, *kind, text))
}
