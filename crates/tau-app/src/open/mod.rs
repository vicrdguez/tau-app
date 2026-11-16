//! The open items: what the sidebar lists under `OPEN`, grouped by project, in the order they
//! were opened (`spec.md` §4.1). Device-local; their labels and marks come from the server.

/// A conversation, decision or project you opened.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Item {
    Conversation {
        project: String,
        id: u64,
    },
    /// `id` is the decision's ID, `w1:3:design#2`.
    Decision {
        project: String,
        id: String,
    },
    Project(String),
}

impl Item {
    pub fn project(&self) -> &str {
        match self {
            Item::Conversation { project, .. }
            | Item::Decision { project, .. }
            | Item::Project(project) => project,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenItems {
    groups: Vec<(String, Vec<Item>)>,
}

impl OpenItems {
    /// Opens `item`: it joins the end of its project's group (a new group joins the end of the
    /// list). An item already open stays where it is.
    pub fn open(&mut self, item: Item) {
        if self.contains(&item) {
            return;
        }
        match self.groups.iter_mut().find(|(p, _)| p == item.project()) {
            Some((_, items)) => items.push(item),
            None => self.groups.push((item.project().to_string(), vec![item])),
        }
    }

    /// Closes `item` and returns what the stage shows instead if it was the one shown: the next
    /// item in the sidebar's order, else the previous one, else `None` (Home).
    pub fn close(&mut self, item: &Item) -> Option<Item> {
        let order: Vec<Item> = self.iter().cloned().collect();
        let at = order.iter().position(|i| i == item)?;
        for (_, items) in &mut self.groups {
            items.retain(|i| i != item);
        }
        self.groups.retain(|(_, items)| !items.is_empty());
        order
            .get(at + 1)
            .or_else(|| at.checked_sub(1).and_then(|p| order.get(p)))
            .cloned()
    }

    /// The item `by` steps from `from` in the sidebar's order, wrapping. From `None` (or an item
    /// no longer open) a forward step lands on the first and a backward one on the last.
    pub fn step(&self, from: Option<&Item>, by: i64) -> Option<Item> {
        let order: Vec<&Item> = self.iter().collect();
        let n = order.len() as i64;
        if n == 0 {
            return None;
        }
        let start = match from.and_then(|f| order.iter().position(|i| *i == f)) {
            Some(at) => at as i64 + by,
            None if by >= 0 => by - 1,
            None => n + by,
        };
        Some(order[start.rem_euclid(n) as usize].clone())
    }

    pub fn groups(&self) -> &[(String, Vec<Item>)] {
        &self.groups
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn contains(&self, item: &Item) -> bool {
        self.iter().any(|i| i == item)
    }

    /// Every item in the sidebar's order.
    pub fn iter(&self) -> impl Iterator<Item = &Item> {
        self.groups.iter().flat_map(|(_, items)| items)
    }
}

#[cfg(test)]
mod tests;
