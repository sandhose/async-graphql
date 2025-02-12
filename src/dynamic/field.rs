use std::{
    any::Any,
    borrow::Cow,
    fmt::{self, Debug},
    ops::Deref,
};

use futures_util::{future::BoxFuture, Future, FutureExt};
use indexmap::IndexMap;

use crate::{
    dynamic::{InputValue, ObjectAccessor, TypeRef},
    registry::Deprecation,
    Context, Error, Result, Value,
};

/// A value returned from the resolver function
pub enum FieldValue<'a> {
    /// Const value
    Value(Value),
    /// Borrowed any value
    BorrowedAny(&'a (dyn Any + Send + Sync)),
    /// Owned any value
    OwnedAny(Box<dyn Any + Send + Sync>),
    /// A list
    List(Vec<FieldValue<'a>>),
    /// A typed Field value
    WithType {
        /// Field value
        value: Box<FieldValue<'a>>,
        /// Object name
        ty: Cow<'static, str>,
    },
}

impl<'a> From<()> for FieldValue<'a> {
    #[inline]
    fn from(_: ()) -> Self {
        FieldValue::Value(Value::Null)
    }
}

impl<'a> From<Value> for FieldValue<'a> {
    #[inline]
    fn from(value: Value) -> Self {
        FieldValue::Value(value)
    }
}

impl<'a, T: Into<FieldValue<'a>>> From<Vec<T>> for FieldValue<'a> {
    fn from(values: Vec<T>) -> Self {
        FieldValue::List(values.into_iter().map(Into::into).collect())
    }
}

impl<'a> FieldValue<'a> {
    /// A null value equivalent to `FieldValue::Value(Value::Null)`
    pub const NULL: FieldValue<'a> = FieldValue::Value(Value::Null);

    /// A none value equivalent to `None::<FieldValue>`
    ///
    /// It is more convenient to use when your resolver needs to return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_graphql::dynamic::*;
    ///
    /// let query = Object::new("Query").field(Field::new("value", TypeRef::INT, |ctx| {
    ///     FieldFuture::new(async move { Ok(FieldValue::NONE) })
    /// }));
    /// ```
    pub const NONE: Option<FieldValue<'a>> = None;

    /// Returns a `None::<FieldValue>` meaning the resolver no results.
    pub const fn none() -> Option<FieldValue<'a>> {
        None
    }

    /// Create a FieldValue from [`Value`]
    #[inline]
    pub fn value(value: impl Into<Value>) -> Self {
        FieldValue::Value(value.into())
    }

    /// Create a FieldValue from owned any value
    #[inline]
    pub fn owned_any(obj: impl Any + Send + Sync) -> Self {
        FieldValue::OwnedAny(Box::new(obj))
    }

    /// Create a FieldValue from owned any value
    #[inline]
    pub fn borrowed_any(obj: &'a (impl Any + Send + Sync)) -> Self {
        FieldValue::BorrowedAny(obj)
    }

    /// Create a FieldValue from list
    #[inline]
    pub fn list<I, T>(values: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<FieldValue<'a>>,
    {
        FieldValue::List(values.into_iter().map(Into::into).collect())
    }

    /// Create a FieldValue and specify its type, which must be an object
    ///
    /// NOTE: Fields of type `Interface` or `Union` must return
    /// `FieldValue::WithType`.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_graphql::{dynamic::*, value, Value};
    ///
    /// struct MyObjData {
    ///     a: i32,
    /// }
    ///
    /// let my_obj = Object::new("MyObj").field(Field::new(
    ///     "a",
    ///     TypeRef::INT,
    ///     |ctx| FieldFuture::new(async move {
    ///         let data = ctx.parent_value.try_downcast_ref::<MyObjData>()?;
    ///         Ok(Some(Value::from(data.a)))
    ///     }),
    /// ));
    ///
    /// let my_union = Union::new("MyUnion").possible_type(my_obj.type_name());
    ///
    /// let query = Object::new("Query").field(Field::new(
    ///     "obj",
    ///     my_union.type_ref(),
    ///     |_| FieldFuture::new(async move {
    ///         Ok(Some(FieldValue::with_type(
    ///             FieldValue::owned_any(MyObjData { a: 10 }),
    ///             "MyObj",
    ///         )))
    ///     }),
    /// ));
    ///
    /// let schema = Schema::build("Query", None, None)
    ///     .register(my_obj)
    ///     .register(my_union)
    ///     .register(query)
    ///     .finish()
    ///     .unwrap();
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async move {
    /// assert_eq!(
    ///    schema
    ///        .execute("{ obj { ... on MyObj { a } } }")
    ///        .await
    ///        .into_result()
    ///        .unwrap()
    ///        .data,
    ///    value!({ "obj": { "a": 10 } })
    /// );
    /// # });
    /// ```
    pub fn with_type(value: impl Into<FieldValue<'a>>, ty: impl Into<Cow<'static, str>>) -> Self {
        FieldValue::WithType {
            value: Box::new(value.into()),
            ty: ty.into(),
        }
    }

    /// If the FieldValue is a [`FieldValue::Value`], returns the associated
    /// Value. Returns `None` otherwise.
    #[inline]
    pub fn as_value(&self) -> Option<&Value> {
        match &self {
            FieldValue::Value(value) => Some(value),
            _ => None,
        }
    }

    /// Like `as_value`, but returns `Result`.
    #[inline]
    pub fn try_to_value(&self) -> Result<&Value> {
        self.as_value()
            .ok_or_else(|| Error::new("internal: not a Value"))
    }

    /// If the FieldValue is a [`FieldValue::List`], returns the associated
    /// vector. Returns `None` otherwise.
    #[inline]
    pub fn as_list(&self) -> Option<&[FieldValue]> {
        match &self {
            FieldValue::List(values) => Some(values),
            _ => None,
        }
    }

    /// Like `as_list`, but returns `Result`.
    #[inline]
    pub fn try_to_list(&self) -> Result<&[FieldValue]> {
        self.as_list()
            .ok_or_else(|| Error::new("internal: not a list"))
    }

    /// If the FieldValue is a [`FieldValue::Any`], returns the associated
    /// vector. Returns `None` otherwise.
    #[inline]
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        match &self {
            FieldValue::BorrowedAny(value) => value.downcast_ref::<T>(),
            FieldValue::OwnedAny(value) => value.downcast_ref::<T>(),
            _ => None,
        }
    }

    /// Like `downcast_ref`, but returns `Result`.
    #[inline]
    pub fn try_downcast_ref<T: Any>(&self) -> Result<&T> {
        self.downcast_ref().ok_or_else(|| {
            Error::new(format!(
                "internal: not type \"{}\"",
                std::any::type_name::<T>()
            ))
        })
    }
}

