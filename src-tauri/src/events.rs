use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgress {
    pub scanned: usize,
    pub total: usize,
}

pub const SCAN_PROGRESS: &str = "scan-progress";
