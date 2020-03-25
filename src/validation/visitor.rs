use crate::error::RuleError;
use crate::registry;
use graphql_parser::query::{
    Definition, Directive, Document, Field, FragmentDefinition, FragmentSpread, InlineFragment,
    Name, OperationDefinition, Selection, SelectionSet, TypeCondition, Value, VariableDefinition,
};
use graphql_parser::Pos;
use std::collections::HashMap;

pub struct VisitorContext<'a> {
    pub registry: &'a registry::Registry,
    pub errors: Vec<RuleError>,
    type_stack: Vec<&'a registry::Type>,
    fragments: HashMap<&'a str, &'a FragmentDefinition>,
}

impl<'a> VisitorContext<'a> {
    pub fn new(registry: &'a registry::Registry, doc: &'a Document) -> Self {
        Self {
            registry,
            errors: Default::default(),
            type_stack: Default::default(),
            fragments: doc
                .definitions
                .iter()
                .filter_map(|d| match d {
                    Definition::Fragment(fragment) => Some((fragment.name.as_str(), fragment)),
                    _ => None,
                })
                .collect(),
        }
    }

    pub fn report_error<T: Into<String>>(&mut self, locations: Vec<Pos>, msg: T) {
        self.errors.push(RuleError {
            locations,
            message: msg.into(),
        })
    }

    pub fn append_errors(&mut self, errors: Vec<RuleError>) {
        self.errors.extend(errors);
    }

    pub fn with_type<F: FnMut(&mut VisitorContext<'a>)>(
        &mut self,
        ty: &'a registry::Type,
        mut f: F,
    ) {
        self.type_stack.push(ty);
        f(self);
        self.type_stack.pop();
    }

    pub fn parent_type(&self) -> Option<&'a registry::Type> {
        if self.type_stack.len() >= 2 {
            self.type_stack.get(self.type_stack.len() - 2).copied()
        } else {
            None
        }
    }

    pub fn current_type(&self) -> &'a registry::Type {
        self.type_stack.last().unwrap()
    }

    pub fn is_known_fragment(&self, name: &str) -> bool {
        self.fragments.contains_key(name)
    }

    pub fn fragment(&self, name: &str) -> Option<&'a FragmentDefinition> {
        self.fragments.get(name).copied()
    }
}

pub trait Visitor<'a> {
    fn enter_document(&mut self, _ctx: &mut VisitorContext<'a>, _doc: &'a Document) {}
    fn exit_document(&mut self, _ctx: &mut VisitorContext<'a>, _doc: &'a Document) {}

    fn enter_operation_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _operation_definition: &'a OperationDefinition,
    ) {
    }
    fn exit_operation_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _operation_definition: &'a OperationDefinition,
    ) {
    }

    fn enter_fragment_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _fragment_definition: &'a FragmentDefinition,
    ) {
    }
    fn exit_fragment_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _fragment_definition: &'a FragmentDefinition,
    ) {
    }

    fn enter_variable_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _variable_definition: &'a VariableDefinition,
    ) {
    }
    fn exit_variable_definition(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _variable_definition: &'a VariableDefinition,
    ) {
    }

    fn enter_directive(&mut self, _ctx: &mut VisitorContext<'a>, _directive: &'a Directive) {}
    fn exit_directive(&mut self, _ctx: &mut VisitorContext<'a>, _directive: &'a Directive) {}

    fn enter_argument(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _pos: Pos,
        _name: &'a str,
        _value: &'a Value,
    ) {
    }
    fn exit_argument(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _pos: Pos,
        _name: &'a str,
        _value: &'a Value,
    ) {
    }

    fn enter_selection_set(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _selection_set: &'a SelectionSet,
    ) {
    }
    fn exit_selection_set(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _selection_set: &'a SelectionSet,
    ) {
    }

    fn enter_selection(&mut self, _ctx: &mut VisitorContext<'a>, _selection: &'a Selection) {}
    fn exit_selection(&mut self, _ctx: &mut VisitorContext<'a>, _selection: &'a Selection) {}

    fn enter_field(&mut self, _ctx: &mut VisitorContext<'a>, _field: &'a Field) {}
    fn exit_field(&mut self, _ctx: &mut VisitorContext<'a>, _field: &'a Field) {}

    fn enter_fragment_spread(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _fragment_spread: &'a FragmentSpread,
    ) {
    }
    fn exit_fragment_spread(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _fragment_spread: &'a FragmentSpread,
    ) {
    }

    fn enter_inline_fragment(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _inline_fragment: &'a InlineFragment,
    ) {
    }
    fn exit_inline_fragment(
        &mut self,
        _ctx: &mut VisitorContext<'a>,
        _inline_fragment: &'a InlineFragment,
    ) {
    }
}

