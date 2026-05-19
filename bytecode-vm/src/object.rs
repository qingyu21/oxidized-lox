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
    pub(crate) fn take_string(chars: Box<str>) -> Self {
        let length = chars.len();
        let string = Box::leak(Box::new(ObjString {
            obj: Obj {
                obj_type: ObjType::String,
            },
            length,
            chars,
        }));

        Self(NonNull::from(string).cast())
    }

    pub(crate) fn copy_string(chars: &str) -> Self {
        Self::take_string(chars.into())
    }

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
mod tests {
    use super::{ObjRef, ObjType};

    #[test]
    fn string_objects_carry_their_base_type_tag() {
        let object = ObjRef::copy_string("hello");

        assert_eq!(object.obj_type(), ObjType::String);
        assert!(object.is_type(ObjType::String));

        let string = object.as_string().expect("object should be a string");
        assert_eq!(string.len(), 5);
        assert_eq!(string.as_str(), "hello");
    }

    #[test]
    fn take_string_claims_an_existing_boxed_string() {
        let object = ObjRef::take_string(String::from("owned").into_boxed_str());
        let string = object.as_string().expect("object should be a string");

        assert_eq!(string.as_str(), "owned");
    }
}
