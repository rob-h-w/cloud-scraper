use crate::static_init::error::Error;
use serde_yaml::{Mapping, Value};
use std::fmt::Debug;

type Result<'a> = std::result::Result<&'a Value, Error>;
type MutResult = std::result::Result<Value, Error>;

pub trait MappingExtension: Clone + PartialEq {
    fn as_mapping(&self) -> Option<&Mapping>;

    fn contains_key(&self, key: &Value) -> bool {
        self.as_mapping()
            .map_or(false, |mapping| mapping.contains_key(key))
    }

    fn get_by_str(&self, key: &str) -> Result {
        self.get_value(&Value::String(key.to_string()))
    }

    fn get_value(&self, key: &Value) -> Result;
}

impl MappingExtension for Value {
    fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Value::Mapping(mapping) => Some(mapping),
            _ => None,
        }
    }

    fn get_value(&self, key: &Value) -> Result {
        match self {
            Value::Mapping(mapping) => mapping.get(key).ok_or(Error::KeyNotFound(key.clone())),
            _ => Err(Error::NotAMapping(self.clone())),
        }
    }
}

impl MappingExtension for Result<'_> {
    fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Ok(value) => value.as_mapping(),
            Err(_) => None,
        }
    }

    fn get_value(&self, key: &Value) -> Result {
        match self {
            Ok(self_value) => self_value.get_value(key),
            Err(_) => self.clone(),
        }
    }
}

impl MappingExtension for MutResult {
    fn as_mapping(&self) -> Option<&Mapping> {
        match self {
            Ok(value) => value.as_mapping(),
            Err(_) => None,
        }
    }

    fn get_value(&self, key: &Value) -> Result {
        match self {
            Ok(self_value) => self_value.get_value(key),
            Err(e) => Err(e.clone()),
        }
    }
}

impl MappingExtension for Mapping {
    fn as_mapping(&self) -> Option<&Mapping> {
        Some(self)
    }

    fn get_value(&self, key: &Value) -> Result {
        match self.get(key) {
            Some(value) => Ok(value),
            None => Err(Error::KeyNotFound(key.clone())),
        }
    }
}

pub trait ConvertableToMovableValueResult {
    fn to_movable_value_result(&self) -> MutResult;
}

impl ConvertableToMovableValueResult for Value {
    fn to_movable_value_result(&self) -> MutResult {
        Ok(self.clone())
    }
}

impl ConvertableToMovableValueResult for MutResult {
    fn to_movable_value_result(&self) -> MutResult {
        self.clone()
    }
}

pub trait KeyIdValue: Sized {
    fn to_key_id_value(&self) -> Value;
}

impl KeyIdValue for String {
    fn to_key_id_value(&self) -> Value {
        Value::String(self.to_string())
    }
}

impl KeyIdValue for Value {
    fn to_key_id_value(&self) -> Value {
        self.clone()
    }
}

pub trait FluentMutable: Debug + MappingExtension {
    fn with_default_value_at(&self, key: &Value, value: &Value) -> MutResult {
        if self.contains_key(key) {
            Ok(Value::Mapping(
                self.as_mapping()
                    .unwrap_or_else(|| {
                        panic!(
                            "{:?} was not a mapping despite containing key {:?}",
                            self, key
                        )
                    })
                    .clone(),
            ))
        } else {
            self.with_value_at(key, value)
        }
    }
    fn with_value_at(&self, key: &Value, value: &Value) -> MutResult;
}

impl FluentMutable for Result<'_> {
    fn with_value_at(&self, key: &Value, value: &Value) -> MutResult {
        match self {
            Ok(self_value) => (*self_value).with_value_at(key, value),
            Err(e) => Err(e.clone()),
        }
    }
}

impl FluentMutable for MutResult {
    fn with_value_at(&self, key: &Value, value: &Value) -> MutResult {
        match self {
            Ok(self_value) => self_value.with_value_at(key, value),
            Err(e) => Err(e.clone()),
        }
    }
}

impl FluentMutable for Value {
    fn with_value_at(&self, key: &Value, value: &Value) -> MutResult {
        match self {
            Value::Mapping(mapping) => mapping.with_value_at(key, value),
            _ => Err(Error::NotAMapping(self.clone())),
        }
    }
}

impl FluentMutable for Mapping {
    fn with_default_value_at(&self, key: &Value, value: &Value) -> MutResult {
        match self.get(key) {
            Some(_) => Ok(Value::Mapping(self.clone())),
            None => self.with_value_at(key, value),
        }
    }

    fn with_value_at(&self, key: &Value, value: &Value) -> MutResult {
        let mut mapping = self.clone();
        mapping.insert(key.clone(), value.clone());
        Ok(Value::Mapping(mapping))
    }
}

pub trait FluentMutatorMappingExtension: FluentMutable + PartialEq {
    fn with_default_at<KeyIdType, ValueType>(&self, key: &KeyIdType, value: &ValueType) -> MutResult
    where
        KeyIdType: KeyIdValue,
        ValueType: ConvertableToMovableValueResult,
    {
        match value.to_movable_value_result() {
            Ok(value) => self.with_default_value_at(&key.to_key_id_value(), &value),
            Err(e) => Err(e.clone()),
        }
    }