pub struct VisitorNil;

impl VisitorNil {
    pub fn with<V>(self, visitor: V) -> VisitorCons<V, Self> {
        VisitorCons(visitor, self)
    }
}

pub struct VisitorCons<A, B>(A, B);

impl<A, B> VisitorCons<A, B> {
    pub fn with<V>(self, visitor: V) -> VisitorCons<V, Self> {
        VisitorCons(visitor, self)
    }
}

impl<'a> Visitor<'a> for VisitorNil {}

impl<'a, A, B> Visitor<'a> for VisitorCons<A, B>
where
    A: Visitor<'a> + 'a,
    B: Visitor<'a> + 'a,
{
    fn enter_document(&mut self, ctx: &mut VisitorContext<'a>, doc: &'a Document) {
        self.0.enter_document(ctx, doc);
        self.1.enter_document(ctx, doc);
    }

    fn exit_document(&mut self, ctx: &mut VisitorContext<'a>, doc: &'a Document) {
        self.0.exit_document(ctx, doc);
        self.1.exit_document(ctx, doc);
    }

    fn enter_operation_definition(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        operation_definition: &'a OperationDefinition,
    ) {
        self.0.enter_operation_definition(ctx, operation_definition);
        self.1.enter_operation_definition(ctx, operation_definition);
    }

    fn exit_operation_definition(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        operation_definition: &'a OperationDefinition,
    ) {
        self.0.exit_operation_definition(ctx, operation_definition);
        self.1.exit_operation_definition(ctx, operation_definition);
    }

    fn enter_fragment_definition(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        fragment_definition: &'a FragmentDefinition,
    ) {
        self.0.enter_fragment_definition(ctx, fragment_definition);
        self.1.enter_fragment_definition(ctx, fragment_definition);
    }

    fn exit_fragment_definition(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        fragment_definition: &'a FragmentDefinition,
    ) {
        self.0.exit_fragment_definition(ctx, fragment_definition);
        self.1.exit_fragment_definition(ctx, fragment_definition);
    }

    fn enter_variable_definition(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        variable_definition: &'a VariableDefinition,
    ) {
        self.0.enter_variable_definition(ctx, variable_definition);
        self.1.enter_variable_definition(ctx, variable_definition);
    }

    fn exit_variable_definition(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        variable_definition: &'a VariableDefinition,
    ) {
        self.0.exit_variable_definition(ctx, variable_definition);
        self.1.exit_variable_definition(ctx, variable_definition);
    }

    fn enter_directive(&mut self, ctx: &mut VisitorContext<'a>, directive: &'a Directive) {
        self.0.enter_directive(ctx, directive);
        self.1.enter_directive(ctx, directive);
    }

    fn exit_directive(&mut self, ctx: &mut VisitorContext<'a>, directive: &'a Directive) {
        self.0.exit_directive(ctx, directive);
        self.1.exit_directive(ctx, directive);
    }

    fn enter_argument(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        pos: Pos,
        name: &'a str,
        value: &'a Value,
    ) {
        self.0.enter_argument(ctx, pos, name, value);
        self.1.enter_argument(ctx, pos, name, value);
    }

    fn exit_argument(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        pos: Pos,
        name: &'a str,
        value: &'a Value,
    ) {
        self.0.exit_argument(ctx, pos, name, value);
        self.1.exit_argument(ctx, pos, name, value);
    }

    fn enter_selection_set(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        selection_set: &'a SelectionSet,
    ) {
        self.0.enter_selection_set(ctx, selection_set);
        self.1.enter_selection_set(ctx, selection_set);
    }

    fn exit_selection_set(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        selection_set: &'a SelectionSet,
    ) {
        self.0.exit_selection_set(ctx, selection_set);
        self.1.exit_selection_set(ctx, selection_set);
    }

    fn enter_selection(&mut self, ctx: &mut VisitorContext<'a>, selection: &'a Selection) {
        self.0.enter_selection(ctx, selection);
        self.1.enter_selection(ctx, selection);
    }

    fn exit_selection(&mut self, ctx: &mut VisitorContext<'a>, selection: &'a Selection) {
        self.0.exit_selection(ctx, selection);
        self.1.exit_selection(ctx, selection);
    }

    fn enter_field(&mut self, ctx: &mut VisitorContext<'a>, field: &'a Field) {
        self.0.enter_field(ctx, field);
        self.1.enter_field(ctx, field);
    }

    fn exit_field(&mut self, ctx: &mut VisitorContext<'a>, field: &'a Field) {
        self.0.exit_field(ctx, field);
        self.1.exit_field(ctx, field);
    }

    fn enter_fragment_spread(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        fragment_spread: &'a FragmentSpread,
    ) {
        self.0.enter_fragment_spread(ctx, fragment_spread);
        self.1.enter_fragment_spread(ctx, fragment_spread);
    }

    fn exit_fragment_spread(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        fragment_spread: &'a FragmentSpread,
    ) {
        self.0.exit_fragment_spread(ctx, fragment_spread);
        self.1.exit_fragment_spread(ctx, fragment_spread);
    }

    fn enter_inline_fragment(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        inline_fragment: &'a InlineFragment,
    ) {
        self.0.enter_inline_fragment(ctx, inline_fragment);
        self.1.enter_inline_fragment(ctx, inline_fragment);
    }

    fn exit_inline_fragment(
        &mut self,
        ctx: &mut VisitorContext<'a>,
        inline_fragment: &'a InlineFragment,
    ) {
        self.0.exit_inline_fragment(ctx, inline_fragment);
        self.1.exit_inline_fragment(ctx, inline_fragment);
    }
}