type BoxResolveFut<'a> = BoxFuture<'a, Result<Option<FieldValue<'a>>>>;

/// A context for resolver function
pub struct ResolverContext<'a> {
    /// GraphQL context
    pub ctx: &'a Context<'a>,
    /// Field arguments
    pub args: ObjectAccessor<'a>,
    /// Parent value
    pub parent_value: &'a FieldValue<'a>,
}

impl<'a> Deref for ResolverContext<'a> {
    type Target = Context<'a>;

    fn deref(&self) -> &Self::Target {
        self.ctx
    }
}

/// A future that returned from field resolver
pub struct FieldFuture<'a>(pub(crate) BoxResolveFut<'a>);

impl<'a> FieldFuture<'a> {
    /// Create a ResolverFuture
    pub fn new<Fut, R>(future: Fut) -> Self
    where
        Fut: Future<Output = Result<Option<R>>> + Send + 'a,
        R: Into<FieldValue<'a>> + Send,
    {
        Self(
            async move {
                let res = future.await?;
                Ok(res.map(Into::into))
            }
            .boxed(),
        )
    }
}

type BoxResolverFn = Box<(dyn for<'a> Fn(ResolverContext<'a>) -> FieldFuture<'a> + Send + Sync)>;

/// A GraphQL field
pub struct Field {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) arguments: IndexMap<String, InputValue>,
    pub(crate) ty: TypeRef,
    pub(crate) resolver_fn: BoxResolverFn,
    pub(crate) deprecation: Deprecation,
}

impl Debug for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Field")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("ty", &self.ty)
            .field("deprecation", &self.deprecation)
            .finish()
    }
}

impl Field {
    /// Create a GraphQL field
    pub fn new<N, T, F>(name: N, ty: T, resolver_fn: F) -> Self
    where
        N: Into<String>,
        T: Into<TypeRef>,
        F: for<'a> Fn(ResolverContext<'a>) -> FieldFuture<'a> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            description: None,
            arguments: Default::default(),
            ty: ty.into(),
            resolver_fn: Box::new(resolver_fn),
            deprecation: Deprecation::NoDeprecated,
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

    /// Add an argument to the field
    #[inline]
    pub fn argument(mut self, input_value: InputValue) -> Self {
        self.arguments.insert(input_value.name.clone(), input_value);
        self
    }
}
