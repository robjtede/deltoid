//! Code generation module
#![allow(unused)]

#[macro_use] mod trait_bounds;
pub(crate) mod enums;
pub(crate) mod markers;
pub(crate) mod structs;
pub(crate) mod where_clause;

use crate::{DeriveError, DeriveResult};
use proc_macro2::{
    Ident as Ident2, Literal as Literal2, TokenStream as TokenStream2
};
use syn::*;
use syn::punctuated::*;
use syn::token::Comma;
use quote::{format_ident, quote};


/// A description of the input type
#[derive(Clone, Debug)]
pub enum InputType {
    /// The input type is an enum
    Enum {
        /// The input enum's type name
        type_name: Ident2,
        /// The name of the generated delta type
        delta_type_name: Ident2,
        /// A description of the input enum's variants
        enum_variants: Vec<EnumVariant>,
        /// The input enum's type parameter declarations,
        /// including any trait bounds e.g. <T: Copy, U, V>
        type_param_decls: Punctuated<GenericParam, Comma>,
        /// The input enum's type parameters without trait bounds e.g. <T, U, V>
        type_params: Punctuated<Ident, Comma>,
        // TODO: defined lifetimes
        /// The input enum's where clause
        where_clause: WhereClause,
    },
    /// The input type is a struct
    Struct {
        /// Indicates whether the input struct is a named struct,
        /// a tuple struct or a unit struct
        struct_variant: StructVariant,
        /// The input struct's name
        type_name: Ident2,
        /// The generated delta type's name
        delta_type_name: Ident2,
        /// A description of the input struct's fields
        fields: Vec<FieldDesc>,
        /// The input struct's type parameter declarations including
        /// any trait bounds e.g. <T: Copy, U, V>
        type_param_decls: Punctuated<GenericParam, Comma>,
        /// The input struct's type parameters without
        /// trait bounds e.g. <T, U, V>
        type_params: Punctuated<Ident, Comma>,
        // TODO: defined lifetimes
        /// The input struct's where clause
        where_clause: WhereClause,
    },
    /// The input type is a union.  This is unsupported.
    #[allow(unused)]
    #[doc(hidden)]
    Union,
}

impl InputType {
    pub fn parse(input: &DeriveInput) -> DeriveResult<Self> {
        match &input.data {
            Data::Struct(DataStruct { fields, .. }) if !fields.is_empty() =>
                Self::parse_struct(input, fields),
            Data::Struct(DataStruct { .. }) =>
                Self::parse_unit_struct(input),
            Data::Enum(DataEnum { variants, .. }) =>
                Self::parse_enum(input, variants),
            Data::Union(_) => Ok(Self::Union),
        }
    }

    fn parse_struct(
        input: &DeriveInput,
        input_fields: &Fields,
    ) -> DeriveResult<Self> {
        let mut new = Self::new_struct(input);
        if let Self::Struct { struct_variant, fields, .. } = &mut new {
            for (fidx, field) in input_fields.iter().enumerate() {
                if let Some(field_ident) = field.ident.as_ref() {
                    *struct_variant = StructVariant::NamedStruct;
                    fields.push(FieldDesc::Named {
                        name: field_ident.clone(),
                        ty: field.ty.clone(),
                        ignore_field: markers::ignore_field(field),
                    });
                } else {
                    *struct_variant = StructVariant::TupleStruct;
                    fields.push(FieldDesc::Positional {
                        position: Literal2::usize_unsuffixed(fidx),
                        ty: field.ty.clone(),
                        ignore_field: markers::ignore_field(field),
                    });
                }
            }
            ensure!(
                fields.iter().all(|field| field.is_named()) ||
                fields.iter().all(|field| field.is_positional())
            )?;
        }
        Ok(new)
    }

    fn parse_unit_struct(input: &DeriveInput) -> DeriveResult<Self> {
        let mut new = Self::new_struct(input);
        if let Self::Struct { struct_variant, .. } = &mut new {
            *struct_variant = StructVariant::UnitStruct;
        }
        Ok(new)
    }

