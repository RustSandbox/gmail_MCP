//! A library of procedural macros for defining MCP core tools.
//!
//! This crate provides macros for defining tools that can be used with the MCP system.
//! The main macro is `tool`, which is used to define a tool function that can be called
//! by the system.

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, ExprLit, FnArg, ItemFn, Lit, Meta, Pat, PatType, Token, Type,
};

#[derive(Debug)]
struct ToolArgs {
    name: Option<String>,
    description: Option<String>,
    annotations: ToolAnnotations,
}

#[derive(Debug)]
struct ToolAnnotations {
    title: Option<String>,
    read_only_hint: Option<bool>,
    destructive_hint: Option<bool>,
    idempotent_hint: Option<bool>,
    open_world_hint: Option<bool>,
}

impl Default for ToolAnnotations {
    fn default() -> Self {
        Self {
            title: None,
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
        }
    }
}

impl Parse for ToolArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut description = None;
        let mut annotations = ToolAnnotations::default();

        let meta_list: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;

        for meta in meta_list {
            match meta {
                Meta::NameValue(nv) => {
                    let ident = nv.path.get_ident().unwrap().to_string();
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(lit_str),
                        ..
                    }) = nv.value
                    {
                        match ident.as_str() {
                            "name" => name = Some(lit_str.value()),
                            "description" => description = Some(lit_str.value()),
                            _ => {
                                return Err(syn::Error::new_spanned(
                                    nv.path,
                                    format!("Unknown attribute: {}", ident),
                                ))
                            }
                        }
                    } else {
                        return Err(syn::Error::new_spanned(nv.value, "Expected string literal"));
                    }
                }
                Meta::List(list) if list.path.is_ident("annotations") => {
                    let nested: Punctuated<Meta, Token![,]> =
                        list.parse_args_with(Punctuated::parse_terminated)?;

                    for meta in nested {
                        if let Meta::NameValue(nv) = meta {
                            let key = nv.path.get_ident().unwrap().to_string();

                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(lit_str),
                                ..
                            }) = nv.value
                            {
                                if key == "title" {
                                    annotations.title = Some(lit_str.value());
                                } else {
                                    return Err(syn::Error::new_spanned(
                                        nv.path,
                                        format!("Unknown string annotation: {}", key),
                                    ));
                                }
                            } else if let Expr::Lit(ExprLit {
                                lit: Lit::Bool(lit_bool),
                                ..
                            }) = nv.value
                            {
                                match key.as_str() {
                                    "read_only_hint" | "readOnlyHint" => {
                                        annotations.read_only_hint = Some(lit_bool.value)
                                    }
                                    "destructive_hint" | "destructiveHint" => {
                                        annotations.destructive_hint = Some(lit_bool.value)
                                    }
                                    "idempotent_hint" | "idempotentHint" => {
                                        annotations.idempotent_hint = Some(lit_bool.value)
                                    }
                                    "open_world_hint" | "openWorldHint" => {
                                        annotations.open_world_hint = Some(lit_bool.value)
                                    }
                                    _ => {
                                        return Err(syn::Error::new_spanned(
                                            nv.path,
                                            format!("Unknown boolean annotation: {}", key),
                                        ))
                                    }
                                }
                            } else {
                                return Err(syn::Error::new_spanned(
                                    nv.value,
                                    "Expected string or boolean literal for annotation value",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new_spanned(
                                meta,
                                "Expected name-value pair for annotation",
                            ));
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Expected name-value pair or list",
                    ))
                }
            }
        }

        Ok(ToolArgs {
            name,
            description,
            annotations,
        })
    }
}

