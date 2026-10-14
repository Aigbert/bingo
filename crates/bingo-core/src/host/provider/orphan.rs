//! An inactive declaration still owns its credential.
use super::{Host, KernelError, invalid, settings};
use serde_json::{Map, Value};

pub(super) fn remove(host: &Host, name: &str) -> Result<String, KernelError> {
    if host.config.reserved_providers.contains(name)
        || host.registry.endpoint_name_taken(name)
        || host.registry.provider_state.shapes.contains_key(name)
    {
        return Err(invalid("a registered provider credential is not an orphan"));
    }
    for shape in host.registry.provider_state.shapes.values() {
        if !shape
            .credential_exists(name)
            .map_err(|_| invalid("could not read credential store"))?
        {
            continue;
        }
        let owner = shape
            .credential_owner(name)
            .map_err(|_| invalid("could not read credential owner"))?;
        if let Some(file) = declared(host, name, owner.as_deref())? {
            return Err(invalid(&format!(
                "`{name}` still has a declaration in {file}; rename or delete it there before removing its credential."
            )));
        }
        shape
            .forget_credential(name, owner.as_deref())
            .map_err(|_| invalid("could not remove stored credential"))?;
        return Ok(format!("Removed orphaned credential {name}."));
    }
    Err(invalid("no instance or orphaned credential has that name"))
}

fn declared(host: &Host, name: &str, owner: Option<&str>) -> Result<Option<String>, KernelError> {
    let path = settings::user_path(host.env());
    let user =
        settings::read_document(&path).map_err(|_| invalid("could not read user settings"))?;
    if holds(&user, name, owner) {
        return Ok(Some(path.display().to_string()));
    }
    Ok(host
        .config
        .layers
        .iter()
        .filter(|layer| !layer.is_user())
        .find(|layer| holds(&layer.value, name, owner))
        .map(|layer| layer.source.clone()))
}

fn holds(values: &Map<String, Value>, name: &str, owner: Option<&str>) -> bool {
    values.iter().any(|(family, value)| {
        owner.is_none_or(|owner| owner == family)
            && value
                .get("instances")
                .and_then(|instances| instances.get(name))
                .is_some()
    })
}
