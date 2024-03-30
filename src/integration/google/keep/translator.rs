use domain::entity_translator::EntityTranslator as TranslatorTrait;

use crate::domain;
use crate::domain::entity::Entity;
use crate::integration::google::keep::google_keep::GoogleKeep;

#[derive(Clone, Debug)]
pub(crate) struct Translator;

impl TranslatorTrait<GoogleKeep, String> for Translator {
    fn translate(&self, entity: &Entity<GoogleKeep>) -> Entity<String> {
        entity.with_data(&"translated data".to_string())
    }
}
