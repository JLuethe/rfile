// SPDX-License-Identifier: GPL-3.0-only
use crate::{Error, Result, UPSTREAM_COMMIT};
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SemanticProfile {
    #[default]
    Pinned548,
    CorrectedV1,
}
impl SemanticProfile {
    pub fn name(self) -> &'static str {
        match self {
            Self::Pinned548 => "pinned-5.48",
            Self::CorrectedV1 => "corrected-v1",
        }
    }
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "pinned-5.48" => Ok(Self::Pinned548),
            "corrected-v1" => Ok(Self::CorrectedV1),
            _ => Err(Error::Unsupported(format!("semantic profile {name}"))),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Capabilities {
    pub engine_version: &'static str,
    pub upstream_commit: &'static str,
    pub database_format: u32,
    pub profile: SemanticProfile,
    pub codecs: &'static [&'static str],
    pub regex_backreferences: bool,
    pub c_abi: bool,
    pub kernel_sandbox: bool,
    pub display_locale: String,
    pub time_zone: String,
}
impl Capabilities {
    pub(crate) fn current(profile: SemanticProfile) -> Self {
        Self {
            engine_version: env!("CARGO_PKG_VERSION"),
            upstream_commit: UPSTREAM_COMMIT,
            database_format: crate::FORMAT_VERSION,
            profile,
            codecs: crate::codecs::COMPILED,
            regex_backreferences: true,
            c_abi: false,
            kernel_sandbox: cfg!(all(
                feature = "sandbox",
                target_os = "linux",
                target_env = "gnu",
                target_arch = "x86_64"
            )),
            display_locale: "C".into(),
            time_zone: "UTC".into(),
        }
    }
    pub fn json(&self) -> String {
        serde_json::json!({"schema_version":1,
        "engine_version":self.engine_version,"upstream_commit":self.upstream_commit,
        "database_format":self.database_format,"profile":self.profile.name(),"codecs":self.codecs,
        "regex_backreferences":self.regex_backreferences,
        "regex":{"capture_groups":32,"instructions":65536,"pinned_quantified_backreferences":"restricted","corrected_capture_histories":true},"c_abi":self.c_abi,
        "sandbox":{"compiled":self.kernel_sandbox,"enforced":false},
        "display_locale":self.display_locale,"display_backend":if self.display_locale=="C"{"portable C bytes"}else{"glibc locale_t"},"time_zone":self.time_zone,
        "hardening":{"sustained_fuzz_gate":false,"independent_security_review":false}
    }).to_string()
    }
}
