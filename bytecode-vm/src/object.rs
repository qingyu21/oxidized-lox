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
    next: Option<ObjRef>,
}

impl Obj {
    const fn new(obj_type: ObjType, next: Option<ObjRef>) -> Self {
        Self { obj_type, next }
    }

    pub(crate) const fn obj_type(&self) -> ObjType {
        self.obj_type
    }

    const fn next(&self) -> Option<ObjRef> {
        self.next
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

    fn next(self) -> Option<Self> {
        // ObjRef's invariant is that the pointer targets a live Obj header.
        unsafe { self.0.as_ref().next() }
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

/// Owns every heap object allocated by a VM.
///
/// This mirrors the book's intrusive object list: each `Obj` header stores the
/// next pointer, and dropping this list frees every object at VM shutdown.
#[derive(Debug, Default)]
pub(crate) struct Objects {
    head: Option<ObjRef>,
}

impl Objects {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn take_string(&mut self, chars: Box<str>) -> ObjRef {
        let length = chars.len();
        let string = Box::new(ObjString {
            obj: Obj::new(ObjType::String, self.head),
            length,
            chars,
        });
        let object = ObjRef(NonNull::from(Box::leak(string)).cast());

        self.head = Some(object);
        object
    }

    pub(crate) fn copy_string(&mut self, chars: &str) -> ObjRef {
        self.take_string(chars.into())
    }

    fn free_objects(&mut self) {
        let mut object = self.head;

        while let Some(current) = object {
            object = current.next();
            free_object(current);
        }

        self.head = None;
    }
}

impl Drop for Objects {
    fn drop(&mut self) {
        self.free_objects();
    }
}

fn free_object(object: ObjRef) {
    match object.obj_type() {
        ObjType::String => {
            // ObjString is repr(C) and starts with Obj, so the object header
            // pointer is the original Box allocation address.
            unsafe {
                drop(Box::from_raw(object.0.cast::<ObjString>().as_ptr()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjType, Objects};

    fn count_objects(objects: &Objects) -> usize {
        let mut count = 0;
        let mut object = objects.head;

        while let Some(current) = object {
            count += 1;
            object = current.next();
        }

        count
    }

    #[test]
    fn string_objects_carry_their_base_type_tag() {
        let mut objects = Objects::new();
        let object = objects.copy_string("hello");

        assert_eq!(object.obj_type(), ObjType::String);
        assert!(object.is_type(ObjType::String));

        let string = object.as_string().expect("object should be a string");
        assert_eq!(string.len(), 5);
        assert_eq!(string.as_str(), "hello");
    }

    #[test]
    fn take_string_claims_an_existing_boxed_string() {
        let mut objects = Objects::new();
        let object = objects.take_string(String::from("owned").into_boxed_str());
        let string = object.as_string().expect("object should be a string");

        assert_eq!(string.as_str(), "owned");
    }

    #[test]
    fn allocated_objects_are_linked_for_later_freeing() {
        let mut objects = Objects::new();

        let first = objects.copy_string("first");
        let second = objects.copy_string("second");

        assert_eq!(count_objects(&objects), 2);
        assert_eq!(second.next(), Some(first));
    }
}
