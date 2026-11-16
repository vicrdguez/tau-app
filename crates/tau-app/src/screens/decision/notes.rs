//! Note drafts, kept on this device per decision until a `result` of `answer` carries them
//! (`spec.md` §5.6, §3.3). Each change is one undo step; every change is saved at once.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use serde::{Deserialize, Serialize};
use tau_client::DecisionRef;
use tau_client::store::Store;
use tau_client::wire::{Annotation, TextRange};

const FILE: &str = "decision-notes.json";

/// One note: on a range of the basis (UTF-16 code units of `text`), or on the whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Range<u64>>,
    /// The selection crossed markup or blocks and took the blocks it touched.
    #[serde(default)]
    pub widened: bool,
}

impl Draft {
    pub fn annotation(&self) -> Annotation {
        Annotation {
            text: self.text.clone(),
            range: self.range.clone().map(|r| TextRange {
                start: r.start,
                end: r.end,
            }),
        }
    }

    pub fn from_annotation(a: &Annotation) -> Draft {
        Draft {
            text: a.text.clone(),
            range: a.range.as_ref().map(|r| r.start..r.end),
            widened: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Saved {
    /// By `project/decision`.
    drafts: BTreeMap<String, Vec<Draft>>,
    /// Notes written on this device: the `+ note c` ghost shows until 3.
    written: u32,
}

#[derive(Default)]
pub struct Notes {
    saved: Saved,
    loaded: bool,
    undo: HashMap<DecisionRef, Vec<Vec<Draft>>>,
    redo: HashMap<DecisionRef, Vec<Vec<Draft>>>,
}

fn key(d: &DecisionRef) -> String {
    format!("{}/{}", d.project, d.id)
}

/// Passage notes in text order, then the note on the whole: the order the readback reads and
/// the answer sends.
fn ordered(mut drafts: Vec<Draft>) -> Vec<Draft> {
    drafts.retain(|d| !d.text.trim().is_empty());
    drafts.sort_by_key(|d| d.range.as_ref().map_or((1, 0, 0), |r| (0, r.start, r.end)));
    drafts
}

impl Notes {
    /// Reads this server's drafts once.
    pub fn load(&mut self, store: &Store) {
        if !self.loaded {
            self.saved = store.load(FILE).unwrap_or_default();
            self.loaded = true;
        }
    }

    /// Sets drafts without an undo step or a save: what a scene starts from.
    pub fn seed(&mut self, d: &DecisionRef, drafts: Vec<Draft>) {
        self.loaded = true;
        self.saved.drafts.insert(key(d), ordered(drafts));
    }

    pub fn of(&self, d: &DecisionRef) -> &[Draft] {
        self.saved.drafts.get(&key(d)).map_or(&[], Vec::as_slice)
    }

    pub fn count(&self) -> usize {
        self.saved.drafts.values().map(Vec::len).sum()
    }

    pub fn written(&self) -> u32 {
        self.saved.written
    }

    pub fn annotations(&self, d: &DecisionRef) -> Vec<Annotation> {
        self.of(d).iter().map(Draft::annotation).collect()
    }

    fn save(&self, store: &Store) {
        // A failed save keeps the drafts in memory; the next change tries again.
        let _ = store.save(FILE, &self.saved);
    }

    /// Replaces the decision's drafts as one undo step.
    pub fn set(&mut self, d: &DecisionRef, drafts: Vec<Draft>, store: &Store) {
        let before = self.of(d).to_vec();
        let after = ordered(drafts);
        if before == after {
            return;
        }
        self.undo.entry(d.clone()).or_default().push(before);
        self.redo.remove(d);
        self.saved.drafts.insert(key(d), after);
        self.save(store);
    }

    /// A new note: one more toward the ghost's three.
    pub fn add(&mut self, d: &DecisionRef, draft: Draft, store: &Store) {
        let mut all = self.of(d).to_vec();
        all.push(draft);
        self.saved.written += 1;
        self.set(d, all, store);
    }

    /// `u`: false when there is nothing to undo.
    pub fn undo(&mut self, d: &DecisionRef, store: &Store) -> bool {
        self.step(d, store, true)
    }

    /// `C-r`.
    pub fn redo(&mut self, d: &DecisionRef, store: &Store) -> bool {
        self.step(d, store, false)
    }

    fn step(&mut self, d: &DecisionRef, store: &Store, back: bool) -> bool {
        let (from, to) = if back {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        let Some(prev) = from.get_mut(d).and_then(Vec::pop) else {
            return false;
        };
        let now = self.saved.drafts.insert(key(d), prev).unwrap_or_default();
        to.entry(d.clone()).or_default().push(now);
        self.save(store);
        true
    }

    /// A `result` of `answer` carried them.
    pub fn clear(&mut self, d: &DecisionRef, store: &Store) {
        self.saved.drafts.remove(&key(d));
        self.undo.remove(d);
        self.redo.remove(d);
        self.save(store);
    }
}