    fn with_at<KeyIdType, ValueType>(&self, key: &KeyIdType, value: &ValueType) -> MutResult
    where
        KeyIdType: KeyIdValue,
        ValueType: ConvertableToMovableValueResult,
    {
        match value.to_movable_value_result() {
            Ok(value) => self.with_value_at(&key.to_key_id_value(), &value),
            Err(e) => Err(e.clone()),
        }
    }
}

impl FluentMutatorMappingExtension for Result<'_> {}

impl FluentMutatorMappingExtension for MutResult {}

impl FluentMutatorMappingExtension for Value {}

impl FluentMutatorMappingExtension for Mapping {}

#[cfg(test)]
mod tests {
    use super::*;

    mod mapping_extension {
        use super::*;

        mod value {
            use super::*;

            mod as_mapping {
                use super::*;

                #[test]
                fn on_mapping_returns_mapping() {
                    let value = Value::Mapping(Mapping::new());
                    assert_eq!(value.as_mapping(), Some(&Mapping::new()));
                }

                #[test]
                fn on_non_mapping_returns_none() {
                    let value = Value::Null;
                    assert_eq!(value.as_mapping(), None);

                    let value = Value::Sequence(vec![]);
                    assert_eq!(value.as_mapping(), None);
                }
            }

            mod get_value {
                use super::*;

                #[test]
                fn on_non_mapping_returns_none() {
                    let value = Value::Null;
                    assert_eq!(
                        value.get_value(&Value::String("key".to_string())),
                        Err(Error::NotAMapping(value.clone()))
                    );

                    let value = Value::Sequence(vec![]);
                    assert_eq!(
                        value.get_value(&Value::String("key".to_string())),
                        Err(Error::NotAMapping(value.clone()))
                    );
                }

                #[test]
                fn on_missing_key_returns_none() {
                    let value = Value::Mapping(Mapping::new());
                    let key = Value::String("key".to_string());
                    assert_eq!(value.get_value(&key), Err(Error::KeyNotFound(key.clone())));
                }

                #[test]
                fn retrieves_key_if_present() {
                    let mut mapping = Mapping::new();
                    let key = Value::Bool(true);
                    mapping.insert(key.clone(), Value::String("value".to_string()));
                    let value = Value::Mapping(mapping);
                    assert_eq!(
                        value.get_value(&key),
                        Ok(&Value::String("value".to_string()))
                    );
                }
            }

            mod get_by_str {
                use super::*;

                #[test]
                fn works_as_get() {
                    let mut mapping = Mapping::new();
                    mapping.insert(
                        Value::String("key".to_string()),
                        Value::String("value".to_string()),
                    );
                    let value = Value::Mapping(mapping);
                    assert_eq!(
                        value.get_by_str("key"),
                        Ok(&Value::String("value".to_string()))
                    );
                }
            }
        }
    }

    mod convertable_to_movable_value_result {
        use super::*;

        #[test]
        fn value_to_movable_value_result() {
            let value = Value::String("value".to_string());
            assert_eq!(value.to_movable_value_result(), Ok(value.clone()));
        }
    }

    mod fluent_mutator_mapping_extension {
        use super::*;

        #[test]
        fn general_fluent_exercise() {
            let value: Value = Value::Mapping(Mapping::new());

            assert_eq!(value.get("key"), None);

            let value = value
                .with_default_at::<String, Value>(
                    &"key".to_string(),
                    &Value::String("value".to_string()),
                )
                .with_default_at(&Value::Bool(true), &Value::Bool(false));

            assert_eq!(
                value.get_by_str("key"),
                Ok(&Value::String("value".to_string()))
            );
            assert_eq!(value.get_value(&Value::Bool(true)), Ok(&Value::Bool(false)));

            let value_parent =
                Value::Mapping(Mapping::new()).with_default_at(&"value".to_string(), &value);

            assert_eq!(value_parent.get_by_str("value"), Ok(&value.unwrap()));
        }

        mod with_default_at {
            use super::*;

            #[test]
            fn does_not_modify_pre_existing_properties() {
                let value: Value = Value::Mapping(Mapping::new());
                let value = value
                    .with_default_at::<String, Value>(
                        &"key".to_string(),
                        &Value::String("value".to_string()),
                    )
                    .with_default_at(&Value::Bool(true), &Value::Bool(false));

                assert_eq!(
                    value.get_by_str("key"),
                    Ok(&Value::String("value".to_string()))
                );
                assert_eq!(value.get_value(&Value::Bool(true)), Ok(&Value::Bool(false)));

                let value = value.with_default_at(&"key".to_string(), &value);

                assert_eq!(
                    value.get_by_str("key"),
                    Ok(&Value::String("value".to_string()))
                );
            }
        }

        mod with_at {
            use super::*;

            #[test]
            fn replaces_pre_existing_properties() {
                let value: Value = Value::Mapping(Mapping::new());
                let value = value
                    .with_default_at::<String, Value>(
                        &"key".to_string(),
                        &Value::String("value".to_string()),
                    )
                    .with_default_at(&Value::Bool(true), &Value::Bool(false));

                assert_eq!(
                    value.get_by_str("key"),
                    Ok(&Value::String("value".to_string()))
                );
                assert_eq!(value.get_value(&Value::Bool(true)), Ok(&Value::Bool(false)));

                let new_value = value.with_at(&"key".to_string(), &value);

                assert_eq!(
                    new_value
                        .get_by_str("key")
                        .expect("Could not get value for key"),
                    &value.expect("Could not get value")
                );
            }
        }
    }
}
