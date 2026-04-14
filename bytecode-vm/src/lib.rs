#[allow(dead_code)]
pub(crate) mod chunk;
#[allow(dead_code)]
pub(crate) mod debug;
#[allow(dead_code)]
pub(crate) mod value;

/// Returns the current implementation status of the bytecode VM crate.
pub fn status() -> &'static str {
    "bytecode-vm scaffold: chunk support in progress"
}
