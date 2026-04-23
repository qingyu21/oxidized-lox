use std::fmt;

/// Runtime value representation for the current VM chapter stage.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum Value {
    Bool(bool),
    #[default]
    Nil,
    Number(f64),
}

impl Value {
    pub(crate) const fn number(value: f64) -> Self {
        Self::Number(value)
    }

    pub(crate) fn as_number(self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(value),
            Self::Bool(_) | Self::Nil => None,
        }
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
        }
    }
}

pub(crate) fn print_value(value: Value) {
    print!("{value}");
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn number_values_round_trip_through_as_number() {
        assert_eq!(Value::number(3.5).as_number(), Some(3.5));
        assert_eq!(Value::Bool(true).as_number(), None);
        assert_eq!(Value::Nil.as_number(), None);
    }

    #[test]
    fn display_matches_lox_literal_spellings() {
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Nil.to_string(), "nil");
        assert_eq!(Value::number(12.5).to_string(), "12.5");
    }
}