pub fn visit<'a, V: Visitor<'a>>(v: &mut V, ctx: &mut VisitorContext<'a>, doc: &'a Document) {
    v.enter_document(ctx, doc);
    visit_definitions(v, ctx, doc);
    v.exit_document(ctx, doc);
}

fn visit_definitions<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    doc: &'a Document,
) {
    for d in &doc.definitions {
        match d {
            Definition::Operation(operation) => {
                visit_operation_definition(v, ctx, operation);
            }
            Definition::Fragment(fragment) => {
                let TypeCondition::On(name) = &fragment.type_condition;
                if let Some(ty) = ctx.registry.types.get(name) {
                    ctx.with_type(ty, |ctx| visit_fragment_definition(v, ctx, fragment));
                } else {
                    ctx.report_error(
                        vec![fragment.position],
                        format!("Unknown type \"{}\".", name),
                    );
                }
            }
        }
    }
}

fn visit_operation_definition<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    operation: &'a OperationDefinition,
) {
    v.enter_operation_definition(ctx, operation);
    match operation {
        OperationDefinition::SelectionSet(selection_set) => {
            ctx.with_type(&ctx.registry.types[&ctx.registry.query_type], |ctx| {
                visit_selection_set(v, ctx, selection_set)
            });
        }
        OperationDefinition::Query(query) => {
            ctx.with_type(&ctx.registry.types[&ctx.registry.query_type], |ctx| {
                visit_variable_definitions(v, ctx, &query.variable_definitions);
                visit_directives(v, ctx, &query.directives);
                visit_selection_set(v, ctx, &query.selection_set);
            });
        }
        OperationDefinition::Mutation(mutation) => {
            if let Some(mutation_type) = &ctx.registry.mutation_type {
                ctx.with_type(&ctx.registry.types[mutation_type], |ctx| {
                    visit_variable_definitions(v, ctx, &mutation.variable_definitions);
                    visit_directives(v, ctx, &mutation.directives);
                    visit_selection_set(v, ctx, &mutation.selection_set);
                });
            } else {
                ctx.report_error(
                    vec![mutation.position],
                    "Schema is not configured for mutations.",
                );
            }
        }
        OperationDefinition::Subscription(subscription) => {
            if let Some(subscription_type) = &ctx.registry.subscription_type {
                ctx.with_type(&ctx.registry.types[subscription_type], |ctx| {
                    visit_variable_definitions(v, ctx, &subscription.variable_definitions);
                    visit_directives(v, ctx, &subscription.directives);
                    visit_selection_set(v, ctx, &subscription.selection_set);
                });
            } else {
                ctx.report_error(
                    vec![subscription.position],
                    "Schema is not configured for subscriptions.",
                );
            }
        }
    }
    v.exit_operation_definition(ctx, operation);
}

