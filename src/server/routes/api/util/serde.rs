use std::cell::Cell;

use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct SerializeFromIter<I>(Cell<Option<I>>);

impl<I> SerializeFromIter<I> {
    pub fn new(iter: I) -> Self {
        Self(Cell::new(Some(iter)))
    }
}

impl<I, T> Serialize for SerializeFromIter<I>
where
    I: IntoIterator<Item = T>,
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0.take() {
            None => serializer.serialize_none(),
            Some(seq) => {
                let mut iter = seq.into_iter();
                let mut seq = serializer.serialize_seq(iter.size_hint().1)?;

                while let Some(value) = iter.next() {
                    seq.serialize_element(&value)?;
                }

                seq.end()
            }
        }
    }
}
