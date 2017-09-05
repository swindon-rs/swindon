use std::collections::{HashMap, HashSet};
use std::hash::Hash;


pub trait PairCollection<A, B> {
    fn insert_clone(&mut self, key1: &A, key2: &B)
        where A: Clone, B: Clone;
    fn insert_list_key1<'x, I>(&mut self, key1list: I, key2: &B)
        where I: IntoIterator<Item=&'x A>,
              A: Clone + 'x,
              B: Clone
    {
        for key1 in key1list {
            self.insert_clone(key1, key2);
        }
    }
    fn remove(&mut self, key1: &A, key2: &B);
    fn remove_list_key1<'x, I>(&mut self, key1list: I, key2: &B)
        where I: IntoIterator<Item=&'x A>,
              A: 'x
    {
        for key1 in key1list {
            self.remove(key1, key2);
        }
    }
}

impl<A, B> PairCollection<A, B> for HashMap<A, HashSet<B>>
    where A: Eq + Hash,
          B: Eq + Hash,
{
    fn insert_clone(&mut self, key1: &A, key2: &B)
        where A: Clone, B: Clone
    {
        self.entry(key1.clone())
            .or_insert_with(HashSet::new)
            .insert(key2.clone());
    }
    fn remove(&mut self, key1: &A, key2: &B) {
        let len = if let Some(sub) = self.get_mut(key1) {
            sub.remove(key2);
            sub.len()
        } else {
            return;
        };
        if len == 0 {
            self.remove(key1);
        }
    }
}
