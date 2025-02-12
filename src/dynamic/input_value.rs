use crate::{dynamic::TypeRef, registry::MetaInputValue, Value};

/// A GraphQL input value type
#[derive(Debug)]
pub struct InputValue {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) ty: TypeRef,
    pub(crate) default_value: Option<Value>,
}

impl InputValue {
    /// Create a GraphQL input value type
    #[inline]
    pub fn new(name: impl Into<String>, ty: impl Into<TypeRef>) -> Self {
        Self {
            name: name.into(),
            description: None,
            ty: ty.into(),
            default_value: None,
        }
    }

    /// Set the description
    #[inline]
    pub fn description(self, description: impl Into<String>) -> Self {
        Self {
            description: Some(description.into()),
            ..self
        }
    }

    /// Set the default value
    #[inline]
    pub fn default_value(self, value: impl Into<Value>) -> Self {
        Self {
            default_value: Some(value.into()),
            ..self
        }
    }

    pub(crate) fn to_meta_input_value(&self) -> MetaInputValue {
        MetaInputValue {
            name: self.name.clone(),
            description: self.description.clone(),
            ty: self.ty.to_string(),
            default_value: self
                .default_value
                .as_ref()
                .map(std::string::ToString::to_string),
            visible: None,
            inaccessible: false,
            tags: vec![],
            is_secret: false,
        }
    }
}
