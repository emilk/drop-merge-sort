// By Emil Ernerfeldt December 2016

pub use dmsort::{sort, sort_by, sort_by_key};

/// For in module-level testing only. TODO: this shouldn't be public.
pub use dmsort::{sort_copy};

mod dmsort;
