//! Persisted pictures are references; live frames and checkpoints keep Image.

mod parts;
mod raw;

use base64::{Engine, engine::general_purpose::STANDARD};
use bingo_sdk::{Checkpoint, ContentPart, Frame, Image, KernelError};
use raw::{Changed, encoded};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::path::PathBuf;

type Put<'a> = dyn FnMut(&[u8]) -> Result<String, KernelError> + 'a;
type Get<'a> = dyn FnMut(&str) -> Option<Vec<u8>> + 'a;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Picture {
    #[serde(rename = "type")]
    kind: String,
    media_type: String,
    blob: String,
    bytes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
}

pub(crate) fn encode(frame: &Frame, put: &mut Put<'_>) -> Result<String, KernelError> {
    let raw = encoded(frame)?;
    Ok(parts::frame(&raw, true, &mut |raw| keep(raw, put))?
        .unwrap_or(raw)
        .get()
        .to_owned())
}

pub(crate) fn decode(line: &[u8], get: &mut Get<'_>) -> Result<Frame, serde_json::Error> {
    let raw: Box<RawValue> = serde_json::from_slice(line)?;
    let changed = parts::frame(&raw, false, &mut |raw| resolve(raw, get))
        .map_err(<serde_json::Error as serde::de::Error>::custom)?;
    serde_json::from_str(changed.as_deref().unwrap_or(&raw).get())
}

pub(crate) fn encode_checkpoint(
    checkpoint: &Checkpoint,
    put: &mut Put<'_>,
) -> Result<Vec<u8>, KernelError> {
    let raw = encoded(checkpoint)?;
    let changed = parts::checkpoint(&raw, true, &mut |raw| keep(raw, put))?;
    Ok(changed.as_deref().unwrap_or(&raw).get().as_bytes().to_vec())
}

pub(crate) fn decode_checkpoint(
    bytes: &[u8],
    get: &mut Get<'_>,
) -> Result<Checkpoint, KernelError> {
    let raw: Box<RawValue> = serde_json::from_slice(bytes).map_err(decode_error)?;
    let changed = parts::checkpoint(&raw, false, &mut |raw| resolve(raw, get))?;
    serde_json::from_str(changed.as_deref().unwrap_or(&raw).get()).map_err(decode_error)
}

fn decode_error(error: serde_json::Error) -> KernelError {
    crate::storage(format!("decode checkpoint: {error}"))
}

fn keep(raw: &RawValue, put: &mut Put<'_>) -> Changed {
    let Ok(image) = serde_json::from_str::<Image>(raw.get()) else {
        return Ok(None);
    };
    let bytes = STANDARD
        .decode(&*image.data)
        .map_err(|e| crate::storage(format!("decode picture base64: {e}")))?;
    let blob = put(&bytes)?;
    encoded(&Picture {
        kind: "image".into(),
        media_type: image.media_type,
        blob,
        bytes: bytes.len(),
        width: image.width,
        height: image.height,
        path: image.path,
    })
    .map(Some)
}

fn resolve(raw: &RawValue, get: &mut Get<'_>) -> Changed {
    let Ok(picture) = serde_json::from_str::<Picture>(raw.get()) else {
        return Ok(None);
    };
    let part = match get(&picture.blob) {
        Some(bytes) => ContentPart::Image(Image {
            media_type: picture.media_type,
            data: STANDARD.encode(bytes).into(),
            width: picture.width,
            height: picture.height,
            path: picture.path,
        }),
        None => ContentPart::text(format!(
            "[picture missing: {} {}]",
            picture.media_type,
            bingo_sdk::bytes::words(picture.bytes)
        )),
    };
    encoded(&part).map(Some)
}

#[cfg(test)]
mod tests;
