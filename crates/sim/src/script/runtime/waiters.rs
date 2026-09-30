use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::script::{Value, Waiter};

/// Notify wakes in registration order, so both indexes keep it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Waiters {
    next: u64,
    rows: BTreeMap<u64, Waiter>,
    by_event: BTreeMap<(u64, Arc<str>), BTreeSet<u64>>,
    by_thread: BTreeMap<u64, BTreeSet<u64>>,
}

fn object_id(receiver: &Value) -> Option<u64> {
    match receiver {
        Value::Object(id) => Some(*id),
        _ => None,
    }
}

impl Waiters {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Waiter> {
        self.rows.values()
    }

    pub(crate) fn push(&mut self, waiter: Waiter) {
        let seq = self.next;
        self.next += 1;
        if let Some(id) = object_id(&waiter.receiver) {
            self.by_event
                .entry((id, waiter.name.clone()))
                .or_default()
                .insert(seq);
        }
        self.by_thread.entry(waiter.thread).or_default().insert(seq);
        self.rows.insert(seq, waiter);
    }

    fn remove(&mut self, seq: u64) -> Option<Waiter> {
        let waiter = self.rows.remove(&seq)?;
        if let Some(id) = object_id(&waiter.receiver) {
            let key = (id, waiter.name.clone());
            if let Some(seqs) = self.by_event.get_mut(&key) {
                seqs.remove(&seq);
                if seqs.is_empty() {
                    self.by_event.remove(&key);
                }
            }
        }
        if let Some(seqs) = self.by_thread.get_mut(&waiter.thread) {
            seqs.remove(&seq);
            if seqs.is_empty() {
                self.by_thread.remove(&waiter.thread);
            }
        }
        Some(waiter)
    }

    pub(crate) fn take_first(
        &mut self,
        receiver: &Value,
        name: &Arc<str>,
        accepts: impl Fn(&Waiter) -> bool,
    ) -> Option<Waiter> {
        let id = object_id(receiver)?;
        let seq = self
            .by_event
            .get(&(id, name.clone()))?
            .iter()
            .copied()
            .find(|seq| accepts(&self.rows[seq]))?;
        self.remove(seq)
    }

    pub(crate) fn any_on(
        &self,
        receiver: &Value,
        name: &str,
        matches: impl Fn(&Waiter) -> bool,
    ) -> bool {
        let Some(id) = object_id(receiver) else {
            return false;
        };
        self.by_event
            .get(&(id, Arc::from(name)))
            .is_some_and(|seqs| seqs.iter().any(|seq| matches(&self.rows[seq])))
    }

    pub(crate) fn of_thread(&self, serial: u64) -> impl Iterator<Item = &Waiter> {
        self.by_thread
            .get(&serial)
            .into_iter()
            .flatten()
            .map(move |seq| &self.rows[seq])
    }

    pub(crate) fn retain_thread(&mut self, serial: u64, keep: impl Fn(&Waiter) -> bool) {
        let Some(seqs) = self.by_thread.get(&serial) else {
            return;
        };
        let dropped: Vec<u64> = seqs
            .iter()
            .copied()
            .filter(|seq| !keep(&self.rows[seq]))
            .collect();
        for seq in dropped {
            self.remove(seq);
        }
    }

    pub(crate) fn receivers(&self) -> impl Iterator<Item = u64> + '_ {
        let mut last = None;
        self.by_event.keys().filter_map(move |(id, _)| {
            let fresh = last != Some(*id);
            last = Some(*id);
            fresh.then_some(*id)
        })
    }

    pub(crate) fn threads_on(&self, id: u64) -> impl Iterator<Item = u64> + '_ {
        self.by_event
            .range((id, Arc::<str>::from(""))..)
            .take_while(move |((receiver, _), _)| *receiver == id)
            .flat_map(move |(_, seqs)| seqs.iter().map(move |seq| self.rows[seq].thread))
    }
}
