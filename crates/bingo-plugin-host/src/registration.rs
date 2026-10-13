//! The descriptor and admission of a reloadable external plugin.
use crate::{admission::Admission, discovery::Found, grant::grant, manifest::Manifest};
use bingo_sdk::*;
use std::sync::RwLock;

struct Descriptor {
    version: String,
    uses: Vec<String>,
}
pub struct Registration {
    pub name: String,
    origin: PluginOrigin,
    runtime: PluginRuntime,
    descriptor: RwLock<Descriptor>,
    pub admission: Admission,
}
impl Registration {
    pub fn new(found: &Found, runtime: PluginRuntime) -> Self {
        Self {
            name: found.name.clone(),
            origin: found.origin,
            runtime,
            descriptor: RwLock::new(Descriptor {
                version: found.manifest.version.clone(),
                uses: found.manifest.uses.clone(),
            }),
            admission: Admission::default(),
        }
    }
    pub fn update(&self, manifest: &Manifest) {
        *self.descriptor.write().unwrap_or_else(|e| e.into_inner()) = Descriptor {
            version: manifest.version.clone(),
            uses: manifest.uses.clone(),
        };
        self.admission.reset();
    }
    pub fn claim(&self, trust: Option<&PluginTrust>) -> Option<PluginRegistration> {
        let descriptor = self.descriptor.read().unwrap_or_else(|e| e.into_inner());
        self.admission.claim_registration(PluginRegistration {
            name: self.name.clone(),
            version: descriptor.version.clone(),
            execution: PluginExecution {
                origin: self.origin,
                runtime: self.runtime,
                grant: grant(self.runtime, self.origin, trust, &descriptor.uses),
            },
        })
    }
    pub fn status(&self, from: &str) -> PluginStatus {
        let (grant, reason) = match self.admission.decision() {
            Ok(Some(grant)) => (Some(grant), None),
            Ok(None) => (None, Some("registration pending".into())),
            Err(reason) => (None, Some(reason)),
        };
        PluginStatus {
            id: self.name.clone(),
            version: self
                .descriptor
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .version
                .clone(),
            enabled: grant.is_some(),
            reason,
            from: from.into(),
            execution: grant.map(|grant| PluginExecution {
                origin: self.origin,
                runtime: self.runtime,
                grant,
            }),
        }
    }
}
