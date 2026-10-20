use std::collections::{BTreeMap, BTreeSet};

use crate::launcher_gui::model::GameId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Node {
    id: GameId,
    previous: usize,
    next: usize,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CircularGameList {
    nodes: Vec<Node>,
    by_id: BTreeMap<GameId, usize>,
    head: Option<usize>,
    selected: Option<usize>,
}

impl CircularGameList {
    pub fn rebuild(&mut self, ids: impl IntoIterator<Item = GameId>) {
        let ids: Vec<GameId> = ids.into_iter().collect();
        let new_ids: BTreeSet<GameId> = ids.iter().copied().collect();
        let old_selected = self.selected();
        let fallback = old_selected
            .filter(|selected| !new_ids.contains(selected))
            .and_then(|selected| self.fallback_after_removal(selected, &new_ids));
        let wanted = old_selected.filter(|id| new_ids.contains(id)).or(fallback);

        let count = ids.len();
        self.nodes = ids
            .into_iter()
            .enumerate()
            .map(|(index, id)| Node {
                id,
                previous: if count == 0 {
                    0
                } else {
                    (index + count - 1) % count
                },
                next: if count == 0 { 0 } else { (index + 1) % count },
            })
            .collect();
        self.by_id = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id, index))
            .collect();
        self.head = (!self.nodes.is_empty()).then_some(0);
        self.selected = wanted
            .and_then(|id| self.by_id.get(&id).copied())
            .or(self.head);
    }

    fn fallback_after_removal(
        &self,
        removed: GameId,
        survivors: &BTreeSet<GameId>,
    ) -> Option<GameId> {
        let start = *self.by_id.get(&removed)?;
        for node in self.nodes.iter().skip(start + 1) {
            let id = node.id;
            if survivors.contains(&id) {
                return Some(id);
            }
        }
        for node in self.nodes[..start].iter().rev() {
            let id = node.id;
            if survivors.contains(&id) {
                return Some(id);
            }
        }
        None
    }

    pub fn selected(&self) -> Option<GameId> {
        self.selected.map(|index| self.nodes[index].id)
    }

    pub fn select_next(&mut self) -> Option<GameId> {
        self.selected = self.selected.map(|index| self.nodes[index].next);
        self.selected()
    }

    pub fn select_previous(&mut self) -> Option<GameId> {
        self.selected = self.selected.map(|index| self.nodes[index].previous);
        self.selected()
    }

    pub fn visible_window(&self, maximum_rows: usize) -> Vec<GameId> {
        let Some(selected) = self.selected else {
            return Vec::new();
        };
        let count = maximum_rows.min(self.nodes.len());
        if count == 0 {
            return Vec::new();
        }
        let mut start = selected;
        for _ in 0..(count / 2) {
            start = self.nodes[start].previous;
        }
        let mut result = Vec::with_capacity(count);
        let mut cursor = start;
        for _ in 0..count {
            result.push(self.nodes[cursor].id);
            cursor = self.nodes[cursor].next;
        }
        result
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/game_launcher_screen/domain/circular_game_list__tests.rs"]
mod tests;
