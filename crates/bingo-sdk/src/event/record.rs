//! Known records use the one derived vocabulary; unknown payloads retain bytes.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value, value::RawValue};

use super::{Event, Item, ItemBody, raw};

/// Opaque JSON retained by deserialisation. No producer can create its bytes.
///
/// ```compile_fail
/// let raw = bingo_sdk::UnknownRecord { json: "{}".into() };
/// ```
#[derive(Clone, Debug)]
pub struct UnknownRecord {
    json: Box<RawValue>,
}

impl PartialEq for UnknownRecord {
    fn eq(&self, other: &Self) -> bool {
        self.json.get() == other.json.get()
    }
}

impl Serialize for UnknownRecord {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.json.serialize(s)
    }
}

impl JsonSchema for UnknownRecord {
    fn schema_name() -> Cow<'static, str> {
        "UnknownRecord".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        Map::<String, Value>::json_schema(generator)
    }
}

impl<'de> Deserialize<'de> for Event {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let json = raw::capture(d)?;
        Ok(read_event(&json).unwrap_or_else(|_| Self::Unknown {
            raw: UnknownRecord { json },
        }))
    }
}

fn read_event(json: &RawValue) -> Result<Event, serde_json::Error> {
    #[derive(Deserialize)]
    struct Known(#[serde(with = "Event")] Event);
    let mut event = serde_json::from_str::<Known>(json.get())?.0;
    restore_item_bytes(&mut event, json)?;
    Ok(event)
}

fn restore_item_bytes(event: &mut Event, json: &RawValue) -> Result<(), serde_json::Error> {
    let item = match event {
        Event::ItemStarted { item }
        | Event::ItemUpdated { item }
        | Event::ItemCompleted { item }
            if matches!(item.body, ItemBody::Unknown { .. }) =>
        {
            item
        }
        _ => return Ok(()),
    };
    // The tagged enum's derived reader buffered the item. Read its struct
    // directly so the unknown body retains the original whitespace and escapes.
    #[derive(Deserialize)]
    struct Envelope {
        item: Item,
    }
    *item = serde_json::from_str::<Envelope>(json.get())?.item;
    Ok(())
}

impl Serialize for Event {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unknown { raw } => raw.serialize(s),
            _ => {
                #[derive(Serialize)]
                struct Known<'a>(#[serde(with = "Event")] &'a Event);
                Known(self).serialize(s)
            }
        }
    }
}

impl<'de> Deserialize<'de> for ItemBody {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Known(#[serde(with = "ItemBody")] ItemBody);
        let json = raw::capture(d)?;
        Ok(match serde_json::from_str::<Known>(json.get()) {
            Ok(known) => known.0,
            Err(_) => Self::Unknown {
                raw: UnknownRecord { json },
            },
        })
    }
}

impl Serialize for ItemBody {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unknown { raw } => raw.serialize(s),
            _ => {
                #[derive(Serialize)]
                struct Known<'a>(#[serde(with = "ItemBody")] &'a ItemBody);
                Known(self).serialize(s)
            }
        }
    }
}
