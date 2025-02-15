use std::rc::Rc;

#[derive(Clone)]
pub enum Scope {
    Array(usize),
    Object,
    ObjectAtKey {
        /// The index number of this key in the object
        index: usize,
        key: Rc<str>,
    },
}