    fn parse_enum(
        input: &DeriveInput,
        input_enum_variants: &Punctuated<Variant, Comma>,
    ) -> DeriveResult<Self> {
        let mut new = Self::new_enum(input);
        if let Self::Enum { enum_variants, .. } = &mut new {
            for iev in input_enum_variants {
                let mut variant = EnumVariant::new(&iev.ident);
                for (fidx, field) in iev.fields.iter().enumerate() {
                    if let Some(field_ident) = field.ident.as_ref() {
                        variant.struct_variant = StructVariant::NamedStruct;
                        variant.add_field(FieldDesc::Named {
                            name: field_ident.clone(),
                            ty: field.ty.clone(),
                            ignore_field: markers::ignore_field(field),
                        });
                    } else {
                        variant.struct_variant = StructVariant::TupleStruct;
                        variant.add_field(FieldDesc::Positional {
                            position: Literal2::usize_unsuffixed(fidx),
                            ty: field.ty.clone(),
                            ignore_field: markers::ignore_field(field),
                        });
                    }
                }
                ensure!(
                    variant.fields().all(|field| field.is_named()) ||
                    variant.fields().all(|field| field.is_positional())
                )?;
                enum_variants.push(variant);
            }
        }
        Ok(new)
    }

    fn new_enum(input: &DeriveInput) -> Self {
        Self::Enum {
            type_name: input.ident.clone(),
            delta_type_name: format_ident!("{}Delta", &input.ident),
            enum_variants: vec![],
            type_param_decls: input.generics.params.clone(),
            type_params: input.generics.type_params()
                .map(|type_param| type_param.ident.clone())
                .collect(),
            where_clause: input.generics.where_clause.clone()
                .unwrap_or_else(where_clause::empty),
        }
    }

    fn new_struct(input: &DeriveInput) -> Self {
        Self::Struct {
            struct_variant: StructVariant::UnitStruct,
            type_name: input.ident.clone(),
            delta_type_name: format_ident!("{}Delta", &input.ident),
            fields: vec![],
            type_param_decls: input.generics.params.clone(),
            type_params: input.generics.type_params()
                .map(|type_param| type_param.ident.clone())
                .collect(),
            where_clause: input.generics.where_clause.clone()
                .unwrap_or_else(where_clause::empty),
        }
    }

    pub fn is_enum(&self) -> bool { matches!(self, Self::Enum { .. }) }

    pub fn is_struct(&self) -> bool { matches!(self, Self::Struct { .. }) }

    #[cfg(feature = "dump-expansions--unstable")]
    pub fn type_name(&self) -> &Ident2 {
        match self {
            Self::Enum   { type_name, .. } => type_name,
            Self::Struct { type_name, .. } => type_name,
            Self::Union => panic!("Unions are not supported."),
        }
    }

    pub fn type_param_decls(&self) -> &Punctuated<GenericParam, Comma> {
        match self {
            Self::Enum   { type_param_decls, .. } => type_param_decls,
            Self::Struct { type_param_decls, .. } => type_param_decls,
            Self::Union => panic!("Unions are not supported."),
        }
    }

    /// Return the input type's `WhereClause`.
    pub fn where_clause(&self) -> &WhereClause {
        match self {
            Self::Enum   { where_clause, .. } => where_clause,
            Self::Struct { where_clause, .. } => where_clause,
            Self::Union => panic!("Unions are not supported."),
        }
    }

    pub fn define_delta_type(&self) -> DeriveResult<TokenStream2> {
        Ok(match self {
            Self::Struct { .. } => structs::define_delta_struct(self)?,
            Self::Enum   { .. } => enums::define_delta_enum(self)?,
            Self::Union => panic!("Unions are not supported."),
        })
    }

    // #[allow(non_snake_case)]
    // pub fn define_Deltoid_impl(&self) -> DeriveResult<TokenStream2> {
    //     Ok(match self {
    //         Self::Struct { .. } => structs::define_Deltoid_impl(self)?,
    //         Self::Enum   { .. } => enums::define_Deltoid_impl(self)?,
    //         Self::Union => panic!("Unions are not supported."),
    //     })
    // }

    #[allow(non_snake_case)]
    pub fn define_Core_impl(&self) -> DeriveResult<TokenStream2> {
        Ok(match self {
            Self::Struct { .. } => structs::define_Core_impl(self)?,
            Self::Enum   { .. } => enums::define_Core_impl(self)?,
            Self::Union => panic!("Unions are not supported."),
        })
    }

