// The VM starts with numeric-only values and can widen this representation later.
pub(crate) type Value = f64;

pub(crate) fn print_value(value: Value) {
    print!("{value}");
}
