//! Bounded story capability dispatch and deterministic replay validation.

use keygen_diagnostics::{Diagnostic, Severity, SOURCE_RUNTIME};
use keygen_engine::story::{Effect, TraceEvent, Value};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt::Write, sync::Arc};

pub const CAPABILITY_SCHEMA: &str = "keygen.story.capabilities.v1";
pub const CODE_CAPABILITY_MISSING: &str = "KGD-412";
pub const CODE_CAPABILITY_FAILED: &str = "KGD-413";
pub const CODE_REPLAY_MISMATCH: &str = "KGD-444";
pub const CODE_REPLAY_INVALID: &str = "KGD-445";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cancellation {
    cancelled: bool,
}
impl Cancellation {
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityRequest {
    pub id: String,
    pub args: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityResponse {
    pub effects: Vec<Effect>,
}

pub trait CapabilityHandler: Send + Sync {
    fn invoke(
        &self,
        request: CapabilityRequest,
        cancellation: Cancellation,
    ) -> Result<CapabilityResponse, String>;
}
impl<F> CapabilityHandler for F
where
    F: Fn(CapabilityRequest, Cancellation) -> Result<CapabilityResponse, String> + Send + Sync,
{
    fn invoke(
        &self,
        request: CapabilityRequest,
        cancellation: Cancellation,
    ) -> Result<CapabilityResponse, String> {
        self(request, cancellation)
    }
}

#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    handlers: BTreeMap<String, Arc<dyn CapabilityHandler>>,
}
impl CapabilityRegistry {
    pub fn register<H: CapabilityHandler + 'static>(&mut self, id: impl Into<String>, handler: H) {
        self.handlers.insert(id.into(), Arc::new(handler));
    }
    pub fn contains(&self, id: &str) -> bool {
        self.handlers.contains_key(id)
    }
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.handlers.keys().map(String::as_str)
    }
    pub fn dispatch(
        &self,
        request: &CapabilityRequest,
        cancellation: &Cancellation,
    ) -> Result<CapabilityResponse, Diagnostic> {
        if cancellation.is_cancelled() {
            return Ok(CapabilityResponse { effects: vec![] });
        }
        let Some(handler) = self.handlers.get(&request.id) else {
            return Err(Diagnostic::new(
                CODE_CAPABILITY_MISSING,
                Severity::Error,
                SOURCE_RUNTIME,
                "story capability is not registered",
            )
            .field("capability", request.id.clone()));
        };
        handler
            .invoke(request.clone(), cancellation.clone())
            .map_err(|message| {
                Diagnostic::new(
                    CODE_CAPABILITY_FAILED,
                    Severity::Error,
                    SOURCE_RUNTIME,
                    message,
                )
                .field("capability", request.id.clone())
            })
    }
}

pub fn request_from_effect(effect: &Effect) -> Option<CapabilityRequest> {
    match effect {
        Effect::Capability { id, args } => Some(CapabilityRequest {
            id: canonical_id(id),
            args: args.clone(),
        }),
        _ => None,
    }
}
pub fn canonical_id(id: &str) -> String {
    let mut result = String::new();
    for (index, ch) in id.chars().enumerate() {
        if ch.is_ascii_uppercase() && index != 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplayProof {
    pub schema: String,
    pub events: usize,
    pub digest: String,
}
pub fn replay_proof(trace: &[TraceEvent]) -> Result<ReplayProof, Diagnostic> {
    let bytes = serde_json::to_vec(trace).map_err(|error| {
        Diagnostic::new(
            CODE_REPLAY_INVALID,
            Severity::Error,
            SOURCE_RUNTIME,
            error.to_string(),
        )
    })?;
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut digest, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(ReplayProof {
        schema: CAPABILITY_SCHEMA.into(),
        events: trace.len(),
        digest,
    })
}
pub fn validate_replay(expected: &[TraceEvent], actual: &[TraceEvent]) -> Result<(), Diagnostic> {
    if expected == actual {
        return Ok(());
    }
    let first = expected.iter().zip(actual).position(|(a, b)| a != b);
    Err(Diagnostic::new(
        CODE_REPLAY_MISMATCH,
        Severity::Error,
        SOURCE_RUNTIME,
        "story replay diverged from the deterministic trace",
    )
    .field("expected_events", expected.len().to_string())
    .field("actual_events", actual.len().to_string())
    .field(
        "first_difference",
        first.map_or_else(|| "length".into(), |n| n.to_string()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use keygen_engine::story::{Effect, Tag, TraceEvent};
    #[test]
    fn dispatch_and_canonicalize() {
        let mut registry = CapabilityRegistry::default();
        registry.register("menu_input", |request: CapabilityRequest, _| {
            Ok(CapabilityResponse {
                effects: vec![Effect::Capability {
                    id: request.id.clone(),
                    args: request.args.clone(),
                }],
            })
        });
        let request = request_from_effect(&Effect::Capability {
            id: "MenuInput".into(),
            args: BTreeMap::new(),
        })
        .unwrap();
        assert_eq!(request.id, "menu_input");
        assert_eq!(
            registry
                .dispatch(&request, &Cancellation::default())
                .unwrap()
                .effects
                .len(),
            1
        );
    }
    #[test]
    fn missing_failure_and_cancellation_are_diagnostic() {
        let mut registry = CapabilityRegistry::default();
        let missing = CapabilityRequest {
            id: "missing".into(),
            args: BTreeMap::new(),
        };
        assert_eq!(
            registry
                .dispatch(&missing, &Cancellation::default())
                .unwrap_err()
                .code,
            CODE_CAPABILITY_MISSING
        );
        registry.register("broken", |_, _| Err("synthetic failure".into()));
        let broken = CapabilityRequest {
            id: "broken".into(),
            args: BTreeMap::new(),
        };
        assert_eq!(
            registry
                .dispatch(&broken, &Cancellation::default())
                .unwrap_err()
                .code,
            CODE_CAPABILITY_FAILED
        );
        let mut cancellation = Cancellation::default();
        cancellation.cancel();
        assert!(registry
            .dispatch(&broken, &cancellation)
            .unwrap()
            .effects
            .is_empty());
    }
    #[test]
    fn replay_proof_and_divergence_are_deterministic() {
        let event = TraceEvent {
            clock: 0,
            block: 0,
            ip: 0,
            tag: Tag::Nop,
            effects: vec![],
        };
        assert_eq!(
            replay_proof(std::slice::from_ref(&event)).unwrap().events,
            1
        );
        assert!(validate_replay(std::slice::from_ref(&event), &[]).is_err());
        assert!(
            validate_replay(std::slice::from_ref(&event), std::slice::from_ref(&event)).is_ok()
        );
    }
}
