use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data};

/// Derive macro for WebSocket outgoing event types
/// 
/// This macro automatically implements the WsOutEvent trait for structs,
/// setting the event_name() to return the struct's name.
/// 
/// Usage:
/// ```rust
/// #[derive(WsOutEvent)]
/// pub struct MyEvent {
///     pub field: String,
/// }
/// ```
#[proc_macro_derive(WsOutEvent)]
pub fn derive_ws_out_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    // Validate that this is a struct
    match &input.data {
        Data::Struct(_) => {},
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "WsOutEvent can only be derived for structs"
            ).to_compile_error().into();
        }
    }
    
    let struct_name_str = struct_name.to_string();
    
    let expanded = quote! {
        impl crate::server::controllers::websocket::WsOutEvent for #struct_name {
            fn event_name() -> &'static str {
                #struct_name_str
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// Attribute macro for WebSocket incoming event types
/// 
/// This macro:
/// 1. Adds required derives for serde deserialization
/// 2. The build script automatically scans for this attribute and generates enum variants
/// 
/// Usage:
/// ```rust
/// #[ocloud_macros::WsIncomingEvent]
/// pub struct MyEvent {
///     pub field: String,
/// }
/// 
/// impl WsIncomingEvent for MyEvent {
///     async fn handle(self, state: &ServerState, connection_id: Uuid) -> Result<(), EventError> {
///         // Handle the event
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn WsIncomingEvent(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;
    
    // Validate that this is a struct
    match &input.data {
        Data::Struct(_) => {},
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "WsIncomingEvent can only be applied to structs"
            ).to_compile_error().into();
        }
    }
    
    let struct_name_str = struct_name.to_string();
    
    // Add required derives and auto-implement WsOutEvent trait
    let expanded = quote! {
        #[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
        #input
        
        // Auto-implement WsOutEvent trait with struct name
        impl crate::server::controllers::websocket::WsOutEvent for #struct_name {
            fn event_name() -> &'static str {
                #struct_name_str
            }
        }
        
        // Compile-time check that both required traits are implemented
        const _: fn() = || {
            fn _assert_traits<T: crate::server::controllers::websocket::WsIncomingEvent + crate::server::controllers::websocket::WsOutEvent>() {}
            _assert_traits::<#struct_name>();
        };
    };
    
    TokenStream::from(expanded)
}

