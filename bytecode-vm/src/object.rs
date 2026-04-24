use std::ptr::NonNull;

/// Identifies the concrete payload stored after a shared object header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjType {
    String,
}

/// Common heap object header shared by every heap-allocated Lox object.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Obj {
    obj_type: ObjType,
}

impl Obj {
    pub(crate) const fn obj_type(&self) -> ObjType {
        self.obj_type
    }
}

/// Heap-allocated string object payload.
///
/// The shared `Obj` header stays first so an `ObjString` can be treated as an
/// `Obj` header by pointer casts, mirroring the book's struct inheritance.
#[allow(dead_code)]
#[repr(C)]
#[derive(Debug)]
pub(crate) struct ObjString {
    obj: Obj,
    length: usize,
    chars: Box<str>,
}

impl ObjString {
    #[allow(dead_code)]
    pub(crate) const fn len(&self) -> usize {
        self.length
    }

    pub(crate) fn as_str(&self) -> &str {
        self.chars.as_ref()
    }
}

/// Non-null reference to a heap-allocated Lox object managed by the VM.
///
/// The pointer is expected to reference a live object header. Ownership stays
/// with the VM and, later, the garbage collector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObjRef(NonNull<Obj>);

impl ObjRef {
    pub(crate) fn obj_type(self) -> ObjType {
        // ObjRef's invariant is that the pointer targets a live Obj header.
        unsafe { self.0.as_ref().obj_type() }
    }

    pub(crate) fn is_type(self, obj_type: ObjType) -> bool {
        self.obj_type() == obj_type
    }

    pub(crate) fn as_string(&self) -> Option<&ObjString> {
        if !self.is_type(ObjType::String) {
            return None;
        }

        // ObjString is repr(C) and starts with Obj, so both pointers share the
        // same address when the type tag says this object is a string.
        unsafe { Some(self.0.cast::<ObjString>().as_ref()) }
    }
}

#[cfg(test)]
impl ObjRef {
    pub(crate) fn string_for_tests(chars: &str) -> Self {
        let string = Box::leak(Box::new(ObjString {
            obj: Obj {
                obj_type: ObjType::String,
            },
            length: chars.len(),
            chars: chars.into(),
        }));

        Self(NonNull::from(string).cast())
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjRef, ObjType};

    #[test]
    fn string_objects_carry_their_base_type_tag() {
        let object = ObjRef::string_for_tests("hello");

        assert_eq!(object.obj_type(), ObjType::String);
        assert!(object.is_type(ObjType::String));

        let string = object.as_string().expect("object should be a string");
        assert_eq!(string.len(), 5);
        assert_eq!(string.as_str(), "hello");
    }
}