fn visit_selection_set<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    selection_set: &'a SelectionSet,
) {
    if !selection_set.items.is_empty() {
        v.enter_selection_set(ctx, selection_set);
        for selection in &selection_set.items {
            visit_selection(v, ctx, selection);
        }
        v.exit_selection_set(ctx, selection_set);
    }
}

fn visit_selection<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    selection: &'a Selection,
) {
    v.enter_selection(ctx, selection);
    match selection {
        Selection::Field(field) => {
            if let Some(schema_field) = ctx.current_type().field_by_name(&field.name) {
                ctx.with_type(
                    ctx.registry
                        .basic_type_by_typename(&schema_field.ty)
                        .unwrap(),
                    |ctx| {
                        visit_field(v, ctx, field);
                    },
                );
            } else {
                ctx.report_error(
                    vec![field.position],
                    format!(
                        "Cannot query field \"{}\" on type \"{}\".",
                        field.name,
                        ctx.current_type().name()
                    ),
                );
            }
        }
        Selection::FragmentSpread(fragment_spread) => {
            visit_fragment_spread(v, ctx, fragment_spread)
        }
        Selection::InlineFragment(inline_fragment) => {
            if let Some(TypeCondition::On(name)) = &inline_fragment.type_condition {
                if let Some(ty) = ctx.registry.types.get(name) {
                    ctx.with_type(ty, |ctx| visit_inline_fragment(v, ctx, inline_fragment));
                } else {
                    ctx.report_error(
                        vec![inline_fragment.position],
                        format!("Unknown type \"{}\".", name),
                    );
                }
            }
        }
    }
    v.exit_selection(ctx, selection);
}

fn visit_field<'a, V: Visitor<'a>>(v: &mut V, ctx: &mut VisitorContext<'a>, field: &'a Field) {
    v.enter_field(ctx, field);
    visit_arguments(v, ctx, field.position, &field.arguments);
    visit_directives(v, ctx, &field.directives);
    visit_selection_set(v, ctx, &field.selection_set);
    v.exit_field(ctx, field);
}

fn visit_arguments<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    pos: Pos,
    arguments: &'a [(Name, Value)],
) {
    for (name, value) in arguments {
        v.enter_argument(ctx, pos, name, value);
        v.exit_argument(ctx, pos, name, value);
    }
}

fn visit_variable_definitions<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    variable_definitions: &'a [VariableDefinition],
) {
    for d in variable_definitions {
        v.enter_variable_definition(ctx, d);
        v.exit_variable_definition(ctx, d);
    }
}

fn visit_directives<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    directives: &'a [Directive],
) {
    for d in directives {
        v.enter_directive(ctx, d);
        visit_arguments(v, ctx, d.position, &d.arguments);
        v.exit_directive(ctx, d);
    }
}

fn visit_fragment_definition<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    fragment: &'a FragmentDefinition,
) {
    v.enter_fragment_definition(ctx, fragment);
    visit_directives(v, ctx, &fragment.directives);
    visit_selection_set(v, ctx, &fragment.selection_set);
    v.exit_fragment_definition(ctx, fragment);
}

fn visit_fragment_spread<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    fragment_spread: &'a FragmentSpread,
) {
    v.enter_fragment_spread(ctx, fragment_spread);
    visit_directives(v, ctx, &fragment_spread.directives);
    if let Some(fragment) = ctx.fragment(fragment_spread.fragment_name.as_str()) {
        visit_selection_set(v, ctx, &fragment.selection_set);
    }
    v.exit_fragment_spread(ctx, fragment_spread);
}

fn visit_inline_fragment<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut VisitorContext<'a>,
    inline_fragment: &'a InlineFragment,
) {
    v.enter_inline_fragment(ctx, inline_fragment);
    visit_directives(v, ctx, &inline_fragment.directives);
    visit_selection_set(v, ctx, &inline_fragment.selection_set);
    v.exit_inline_fragment(ctx, inline_fragment);
}
