//! UNION machine fingerprint capture (spec: [Platform honesty]).
//!
//! Every source is attempted and recorded. Critical sources (arch, cpu model,
//! kernel, vcpus) failing is a hard error; soft sources (image version, VM
//! size, MHz) record as explicit gaps, and `--strict` ledger writes refuse
//! gapped fingerprints (see `probe-record`). No silent fallbacks: the ARM
//! model composite records WHICH alternative produced it.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::process::Command;

/// Complete UNION fingerprint. `None` fields are RECORDED gaps, never silent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fingerprint {
    pub arch: String,
    pub cpu_model: String,
    pub cpu_model_source: String,
    pub cpu_flags: Vec<String>,
    pub cpu_mhz: Option<String>,
    pub kernel: String,
    pub image_version: Option<String>,
    pub vm_size: Option<String>,
    pub vcpus: u32,
}

impl Fingerprint {
    /// Sources required for a strict ledger write. Informational fields
    /// (flags content, MHz) are recorded but never gate.
    pub fn missing_sources(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.cpu_model.is_empty() {
            out.push("cpu_model");
        }
        if self.image_version.is_none() {
            out.push("image_version");
        }
        if self.vm_size.is_none() {
            out.push("vm_size");
        }
        out
    }

    pub fn is_complete(&self) -> bool {
        self.missing_sources().is_empty()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FingerprintError {
    CommandFailed {
        command: &'static str,
        detail: String,
    },
    MissingCritical {
        source: &'static str,
    },
    Parse {
        source: &'static str,
        detail: String,
    },
}

impl fmt::Display for FingerprintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CommandFailed { command, detail } => {
                write!(f, "command failed: {command}: {detail}")
            }
            Self::MissingCritical { source } => {
                write!(f, "critical source unavailable: {source}")
            }
            Self::Parse { source, detail } => {
                write!(f, "parse failed for {source}: {detail}")
            }
        }
    }
}

impl std::error::Error for FingerprintError {}

fn run(command: &'static str, args: &[&str]) -> Result<String, FingerprintError> {
    let out = Command::new(command).args(args).output().map_err(|e| {
        FingerprintError::CommandFailed {
            command,
            detail: e.to_string(),
        }
    })?;
    if !out.status.success() {
        return Err(FingerprintError::CommandFailed {
            command,
            detail: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    String::from_utf8(out.stdout).map_err(|e| FingerprintError::Parse {
        source: command,
        detail: e.to_string(),
    })
}

/// Parse `lscpu --json` output into field -> data (field names normalized:
/// trailing ':' trimmed so text/JSON variants agree).
pub fn parse_lscpu_json(text: &str) -> Result<HashMap<String, String>, FingerprintError> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| FingerprintError::Parse {
            source: "lscpu",
            detail: e.to_string(),
        })?;
    let arr = v.get("lscpu").and_then(|a| a.as_array()).ok_or_else(|| {
        FingerprintError::Parse {
            source: "lscpu",
            detail: "missing lscpu array".to_string(),
        }
    })?;
    let mut map = HashMap::new();
    for entry in arr {
        let field = entry
            .get("field")
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .trim_end_matches(':')
            .to_string();
        let data = entry
            .get("data")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        if !field.is_empty() {
            map.insert(field, data);
        }
    }
    Ok(map)
}

/// Resolve the CPU model + record which alternative produced it.
/// x86 carries "Model name"; ARM64 often lacks it and reports numeric
/// Model/Family/Stepping instead — the composite keeps attribution honest.
pub fn resolve_cpu_model(map: &HashMap<String, String>) -> Option<(String, String)> {
    if let Some(name) = map.get("Model name").filter(|s| !s.is_empty()) {
        return Some((name.clone(), "lscpu:Model name".to_string()));
    }
    let family = map.get("BIOS CPU family").filter(|s| !s.is_empty());
    let model = map
        .get("Model")
        .filter(|s| !s.is_empty())
        .or_else(|| map.get("BIOS Model").filter(|s| !s.is_empty()));
    let stepping = map.get("Stepping").filter(|s| !s.is_empty());
    match (family, model, stepping) {
        (Some(f), Some(m), Some(s)) => Some((
            format!("family:{f} model:{m} stepping:{s}"),
            "lscpu:BIOS-composite".to_string(),
        )),
        (None, Some(m), _) => Some((
            format!("model:{m}"),
            "lscpu:Model-numeric".to_string(),
        )),
        _ => None,
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// IMDS `vmSize` is plain text. Any transport failure is the caller's gap.
fn fetch_imds_vmsize() -> Option<String> {
    let out = Command::new("curl")
        .args([
            "-s",
            "-m",
            "5",
            "-H",
            "Metadata:true",
            "http://169.254.169.254/metadata/instance/compute/vmSize?api-version=2021-02-01&format=text",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    non_empty(&String::from_utf8_lossy(&out.stdout))
}

/// Collect the UNION fingerprint. Critical sources (arch, kernel, vcpus,
/// cpu model) failing is a hard error; soft sources record as gaps.
pub fn collect() -> Result<Fingerprint, FingerprintError> {
    let lscpu = run("lscpu", &["--json"])?;
    let map = parse_lscpu_json(&lscpu)?;
    let arch = run("uname", &["-m"])?;
    let kernel = run("uname", &["-r"])?;
    let vcpus = std::thread::available_parallelism()
        .map_err(|e| FingerprintError::CommandFailed {
            command: "available_parallelism",
            detail: e.to_string(),
        })?
        .get() as u32;
    let (cpu_model, cpu_model_source) =
        resolve_cpu_model(&map).ok_or(FingerprintError::MissingCritical {
            source: "cpu_model",
        })?;
    let arch = arch.trim().to_string();
    let kernel = kernel.trim().to_string();
    if arch.is_empty() {
        return Err(FingerprintError::MissingCritical { source: "arch" });
    }
    if kernel.is_empty() {
        return Err(FingerprintError::MissingCritical { source: "kernel" });
    }
    let cpu_flags: Vec<String> = map
        .get("Flags")
        .map(|f| f.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    let cpu_mhz = map.get("CPU MHz").and_then(|s| non_empty(s)).or_else(|| {
        map.get("CPU max MHz")
            .and_then(|s| non_empty(s))
    });
    // GH runners export ImageVersion/ImageOS env vars; absent elsewhere.
    // If 1.3 probes discover a file source, it extends this function.
    let image_version = std::env::var("ImageVersion").ok().and_then(|s| non_empty(&s));
    let vm_size = fetch_imds_vmsize();
    Ok(Fingerprint {
        arch,
        cpu_model,
        cpu_model_source,
        cpu_flags,
        cpu_mhz,
        kernel,
        image_version,
        vm_size,
        vcpus,
    })
}
