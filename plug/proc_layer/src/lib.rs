#![feature(stmt_expr_attributes)]


use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned};


#[derive(Clone)]
struct DataField
{
    data: syn::Expr,
    ident: syn::Ident,
    kind: syn::Type,
}


#[derive(Clone)]
struct SimpleField
{
    ident: syn::Ident,
    kind: syn::Type,
}


enum Field
{
    Layer(SimpleField),
    Default(SimpleField),
    Data(DataField),
}


impl Parse for Field
{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self>
    {
        let attr = syn::Attribute::parse_outer(input)?;
        let field = syn::Field::parse_named(input)?;

        let attr = attr
            .first()
            .expect("Layer struct fields must have at least one attribute");

        match &attr.meta
        {
            syn::Meta::List(meta_list) =>
            {
                Err(syn::Error::new(meta_list.span(), "unexpected attribute"))
            }

            syn::Meta::NameValue(name_value) =>
            {
                if name_value.path.require_ident()? == "value"
                {
                    return Ok(Self::Data(DataField {
                        data: name_value.value.clone(),
                        ident: field.ident.unwrap(),
                        kind: field.ty,
                    }));
                }

                Err(syn::Error::new(name_value.span(), "unexpected attribute"))
            }

            syn::Meta::Path(path) =>
            {
                let path = path.require_ident()?;

                let field = SimpleField {
                    ident: field.ident.unwrap(),
                    kind: field.ty,
                };

                if path == "default"
                {
                    return Ok(Self::Default(field));
                }

                if path == "layer"
                {
                    return Ok(Self::Layer(field));
                }

                Err(syn::Error::new(path.span(), "unexpected attribute"))
            }
        }
    }
}


impl ToTokens for Field
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        let stream = match self
        {
            Field::Layer(SimpleField { ident, kind }) => quote! {#ident: Layer<#kind>},
            Field::Default(SimpleField { ident, kind }) => quote! {#ident: #kind},
            Field::Data(DataField { ident, kind, .. }) => quote! {#ident: #kind},
        };

        tokens.extend(stream);
    }
}


type Generics = syn::AngleBracketedGenericArguments;


struct LayerStruct
{
    visibility: syn::Visibility,
    name: syn::Ident,
    generics: Option<Generics>,
    fields: Punctuated<Field, syn::Token![,]>,
}


impl Parse for LayerStruct
{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self>
    {
        let content;
        let visibility = syn::Visibility::parse(input)?;
        input.parse::<syn::Token![struct]>()?;
        let name = syn::Ident::parse(input)?;
        let generics = Generics::parse(input).map(Some).unwrap_or(None);
        syn::braced!(content in input);
        let fields = content.parse_terminated(Field::parse, syn::Token![,])?;

        Ok(Self {
            visibility,
            name,
            generics,
            fields,
        })
    }
}


impl LayerStruct
{
    pub fn data_fields(&self) -> Vec<DataInitializer>
    {
        self.fields
            .iter()
            .filter_map(|e| {
                if let Field::Data(data) = e
                {
                    Some(DataInitializer(data.clone()))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    pub fn default_fields(&self) -> Vec<DefaultInitializer>
    {
        self.fields
            .iter()
            .filter_map(|e| {
                if let Field::Default(data) = e
                {
                    Some(DefaultInitializer(data.clone()))
                }
                else
                {
                    None
                }
            })
            .collect()
    }

    pub fn layer_fields(&self) -> Vec<&SimpleField>
    {
        self.fields
            .iter()
            .filter_map(|e| {
                if let Field::Layer(data) = e
                {
                    Some(data)
                }
                else
                {
                    None
                }
            })
            .collect()
    }
}


struct DefaultInitializer(SimpleField);
impl ToTokens for DefaultInitializer
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        let DefaultInitializer(SimpleField { ident, kind }) = self;

        tokens.extend(quote! {
            #ident: #kind::default()
        });
    }
}


struct DataInitializer(DataField);
impl ToTokens for DataInitializer
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        let DataInitializer(DataField { data, ident, .. }) = self;

        tokens.extend(quote! {
            #ident: #data
        });
    }
}


#[proc_macro_attribute]
pub fn layer_struct(attr: TokenStream, input: TokenStream) -> TokenStream
{
    let custom_context_identifier: Option<syn::Ident> =
        syn::parse(attr).expect("Optional context identifier must be a valid identifier");

    let layer_struct: LayerStruct = syn::parse(input).expect("Failed to parse layer struct");

    let layer_fields = layer_struct.layer_fields();
    let layer_field_deps = layer_fields.iter().map(|l| l.kind.clone());
    let layer_field_names = layer_fields.iter().map(|l| l.ident.clone());

    let default_fields = layer_struct.default_fields();
    let data_fields = layer_struct.data_fields();

    let context_identifier = custom_context_identifier
        .unwrap_or_else(|| format_ident!("{}", layer_struct.name.to_string().to_uppercase()));

    let LayerStruct {
        visibility,
        name,
        generics,
        fields,
    } = &layer_struct;


    #[rustfmt::skip]
    quote!
    {
        static #context_identifier: StaticContext #generics = static_context!(#name, [#(#layer_field_deps),*]);
        
        #visibility struct #name
        {
            #fields
        }

        impl LayerConstruct #generics for #name
        {
            fn construct(reg: &Registry #generics) -> Self
            {
                Self {
                    #(#layer_field_names: reg.get_unchecked(),)*
                    #(#default_fields,)*
                    #(#data_fields,)*
                }
            }
        }
    }
    .into()
}


struct BuildRegArgs(Punctuated<syn::Ident, syn::Token![,]>);
impl Parse for BuildRegArgs
{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self>
    {
        Ok(Self(Punctuated::parse_terminated(input)?))
    }
}


impl ToTokens for BuildRegArgs
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        let args = self
            .0
            .iter()
            .map(|a| format_ident!("{}", a.to_string().to_uppercase()));
        tokens.extend(quote! {#(.add_ctx(&#args))*});
    }
}


#[proc_macro]
pub fn build_reg(attr: TokenStream) -> TokenStream
{
    let attr = syn::parse_macro_input!(attr as BuildRegArgs);

    quote! {
        Resolver::new()
        #attr
        .build_reg()
        .expect("Failed to build registry")
    }
    .into()
}
