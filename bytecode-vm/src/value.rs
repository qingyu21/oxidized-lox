use std::fmt;

use crate::object::{ObjRef, ObjString, ObjType};

/// Runtime value representation for the current VM chapter stage.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum Value {
    Bool(bool),
    #[default]
    Nil,
    Number(f64),
    // Wired before concrete heap object constructors exist.
    #[allow(dead_code)]
    Obj(ObjRef),
}

impl Value {
    pub(crate) const fn number(value: f64) -> Self {
        Self::Number(value)
    }

    pub(crate) fn as_number(self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(value),
            Self::Bool(_) | Self::Nil | Self::Obj(_) => None,
        }
    }

    pub(crate) const fn is_falsey(self) -> bool {
        match self {
            Self::Bool(value) => !value,
            Self::Nil => true,
            Self::Number(_) | Self::Obj(_) => false,
        }
    }

    pub(crate) fn equals(self, other: Self) -> bool {
        match (self, other) {
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Nil, Self::Nil) => true,
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::Obj(left), Self::Obj(right)) => left == right,
            (Self::Bool(_) | Self::Nil | Self::Number(_) | Self::Obj(_), _) => false,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn obj_type(self) -> Option<ObjType> {
        match self {
            Self::Obj(object) => Some(object.obj_type()),
            Self::Bool(_) | Self::Nil | Self::Number(_) => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn is_string(self) -> bool {
        self.is_obj_type(ObjType::String)
    }

    #[allow(dead_code)]
    pub(crate) fn as_string(&self) -> Option<&ObjString> {
        match self {
            Self::Obj(object) => object.as_string(),
            Self::Bool(_) | Self::Nil | Self::Number(_) => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_str(&self) -> Option<&str> {
        self.as_string().map(ObjString::as_str)
    }

    fn is_obj_type(self, obj_type: ObjType) -> bool {
        matches!(self, Self::Obj(object) if object.is_type(obj_type))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::number(value)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Nil => write!(f, "nil"),
            Self::Number(value) => write!(f, "{value}"),
            Self::Obj(_) => write!(f, "<obj>"),
        }
    }
}

pub(crate) fn print_value(value: Value) {
    print!("{value}");
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::object::{ObjRef, ObjType};

    fn object() -> Value {
        Value::Obj(ObjRef::string_for_tests("hello"))
    }

    #[test]
    fn number_values_round_trip_through_as_number() {
        assert_eq!(Value::number(3.5).as_number(), Some(3.5));
        assert_eq!(Value::Bool(true).as_number(), None);
        assert_eq!(Value::Nil.as_number(), None);
        assert_eq!(object().as_number(), None);
    }

    #[test]
    fn only_false_and_nil_are_falsey() {
        assert!(Value::Bool(false).is_falsey());
        assert!(Value::Nil.is_falsey());
        assert!(!Value::Bool(true).is_falsey());
        assert!(!Value::number(0.0).is_falsey());
        assert!(!object().is_falsey());
    }

    #[test]
    fn values_equal_only_when_types_and_inner_values_match() {
        assert!(Value::Bool(true).equals(Value::Bool(true)));
        assert!(Value::Nil.equals(Value::Nil));
        assert!(Value::number(1.2).equals(Value::number(1.2)));
        assert!(!Value::Bool(true).equals(Value::Bool(false)));
        assert!(!Value::Nil.equals(Value::Bool(false)));
        assert!(!Value::number(f64::NAN).equals(Value::number(f64::NAN)));
        let object = object();
        assert!(object.equals(object));
        assert!(!object.equals(Value::Nil));
    }

    #[test]
    fn string_object_values_report_their_object_type() {
        let object = object();

        assert_eq!(object.obj_type(), Some(ObjType::String));
        assert!(object.is_string());
        assert_eq!(object.as_str(), Some("hello"));
        assert_eq!(Value::Nil.obj_type(), None);
        assert!(!Value::Nil.is_string());
        assert_eq!(Value::Nil.as_str(), None);
    }

    #[test]
    fn display_matches_lox_literal_spellings() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Nil.to_string(), "nil");
        assert_eq!(Value::number(12.5).to_string(), "12.5");
        assert_eq!(object().to_string(), "<obj>");
    }
}