/// Defines a tool function that can be called by the MCP system.
///
/// This macro transforms an async function into a tool that can be registered with the MCP system.
/// It generates a corresponding structure with methods to get the tool definition and to handle
/// calls to the tool.
///
/// # Arguments
///
/// * `name` - The name of the tool (optional, defaults to the function name)
/// * `description` - A description of what the tool does
/// * `annotations` - Additional metadata for the tool:
///   * `title` - Display title for the tool (defaults to function name)
///   * `read_only_hint` - Whether the tool only reads data (defaults to false)
///   * `destructive_hint` - Whether the tool makes destructive changes (defaults to true)
///   * `idempotent_hint` - Whether the tool is idempotent (defaults to false)
///   * `open_world_hint` - Whether the tool can access resources outside the system (defaults to true)
///
/// # Example
///
/// ```rust
/// use mcp_core_macros::{tool, tool_param};
/// use mcp_core::types::ToolResponseContent;
/// use mcp_core::tool_text_content;
/// use anyhow::Result;
///
/// #[tool(name = "my_tool", description = "A tool with documented parameters", annotations(title = "My Tool"))]
/// async fn my_tool(
///     // A required parameter with description
///     required_param: tool_param!(String, description = "A required parameter"),
///     
///     // An optional parameter
///     optional_param: tool_param!(Option<String>, description = "An optional parameter"),
///     
///     // A hidden parameter that won't appear in the schema
///     internal_param: tool_param!(String, hidden)
/// ) -> Result<ToolResponseContent> {
///     // Implementation
///     Ok(tool_text_content!("Tool executed".to_string()))
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse::<ToolArgs>(args) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };

    let input_fn = match syn::parse::<ItemFn>(input.clone()) {
        Ok(input_fn) => input_fn,
        Err(e) => return e.to_compile_error().into(),
    };

    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let struct_name = format_ident!("{}", fn_name_str.to_case(Case::Pascal));
    let tool_name = args.name.unwrap_or(fn_name_str.clone());
    let tool_description = args.description.unwrap_or_default();

    // Tool annotations
    let title = args.annotations.title.unwrap_or(fn_name_str.clone());
    let read_only_hint = args.annotations.read_only_hint.unwrap_or(false);
    let destructive_hint = args.annotations.destructive_hint.unwrap_or(true);
    let idempotent_hint = args.annotations.idempotent_hint.unwrap_or(false);
    let open_world_hint = args.annotations.open_world_hint.unwrap_or(true);

    let mut param_defs = Vec::new();
    let mut param_names = Vec::new();
    let mut required_params = Vec::new();
    let mut hidden_params: Vec<String> = Vec::new();
    let mut param_descriptions = Vec::new();

    for arg in input_fn.sig.inputs.iter() {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            let mut is_hidden = false;
            let mut description: Option<String> = None;
            let mut is_optional = false;

            // Check for tool_type macro usage
            if let Type::Macro(type_macro) = &**ty {
                if let Some(ident) = type_macro.mac.path.get_ident() {
                    if ident == "tool_param" {
                        if let Ok(args) =
                            syn::parse2::<ToolParamArgs>(type_macro.mac.tokens.clone())
                        {
                            is_hidden = args.hidden;
                            description = args.description;

                            // Check if the parameter type is Option<T>
                            if let Type::Path(type_path) = &args.ty {
                                is_optional = type_path
                                    .path
                                    .segments
                                    .last()
                                    .map_or(false, |segment| segment.ident == "Option");
                            }
                        }
                    }
                }
            }

            if is_hidden {
                if let Pat::Ident(ident) = &**pat {
                    hidden_params.push(ident.ident.to_string());
                }
            }

            if let Pat::Ident(param_ident) = &**pat {
                let param_name = &param_ident.ident;
                let param_name_str = param_name.to_string();

                param_names.push(param_name.clone());

                // Check if the parameter type is Option<T>
                if !is_optional {
                    is_optional = if let Type::Path(type_path) = &**ty {
                        type_path
                            .path
                            .segments
                            .last()
                            .map_or(false, |segment| segment.ident == "Option")
                    } else {
                        false
                    }
                }

                // Only require non-optional, non-hidden
                if !is_optional && !is_hidden {
                    required_params.push(param_name_str.clone());
                }

                if let Some(desc) = description {
                    param_descriptions.push(quote! {
                        if name == #param_name_str {
                            prop_obj.insert("description".to_string(), serde_json::Value::String(#desc.to_string()));
                        }
                    });
                }

                param_defs.push(quote! {
                    #param_name: #ty
                });
            }
        }
    }

    let params_struct_name = format_ident!("{}Parameters", struct_name);
    let expanded = quote! {
        #[derive(serde::Deserialize, schemars::JsonSchema)]
        struct #params_struct_name {
            #(#param_defs,)*
        }

        #input_fn

        #[derive(Default)]
        pub struct #struct_name;

        impl #struct_name {
            pub fn tool() -> mcp_core::types::Tool {
                let schema = schemars::schema_for!(#params_struct_name);
                let mut schema = serde_json::to_value(schema.schema).unwrap_or_default();
                if let serde_json::Value::Object(ref mut map) = schema {
                    // Add required fields
                    map.insert("required".to_string(), serde_json::Value::Array(
                        vec![#(serde_json::Value::String(#required_params.to_string())),*]
                    ));
                    map.remove("title");

                    // Normalize property types
                    if let Some(serde_json::Value::Object(props)) = map.get_mut("properties") {
                        for (name, prop) in props.iter_mut() {
                            if let serde_json::Value::Object(prop_obj) = prop {
                                // Fix number types
                                if let Some(type_val) = prop_obj.get("type") {
                                    if type_val == "integer" || type_val == "number" || prop_obj.contains_key("format") {
                                        // Convert any numeric type to "number"
                                        prop_obj.insert("type".to_string(), serde_json::Value::String("number".to_string()));
                                        prop_obj.remove("format");
                                        prop_obj.remove("minimum");
                                        prop_obj.remove("maximum");
                                    }
                                }

                                // Fix optional types (array with null)
                                if let Some(serde_json::Value::Array(types)) = prop_obj.get("type") {
                                    if types.len() == 2 && types.contains(&serde_json::Value::String("null".to_string())) {
                                        let mut main_type = types.iter()
                                            .find(|&t| t != &serde_json::Value::String("null".to_string()))
                                            .cloned()
                                            .unwrap_or(serde_json::Value::String("string".to_string()));

                                        // If the main type is "integer", convert it to "number"
                                        if main_type == serde_json::Value::String("integer".to_string()) {
                                            main_type = serde_json::Value::String("number".to_string());
                                        }

                                        prop_obj.insert("type".to_string(), main_type);
                                    }
                                }

                                // Add descriptions if they exist
                                #(#param_descriptions)*
                            }
                        }

                        #(props.remove(#hidden_params);)*
                    }
                }

                let annotations = serde_json::json!({
                    "title": #title,
                    "readOnlyHint": #read_only_hint,
                    "destructiveHint": #destructive_hint,
                    "idempotentHint": #idempotent_hint,
                    "openWorldHint": #open_world_hint
                });

                mcp_core::types::Tool {
                    name: #tool_name.to_string(),
                    description: Some(#tool_description.to_string()),
                    input_schema: schema,
                    annotations: Some(mcp_core::types::ToolAnnotations {
                        title: Some(#title.to_string()),
                        read_only_hint: Some(#read_only_hint),
                        destructive_hint: Some(#destructive_hint),
                        idempotent_hint: Some(#idempotent_hint),
                        open_world_hint: Some(#open_world_hint),
                    }),
                }
            }

            pub fn call() -> mcp_core::tools::ToolHandlerFn {
                move |req: mcp_core::types::CallToolRequest| {
                    Box::pin(async move {
                        let params = match req.arguments {
                            Some(args) => serde_json::to_value(args).unwrap_or_default(),
                            None => serde_json::Value::Null,
                        };

                        let params: #params_struct_name = match serde_json::from_value(params) {
                            Ok(p) => p,
                            Err(e) => return mcp_core::types::CallToolResponse {
                                content: vec![mcp_core::types::ToolResponseContent::Text(
                                    mcp_core::types::TextContent {
                                        content_type: "text".to_string(),
                                        text: format!("Invalid parameters: {}", e),
                                        annotations: None,
                                    }
                                )],
                                is_error: Some(true),
                                meta: None,
                            },
                        };

                        match #fn_name(#(params.#param_names,)*).await {
                            Ok(response) => {
                                let content = if let Ok(vec_content) = serde_json::from_value::<Vec<mcp_core::types::ToolResponseContent>>(serde_json::to_value(&response).unwrap_or_default()) {
                                    vec_content
                                } else if let Ok(single_content) = serde_json::from_value::<mcp_core::types::ToolResponseContent>(serde_json::to_value(&response).unwrap_or_default()) {
                                    vec![single_content]
                                } else {
                                    vec![mcp_core::types::ToolResponseContent::Text(
                                        mcp_core::types::TextContent {
                                            content_type: "text".to_string(),
                                            text: format!("Invalid response type: {:?}", response),
                                            annotations: None,
                                        }
                                    )]
                                };

                                mcp_core::types::CallToolResponse {
                                    content,
                                    is_error: None,
                                    meta: None,
                                }
                            }
                            Err(e) => mcp_core::types::CallToolResponse {
                                content: vec![mcp_core::types::ToolResponseContent::Text(
                                    mcp_core::types::TextContent {
                                        content_type: "text".to_string(),
                                        text: format!("Tool execution error: {}", e),
                                        annotations: None,
                                    }
                                )],
                                is_error: Some(true),
                                meta: None,
                            },
                        }
                    })
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[derive(Debug)]
struct ToolParamArgs {
    ty: Type,
    hidden: bool,
    description: Option<String>,
}

impl Parse for ToolParamArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut hidden = false;
        let mut description = None;
        let ty = input.parse()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let meta_list: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;

            for meta in meta_list {
                match meta {
                    Meta::Path(path) if path.is_ident("hidden") => {
                        hidden = true;
                    }
                    Meta::NameValue(nv) if nv.path.is_ident("description") => {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &nv.value
                        {
                            description = Some(lit_str.value().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(ToolParamArgs {
            ty,
            hidden,
            description,
        })
    }
}

/// Defines a parameter for a tool function with additional metadata.
///
/// This macro allows specifying parameter attributes such as:
/// * `hidden` - Excludes the parameter from the generated schema
/// * `description` - Adds a description to the parameter in the schema
///
/// # Example
///
/// ```rust
/// use mcp_core_macros::{tool, tool_param};
/// use mcp_core::types::ToolResponseContent;
/// use mcp_core::tool_text_content;
/// use anyhow::Result;
///
/// #[tool(name = "my_tool", description = "A tool with documented parameters")]
/// async fn my_tool(
///     // A required parameter with description
///     required_param: tool_param!(String, description = "A required parameter"),
///     
///     // An optional parameter
///     optional_param: tool_param!(Option<String>, description = "An optional parameter"),
///     
///     // A hidden parameter that won't appear in the schema
///     internal_param: tool_param!(String, hidden)
/// ) -> Result<ToolResponseContent> {
///     // Implementation
///     Ok(tool_text_content!("Tool executed".to_string()))
/// }
/// ```
#[proc_macro]
pub fn tool_param(input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as ToolParamArgs);
    let ty = args.ty;

    TokenStream::from(quote! {
        #ty
    })
}
