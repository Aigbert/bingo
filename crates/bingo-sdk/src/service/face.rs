//! A service declares the events its wire methods raise.
use crate::catalogue::{Entry, Shape};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

#[derive(Clone, Debug)]
pub struct Face {
    pub noun: String,
    pub methods: Vec<Method>,
}
#[derive(Clone, Debug)]
pub struct Method {
    pub entry: Entry,
    pub unsafe_call: bool,
}
impl Face {
    pub fn new(noun: impl Into<String>) -> Self {
        Self { noun: noun.into(), methods: Vec::new() }
    }
    pub fn method<I: JsonSchema + DeserializeOwned, O: JsonSchema + DeserializeOwned>(
        mut self, method: &str, unsafe_call: bool,
    ) -> Self {
        let mut entry = Entry::face(format!("{}.{method}", self.noun),
            schemars::schema_for!(I), schemars::schema_for!(O));
        entry.input = Shape::Rust(|| schemars::schema_for!(I));
        entry.result = Shape::Rust(|| schemars::schema_for!(O));
        entry.valid_input = |v| serde_json::from_value::<I>(v.clone()).is_ok();
        entry.valid_result = |v| serde_json::from_value::<O>(v.clone()).is_ok();
        self.methods.push(Method { entry, unsafe_call });
        self
    }
    pub fn find(&self, method: &str) -> Option<&Method> {
        let name = format!("{}.{method}", self.noun);
        self.methods.iter().find(|method| method.entry.name == name)
    }
}
