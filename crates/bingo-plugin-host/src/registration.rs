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
    pub origin: PluginOrigin,
    pub root: std::path::PathBuf,
    runtime: PluginRuntime,
    descriptor: RwLock<Descriptor>,
    pub admission: Admission,
}
impl Registration {
    pub fn new(found: &Found, runtime: PluginRuntime) -> Self {
        Self {
            name: found.name.clone(),
            root: found.root.clone(),
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
        let grant = self.admission.decision().ok().flatten();
        PluginStatus {
            id: if self.origin == PluginOrigin::Project
                && matches!(
                    self.admission.state(),
                    PluginState::Waiting { .. } | PluginState::Failed { .. }
                ) {
                format!("plugins:{}", self.name)
            } else {
                self.name.clone()
            },
            version: self
                .descriptor
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .version
                .clone(),
            state: self.admission.state(),
            from: from.into(),
            placement: Some(PluginPlacement {
                origin: self.origin,
                runtime: self.runtime,
            }),
            grant,
        }
    }
}
