#![feature(stmt_expr_attributes)]


use proc_macro::TokenStream;
use quote::{ToTokens, quote};
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


#[derive(Clone)]
struct EventField
{
    event: syn::Ident,
    ident: syn::Ident,
    kind: syn::Type,
}


enum Field
{
    Layer(SimpleField),
    Default(SimpleField),
    Event(EventField),
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
                let path = meta_list.path.require_ident()?;

                if path == "event"
                {
                    let event = meta_list.parse_args::<syn::Ident>()?;
                    return Ok(Self::Event(EventField {
                        event,
                        ident: field.ident.unwrap(),
                        kind: field.ty,
                    }));
                }

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
            Field::Event(EventField { ident, kind, .. }) => quote! {#ident: Guard<#kind>},
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
        syn::Attribute::parse_outer(input)?;
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

    pub fn event_fields(&self) -> Vec<&EventField>
    {
        self.fields
            .iter()
            .filter_map(|e| {
                if let Field::Event(event) = e
                {
                    Some(event)
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

        let mut kind = kind.clone();
        let mut suffix = None;

        if let syn::Type::Path(syn::TypePath { ref mut path, .. }) = kind
        {
            suffix = path
                .segments
                .pop()
                .map(|segment| segment.value().ident.clone());
        }

        tokens.extend(quote! {
            #ident: #kind #suffix ::default()
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


struct EventInitializer(EventField);
impl ToTokens for EventInitializer
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream)
    {
        let EventInitializer(EventField { event, ident, .. }) = self;

        tokens.extend(quote! {
            #ident: reg.get_unchecked::<EventEmitter<#event>>().subscribe()
        });
    }
}


#[proc_macro_attribute]
pub fn service(_attr: TokenStream, input: TokenStream) -> TokenStream
{
    let layer_struct: LayerStruct = syn::parse(input).expect("Failed to parse layer struct");

    let layer_fields = layer_struct.layer_fields();

    let layer_field_deps = layer_fields.iter().map(|l| {
        l.kind.clone()
    });

    let layer_field_names = layer_fields.iter().map(|l| l.ident.clone());

    let event_fields = layer_struct.event_fields();
    let event_dep_names = event_fields.iter().map(|e| e.event.clone());
    let event_fields = event_fields.iter().map(|e| EventInitializer((*e).clone()));

    let default_fields = layer_struct.default_fields();
    let data_fields = layer_struct.data_fields();

    let LayerStruct {
        visibility,
        name,
        generics,
        fields,
    } = &layer_struct;


    #[rustfmt::skip]
    quote!
    {        
        #visibility struct #name
        {
            #fields
        }

        impl ConstructLayer #generics for #name
        {
            fn construct(reg: &Registry #generics) -> Self
            {
                Self {
                    #(#layer_field_names: reg.get_unchecked(),)*
                    #(#default_fields,)*
                    #(#data_fields,)*
                    #(#event_fields,)*
                }
            }

            fn deps() -> Vec<LayerContext #generics>
            {
                vec![#(LayerContext::new::<#layer_field_deps>(),)* #(EventEmitter::<#event_dep_names>::ctx()),*]
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
        let args = self.0.iter();
        tokens.extend(quote! {#(.add_ctx::<#args>())*});
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