    #[allow(non_snake_case)]
    pub fn define_Apply_impl(&self) -> DeriveResult<TokenStream2> {
        Ok(match self {
            Self::Struct { .. } => structs::define_Apply_impl(self)?,
            Self::Enum   { .. } => enums::define_Apply_impl(self)?,
            Self::Union => panic!("Unions are not supported."),
        })
    }

    #[allow(non_snake_case)]
    pub fn define_Delta_impl(&self) -> DeriveResult<TokenStream2> {
        Ok(match self {
            Self::Struct { .. } => structs::define_Delta_impl(self)?,
            Self::Enum   { .. } => enums::define_Delta_impl(self)?,
            Self::Union => panic!("Unions are not supported."),
        })
    }

    #[allow(non_snake_case)]
    pub fn define_FromDelta_impl(&self) -> DeriveResult<TokenStream2> {
        Ok(match self {
            Self::Struct { .. } => structs::define_FromDelta_impl(self)?,
            Self::Enum   { .. } => enums::define_FromDelta_impl(self)?,
            Self::Union => panic!("Unions are not supported."),
        })
    }

    #[allow(non_snake_case)]
    pub fn define_IntoDelta_impl(&self) -> DeriveResult<TokenStream2> {
        Ok(match self {
            Self::Struct { .. } => structs::define_IntoDelta_impl(self)?,
            Self::Enum   { .. } => enums::define_IntoDelta_impl(self)?,
            Self::Union => panic!("Unions are not supported."),
        })
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructVariant {
    /// A "named struct" i.e. a struct with named fields
    /// e.g. `struct Foo { bar: u8 }`
    NamedStruct,
    /// A tuple struct i.e. unnamed/positional fields
    /// e.g. `struct Baz(String);`
    TupleStruct,
    /// A unit struct i.e. a struct with no fields at all e.g. `struct Quux;`
    UnitStruct,
}


#[derive(Clone, Debug)]
pub struct EnumVariant {
    struct_variant: StructVariant,
    name: Ident2,
    fields: Vec<FieldDesc>,
}

impl EnumVariant {
    pub fn new(name: &Ident2) -> Self {
        Self {
            struct_variant: StructVariant::UnitStruct,
            name: name.clone(),
            fields: vec![],
        }
    }

    pub fn add_field(&mut self, field: FieldDesc) {
        self.fields.push(field);
    }

    pub fn fields(&self) -> impl Iterator<Item = &FieldDesc> {
        self.fields.iter()
    }
}




#[derive(Clone, Debug)]
/// A description of a field that's part of a `struct` or an `enum`.
pub enum FieldDesc {
    /// A field that's part of a named struct
    Named {
        name: Ident2,
        ty: Type,
        ignore_field: bool,
    },
    /// A field that's part of a tuple struct
    Positional {
        position: Literal2,
        ty: Type,
        ignore_field: bool,
    }
}

#[allow(non_snake_case)]
impl FieldDesc {
    pub fn is_named(&self) -> bool {
        matches!(self, Self::Named { .. })
    }

    pub fn is_positional(&self) -> bool {
        matches!(self, Self::Positional { .. })
    }

    pub fn name_ref(&self) -> DeriveResult<&Ident2> {
        match self {
            Self::Named { name, .. } => Ok(name),
            Self::Positional { .. } => Err(DeriveError::ExpectedNamedField),
        }
    }

    pub fn pos_ref(&self) -> DeriveResult<&Literal2> {
        match self {
            Self::Named { .. } => Err(DeriveError::ExpectedPositionalField),
            Self::Positional { position, .. } => Ok(position),
        }
    }

    pub fn type_ref(&self) -> &Type {
        match self {
            Self::Named { ty, .. } => ty,
            Self::Positional { ty, .. } => ty,
        }
    }

    /// Returns true iff. the field was marked with `#[delta(ignore_field)]`.
    pub fn ignore_field(&self) -> bool {
        match self {
            Self::Named      { ignore_field, .. } => *ignore_field,
            Self::Positional { ignore_field, .. } => *ignore_field,
        }
    }

    /// Return the tokens for the type of `self`.
    pub fn type_tokens(&self) -> TokenStream2 {
        let ty: &Type = self.type_ref();
        if self.ignore_field() {
            quote! { std::marker::PhantomData<#ty> }
        } else {
            quote! { Option<<#ty as deltoid::Core>::Delta> }
        }
    }
}