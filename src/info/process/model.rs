/// Process facts relevant to selecting a foreground label.
#[derive(Debug, Clone)]
pub(super) struct ProcEntry {
    pub(super) pid: u32,
    pub(super) pgrp: u32,
    pub(super) comm: String,
    pub(super) argv: Vec<String>,
}
