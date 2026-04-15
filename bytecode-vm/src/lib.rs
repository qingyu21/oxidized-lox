// These modules are still being wired into the executable path, so we suppress
// dead-code noise while the VM scaffold is taking shape.
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
