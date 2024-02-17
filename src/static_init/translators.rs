use once_cell::sync::Lazy;
use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;

use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use uuid::Uuid;

use crate::domain::entity_translator::{EntityTranslator, TranslationDescription};
use crate::integration::stub::translator::UuidToStringTranslator;

#[derive(Clone, EnumIter)]
pub(crate) enum Translators {
    NoOp,
    UuidToString(Option<UuidToStringTranslator>),
}

pub(crate) static SUPPORTED_TYPES: Lazy<HashMap<&str, TypeId>> = Lazy::new(|| {
    let mut types_by_name: HashMap<&str, TypeId> = HashMap::new();
    types_by_name.insert(type_name::<String>(), TypeId::of::<String>());
    types_by_name.insert(type_name::<Uuid>(), TypeId::of::<Uuid>());
    types_by_name
});

pub(crate) fn create_translators() -> HashMap<TranslationDescription, Translators> {
    let mut translators_by_description: HashMap<TranslationDescription, Translators> =
        HashMap::new();
    Translators::iter().for_each(|translator_type| {
        match translator_type {
            Translators::NoOp => translators_by_description.insert(
                TranslationDescription {
                    from: TypeId::of::<dyn Any>(),
                    to: TypeId::of::<dyn Any>(),
                },
                Translators::NoOp,
            ),
            Translators::UuidToString(_) => translators_by_description.insert(
                UuidToStringTranslator::translation_description(),
                Translators::UuidToString(Some(UuidToStringTranslator)),
            ),
        };
    });
    translators_by_description
}
