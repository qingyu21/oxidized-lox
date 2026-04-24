use std::ptr::NonNull;

/// Common heap object header placeholder.
///
/// Concrete heap-allocated object types, such as strings, will grow from this
/// shared representation in later chapters.
#[derive(Debug)]
pub(crate) struct Obj {
    _private: (),
}

/// Non-null reference to a heap-allocated Lox object managed by the VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObjRef(NonNull<Obj>);

#[cfg(test)]
impl ObjRef {
    pub(crate) fn dangling_for_tests() -> Self {
        Self(NonNull::dangling())
    }
}
