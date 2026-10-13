//! Rewrite only selected JSON children; all other payload bytes stay opaque.

use bingo_sdk::KernelError;
use serde::Serialize;
use serde_json::value::{RawValue, to_raw_value};
use std::collections::BTreeMap;

pub(super) type Changed = Result<Option<Box<RawValue>>, KernelError>;
pub(super) type Rewrite<'a> = dyn FnMut(&RawValue) -> Changed + 'a;

pub(super) fn encoded(value: &impl Serialize) -> Result<Box<RawValue>, KernelError> {
    to_raw_value(value).map_err(|e| crate::storage(format!("encode record: {e}")))
}

pub(super) fn field(raw: &RawValue, key: &str, rewrite: &mut Rewrite<'_>) -> Changed {
    let Ok(mut object) = serde_json::from_str::<BTreeMap<String, Box<RawValue>>>(raw.get()) else {
        return Ok(None);
    };
    let Some(child) = object.get_mut(key) else {
        return Ok(None);
    };
    let Some(changed) = rewrite(child)? else {
        return Ok(None);
    };
    *child = changed;
    encoded(&object).map(Some)
}

pub(super) fn array(raw: &RawValue, rewrite: &mut Rewrite<'_>) -> Changed {
    let Ok(mut parts) = serde_json::from_str::<Vec<Box<RawValue>>>(raw.get()) else {
        return Ok(None);
    };
    let mut changed = false;
    for part in &mut parts {
        if let Some(replaced) = rewrite(part)? {
            *part = replaced;
            changed = true;
        }
    }
    if changed {
        encoded(&parts).map(Some)
    } else {
        Ok(None)
    }
}

pub(super) fn tag(raw: &RawValue, key: &str) -> Option<String> {
    let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw.get()).ok()?;
    serde_json::from_str(fields.get(key)?.get()).ok()
}
