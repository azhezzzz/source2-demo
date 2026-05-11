#![doc = include_str!("../README.md")]
//!
//! # Procedural Macros for Source 2 Replay Parser
//!
//! This crate provides procedural macros that simplify implementing the Observer trait.
//!
//! ## Overview
//!
//! The macros automate much of the boilerplate required for handling replay events:
//! - Automatically implement the Observer trait
//! - Generate protobuf message decoding
//! - Set up interest flags based on used methods
//! - Handle optional `Context` parameters
//!
//! ## Main Macros
//!
//! ### `#[observer]` - Main implementation macro
//!
//! Automatically implements the Observer trait with all necessary methods.
//! Takes an impl block with event handlers and generates the full implementation.
//!
//! ```no_run
//! # use source2_demo::prelude::*;
//! #[derive(Default)]
//! struct MyObserver;
//!
//! #[observer]
//! impl MyObserver {
//!     #[on_tick_start]
//!     fn on_tick(&mut self, ctx: &Context) -> ObserverResult {
//!         println!("Tick: {}", ctx.tick());
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ### Event Handler Macros
//!
//! These attributes mark methods as event handlers within an #[observer] impl:
//!
//! - `#[on_message]` - Handles protobuf messages
//! - `#[on_tick_start]` - Called at tick start
//! - `#[on_tick_end]` - Called at tick end
//! - `#[on_entity]` - Called for entity changes
//! - `#[on_game_event]` - Called for game events
//! - `#[on_string_table]` - Called for string table updates
//! - `#[on_stop]` - Called when replay ends
//! - `#[on_combat_log]` - Called for combat log entries (Dota 2 only)
//!
//! ### Trait Markers
//!
//! These mark impl blocks with which data types to track:
//!
//! - `#[uses_entities]` - Track entities
//! - `#[uses_string_tables]` - Track string tables
//! - `#[uses_game_events]` - Track game events
//! - `#[uses_combat_log]` - Track combat log (Dota 2 only)
//!
//! ## How the Macros Work
//!
//! The `#[observer]` macro:
//!
//! 1. Scans all methods for event handler attributes
//! 2. Collects parameter types to generate interest flags
//! 3. Creates decoding logic for protobuf messages
//! 4. Generates the full `Observer` trait implementation
//! 5. Handles optional parameters (Context is optional)
//!
//! ## Complete Example
//!
//! ```no_run
//! use source2_demo::prelude::*;
//!
//! #[derive(Default)]
//! struct GameAnalyzer {
//!     tick_count: u32,
//!     entity_count: u32,
//! }
//!
//! #[observer]
//! #[uses_entities]
//! impl GameAnalyzer {
//!     #[on_tick_start]
//!     fn on_tick(&mut self, ctx: &Context) -> ObserverResult {
//!         self.tick_count += 1;
//!         Ok(())
//!     }
//!
//!     #[on_entity]
//!     fn on_entity_create(&mut self, event: EntityEvents, entity: &Entity) -> ObserverResult {
//!         if event == EntityEvents::Created {
//!             self.entity_count += 1;
//!         }
//!         Ok(())
//!     }
//!
//!     #[on_message]
//!     fn on_chat(&mut self, msg: CDotaUserMsgChatMessage) -> ObserverResult {
//!         println!("Chat: {}", msg.message_text());
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Key Features
//!
//! - **Automatic Interest Management**: Macro automatically determines which interests
//!   to set based on which handler methods are defined
//! - **Optional Context**: Pass `ctx: &Context` to any handler or omit it
//! - **Automatic Message Decoding**: `#[on_message]` handlers automatically decode
//!   protobuf messages based on parameter type
//! - **Filtering Attributes**: Filter events with string arguments:
//!   - `#[on_entity("CDOTA_Unit_Hero_Axe")]` - Only heroes named Axe
//!   - `#[on_game_event("player_death")]` - Only death events
//!   - `#[on_string_table("userinfo")]` - Only userinfo table updates
//! - **Game-Specific Handlers**: Use `#[on_dota_user_message]`, `#[on_citadel_user_message]`,
//!   etc. for game-specific message types

mod protobuf_map;

use crate::protobuf_map::get_enum_from_struct;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse::Parser, parse_macro_input, punctuated::Punctuated, FnArg, Ident, ItemImpl, Token, Type};

#[allow(unused_mut)]
#[proc_macro_attribute]
/// Implements the Observer trait for your struct.
///
/// This is the main macro that ties everything together. Apply it to an `impl` block
/// that contains event handler methods marked with `#[on_*]` attributes.
///
/// # What It Does
///
/// - Automatically implements the Observer trait
/// - Generates code to call all handler methods at appropriate times
/// - Sets up interest flags based on which handlers are defined
/// - Decodes protobuf messages and passes them to handlers
/// - Filters events based on optional string arguments
///
/// # Handler Attributes
///
/// Use these attributes inside the impl block to mark event handlers:
///
/// - `#[on_tick_start]` - Called at the start of each tick
/// - `#[on_tick_end]` - Called at the end of each tick
/// - `#[on_entity]` - Called when entities change
/// - `#[on_entity("ClassName")]` - Only for specific entity classes
/// - `#[on_message]` - Called for protobuf messages (type inferred from param)
/// - `#[on_game_event]` - Called for all game events
/// - `#[on_game_event("event_name")]` - Only for specific events
/// - `#[on_string_table]` - Called when string tables update
/// - `#[on_string_table("table_name")]` - Only for specific tables
/// - `#[on_stop]` - Called when replay ends
/// - `#[on_combat_log]` - Called for combat log entries (Dota 2 only)
///
/// # Trait Attributes
///
/// Apply these to the `impl` block or individual methods to enable tracking:
///
/// - `#[uses_entities]` - Enable entity tracking
/// - `#[uses_string_tables]` - Enable string table tracking
/// - `#[uses_game_events]` - Enable game event tracking
/// - `#[uses_combat_log]` - Enable combat log tracking (Dota 2 only)
///
/// # Parameter Guidelines
///
/// Handlers can have these parameters (all optional except `&mut self`):
/// - `ctx: &Context` - Current replay state (always optional)
/// - Specific parameters depending on the handler:
///   - `event: EntityEvents` (on_entity)
///   - `entity: &Entity` (on_entity)
///   - `ge: &GameEvent` (on_game_event)
///   - `table: &StringTable` (on_string_table)
///   - `modified: &[i32]` (on_string_table)
///   - `cle: &CombatLogEntry` (on_combat_log)
///   - Protobuf message types (on_message)
///
/// # Examples
///
/// ## Basic observer
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[derive(Default)]
/// struct BasicObserver;
///
/// #[observer]
/// impl BasicObserver {
///     #[on_tick_start]
///     fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
///         println!("Tick: {}", ctx.tick());
///         Ok(())
///     }
/// }
/// ```
///
/// ## With entity tracking
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[derive(Default)]
/// struct EntityTracker;
///
/// #[observer]
/// #[uses_entities]
/// impl EntityTracker {
///     #[on_entity]
///     fn on_hero_created(&mut self, event: EntityEvents, entity: &Entity) -> ObserverResult {
///         if event == EntityEvents::Created && entity.class().name().starts_with("CDOTA_Unit_Hero_") {
///             println!("Hero created: {}", entity.class().name());
///         }
///         Ok(())
///     }
/// }
/// ```
///
/// ## With multiple handlers
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[derive(Default)]
/// struct ComplexObserver {
///     ticks: u32,
///     messages: u32,
/// }
///
/// #[observer]
/// impl ComplexObserver {
///     #[on_tick_start]
///     fn on_tick(&mut self, ctx: &Context) -> ObserverResult {
///         self.ticks += 1;
///         Ok(())
///     }
///
///     #[on_message]
///     fn on_chat(&mut self, msg: CDotaUserMsgChatMessage) -> ObserverResult {
///         self.messages += 1;
///         println!("Message: {}", msg.message_text());
///         Ok(())
///     }
///
///     #[on_stop]
///     fn on_replay_end(&mut self) -> ObserverResult {
///         println!("Total ticks: {}, messages: {}", self.ticks, self.messages);
///         Ok(())
///     }
/// }
/// ```
///
/// ## With game event filtering
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[derive(Default)]
/// struct DeathTracker {
///     deaths: u32,
/// }
///
/// #[observer]
/// impl DeathTracker {
///     #[on_game_event("player_death")]
///     fn on_death(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
///         self.deaths += 1;
///         Ok(())
///     }
/// }
/// ```
///
/// # Interest Flags
///
/// The macro automatically determines which interest flags to set based on which
/// handlers are defined. For example:
/// - If you have `#[on_tick_start]`, `Interests::TICK_START` is added
/// - If you have `#[on_entity]`, both `ENABLE_ENTITY` and `TRACK_ENTITY` are added
/// - If you have `#[on_message]` handlers, appropriate message interest flags are added
///
/// You can also use trait attributes like `#[uses_entities]` to manually ensure
/// certain interests are set.
pub fn observer(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut mode_all = false;

    if !attr.is_empty() {
        let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
        if let Ok(idents) = parser.parse(attr.clone()) {
            for id in idents {
                if id == "manual" {
                    mode_all = false;
                }
                if id == "all" {
                    mode_all = true;
                }
            }
        }
    }

    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty;

    let mut interests = quote!(::source2_demo::Interests::empty());
    macro_rules! add_flag {
        ($flag:ident) => {
            interests = quote!(#interests | ::source2_demo::Interests::$flag);
        };
    }

    for a in &input.attrs {
        if a.path().is_ident("uses_entities") {
            add_flag!(ENABLE_ENTITY);
        }
        if a.path().is_ident("uses_string_tables") {
            add_flag!(ENABLE_STRINGTAB);
        }
        if a.path().is_ident("uses_game_events") {
            add_flag!(BASE_GE);
        }
        #[cfg(feature = "dota")]
        if a.path().is_ident("uses_combat_log") {
            add_flag!(ENABLE_STRINGTAB);
            add_flag!(COMBAT_LOG);
        }
    }

    let mut has_demo = false;
    let mut has_net = false;
    let mut has_svc = false;
    let mut has_base_um = false;
    let mut has_base_ge = false;
    let mut has_tick_start = false;
    let mut has_tick_end = false;
    let mut has_entity = false;
    let mut has_entity_track = false;
    let mut has_string_table = false;
    let mut has_string_table_track = false;
    let mut has_stop = false;

    #[cfg(feature = "dota")]
    let mut has_dota_um = false;
    #[cfg(feature = "dota")]
    let mut has_combat_log = false;

    #[cfg(feature = "citadel")]
    let mut has_cita_um = false;
    #[cfg(feature = "citadel")]
    let mut has_cita_ge = false;

    #[cfg(feature = "cs2")]
    let mut has_cs2_um = false;
    #[cfg(feature = "cs2")]
    let mut has_cs2_ge = false;

    #[cfg(feature = "dota")]
    let mut on_combat_log_body = quote!();
    #[cfg(feature = "dota")]
    let mut on_dota_user_message_body = quote!();

    #[cfg(feature = "citadel")]
    let mut on_citadel_user_message_body = quote!();
    #[cfg(feature = "citadel")]
    let mut on_citadel_game_event_body = quote!();

    #[cfg(feature = "cs2")]
    let mut on_cs2_user_message_body = quote!();
    #[cfg(feature = "cs2")]
    let mut on_cs2_game_event_body = quote!();

    let mut on_svc_message_body = quote!();
    let mut on_net_message_body = quote!();
    let mut on_base_game_event_body = quote!();
    let mut on_demo_command_body = quote!();
    let mut on_base_user_message_body = quote!();
    let mut on_tick_start_body = quote!();
    let mut on_tick_end_body = quote!();
    let mut on_entity_body = quote!();
    let mut on_game_event_body = quote!();
    let mut on_string_table_body = quote!();
    let mut on_stop_body = quote!();

    for item in &input.items {
        if let syn::ImplItem::Fn(method) = item {
            for attr in &method.attrs {
                for a in &method.attrs {
                    if a.path().is_ident("uses_entities") {
                        add_flag!(ENABLE_ENTITY);
                    }
                    if a.path().is_ident("uses_string_tables") {
                        add_flag!(ENABLE_STRINGTAB);
                    }
                    if a.path().is_ident("uses_game_events") {
                        add_flag!(BASE_GE);
                    }
                    #[cfg(feature = "dota")]
                    if a.path().is_ident("uses_combat_log") {
                        add_flag!(DOTA_UM);
                    }
                }

                let method_name = method.sig.ident.clone();
                let mut args = vec![];

                let (arg_type, _) = get_arg_type(method, 1);
                if arg_type.to_token_stream().to_string() == "Context" {
                    args.push(quote! { ctx })
                }

                if let Some(ident) = attr.path().get_ident() {
                    match ident.to_string().as_str() {
                        "on_tick_start" => {
                            has_tick_start = true;
                            on_tick_start_body.extend(quote! {
                                self.#method_name(#(#args),*)?;
                            })
                        }
                        "on_tick_end" => {
                            has_tick_end = true;
                            on_tick_end_body.extend(quote! {
                                self.#method_name(#(#args),*)?;
                            })
                        }
                        "on_stop" => {
                            has_stop = true;
                            on_stop_body.extend(quote! {
                                self.#method_name(#(#args),*)?;
                            });
                        }
                        #[cfg(feature = "dota")]
                        "on_combat_log" => {
                            args.push(quote! { cle });

                            on_combat_log_body.extend(quote! {
                                self.#method_name(#(#args),*)?;
                            })
                        }
                        "on_entity" => {
                            has_entity_track = true;
                            let (arg_type, is_ref) = get_arg_type(method, args.len() + 1);

                            if arg_type.to_token_stream().to_string() == "EntityEvents" {
                                if is_ref {
                                    args.push(quote! { &event });
                                } else {
                                    args.push(quote! { event });
                                }
                            }

                            args.push(quote! { entity });

                            on_entity_body.extend(if let Ok(entity_class) = attr.parse_args::<syn::LitStr>() {
                                quote! {
                                    if entity.class().name() == #entity_class {
                                        self.#method_name(#(#args),*)?;
                                    }
                                }
                            } else {
                                quote! {
                                    self.#method_name(#(#args),*)?;
                                }
                            });
                        }
                        "on_game_event" => {
                            args.push(quote! { ge });
                            on_game_event_body.extend(if let Ok(event_name) = attr.parse_args::<syn::LitStr>() {
                                quote! {
                                    if ge.name() == #event_name {
                                        self.#method_name(#(#args),*)?;
                                    }
                                }
                            } else {
                                quote! {
                                    self.#method_name(#(#args),*)?;
                                }
                            });
                        }
                        "on_string_table" => {
                            has_string_table_track = true;
                            args.push(quote! { table });
                            args.push(quote! { modified });
                            on_string_table_body.extend(if let Ok(table_name) = attr.parse_args::<syn::LitStr>() {
                                quote! {
                                    if table.name() == #table_name {
                                        self.#method_name(#(#args),*)?;
                                    }
                                }
                            } else {
                                quote! {
                                    self.#method_name(#(#args),*)?;
                                }
                            });
                        }
                        "on_message" => {
                            let (arg_type, is_ref) = get_arg_type(method, args.len() + 1);
                            let enum_type = get_enum_from_struct(arg_type.to_token_stream().to_string().as_str());
                            let type_string = enum_type.to_token_stream().to_string();
                            let root = type_string.split("::").collect::<Vec<_>>()[0].trim();

                            args.push(if is_ref {
                                quote! { &message }
                            } else {
                                quote! { message }
                            });

                            macro_rules! extend {
                                ($body: ident) => {
                                    $body.extend(quote! {
                                        if msg_type == #enum_type {
                                            if let Ok(message) = #arg_type::decode(msg) {
                                                self.#method_name(#(#args),*)?;
                                            }
                                        }
                                    })
                                };
                            }

                            match root {
                                "EDemoCommands" => has_demo = true,
                                "EBaseUserMessages" => has_base_um = true,
                                "EBaseGameEvents" => has_base_ge = true,
                                "SvcMessages" => has_svc = true,
                                "NetMessages" => has_net = true,
                                #[cfg(feature = "dota")]
                                "EDotaUserMessages" => has_dota_um = true,
                                #[cfg(feature = "citadel")]
                                "CitadelUserMessageIds" => has_cita_um = true,
                                #[cfg(feature = "citadel")]
                                "ECitadelGameEvents" => has_cita_ge = true,
                                #[cfg(feature = "cs2")]
                                "ECstrike15UserMessages" => has_cs2_um = true,
                                #[cfg(feature = "cs2")]
                                "ECsgoGameEvents" => has_cs2_ge = true,
                                _ => {}
                            }

                            match root {
                                "EDemoCommands" => extend!(on_demo_command_body),
                                "EBaseUserMessages" => extend!(on_base_user_message_body),
                                "EBaseGameEvents" => extend!(on_base_game_event_body),
                                "SvcMessages" => extend!(on_svc_message_body),
                                "NetMessages" => extend!(on_net_message_body),

                                #[cfg(feature = "dota")]
                                "EDotaUserMessages" => extend!(on_dota_user_message_body),
                                #[cfg(feature = "citadel")]
                                "CitadelUserMessageIds" => extend!(on_citadel_user_message_body),
                                #[cfg(feature = "citadel")]
                                "ECitadelGameEvents" => extend!(on_citadel_game_event_body),
                                #[cfg(feature = "cs2")]
                                "ECstrike15UserMessages" => extend!(on_cs2_user_message_body),
                                #[cfg(feature = "cs2")]
                                "ECsgoGameEvents" => extend!(on_cs2_game_event_body),

                                x => unreachable!("{}", x),
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let mut obs_body = quote! {
        fn on_base_user_message(
            &mut self,
            ctx: &Context,
            msg_type: EBaseUserMessages,
            msg: &[u8],
        ) -> ObserverResult {
            #on_base_user_message_body
            Ok(())
        }

        fn on_svc_message(
            &mut self,
            ctx: &Context,
            msg_type: SvcMessages,
            msg: &[u8],
        ) -> ObserverResult {
            #on_svc_message_body
            Ok(())
        }

        fn on_net_message(
            &mut self,
            ctx: &Context,
            msg_type: NetMessages,
            msg: &[u8],
        ) -> ObserverResult {
            #on_net_message_body
            Ok(())
        }

        fn on_base_game_event(
            &mut self,
            ctx: &Context,
            msg_type: EBaseGameEvents,
            msg: &[u8],
        ) -> ObserverResult {
            #on_base_game_event_body
            Ok(())
        }

        fn on_demo_command(
            &mut self,
            ctx: &Context,
            msg_type: EDemoCommands,
            msg: &[u8],
        ) -> ObserverResult {
            #on_demo_command_body
            Ok(())
        }

        fn on_tick_start(
            &mut self,
            ctx: &Context,
        ) -> ObserverResult {
            #on_tick_start_body
            Ok(())
        }

        fn on_tick_end(
            &mut self,
            ctx: &Context,
        ) -> ObserverResult {
            #on_tick_end_body
            Ok(())
        }

        fn on_entity(
            &mut self,
            ctx: &Context,
            event: EntityEvents,
            entity: &Entity,
        ) -> ObserverResult {
            #on_entity_body
            Ok(())
        }

        fn on_game_event(
            &mut self,
            ctx: &Context,
            ge: &GameEvent
        ) -> ObserverResult {
            #on_game_event_body
            Ok(())
        }

        fn on_string_table(
            &mut self,
            ctx: &Context,
            table: &StringTable,
            modified: &[i32]
        ) -> ObserverResult {
            #on_string_table_body
            Ok(())
        }

        fn on_stop(
            &mut self,
            ctx: &Context,
        ) -> ObserverResult {
            #on_stop_body
            Ok(())
        }
    };

    #[cfg(feature = "dota")]
    obs_body.extend(quote! {
        fn on_combat_log(
            &mut self,
            ctx: &Context,
            cle: &CombatLogEntry
        ) -> ObserverResult {
            #on_combat_log_body
            Ok(())
        }

        fn on_dota_user_message(
            &mut self,
            ctx: &Context,
            msg_type: EDotaUserMessages,
            msg: &[u8],
        ) -> ObserverResult {
            #on_dota_user_message_body
            Ok(())
        }
    });

    #[cfg(feature = "citadel")]
    obs_body.extend(quote! {
        fn on_citadel_user_message(
            &mut self,
            ctx: &Context,
            msg_type: CitadelUserMessageIds,
            msg: &[u8],
        ) -> ObserverResult {
            #on_citadel_user_message_body
            Ok(())
        }

        fn on_citadel_game_event(
            &mut self,
            ctx: &Context,
            msg_type: ECitadelGameEvents,
            msg: &[u8],
        ) -> ObserverResult {
            #on_citadel_game_event_body
            Ok(())
        }
    });

    #[cfg(feature = "cs2")]
    obs_body.extend(quote! {
        fn on_cs2_user_message(
            &mut self,
            ctx: &Context,
            msg_type: ECstrike15UserMessages,
            msg: &[u8],
        ) -> ObserverResult {
            #on_cs2_user_message_body
            Ok(())
        }

        fn on_cs2_game_event(
            &mut self,
            ctx: &Context,
            msg_type: ECsgoGameEvents,
            msg: &[u8],
        ) -> ObserverResult {
            #on_cs2_game_event_body
            Ok(())
        }
    });

    macro_rules! add_if { ($cond:expr, $flag:ident) => {
        if $cond { interests = quote!(#interests | ::source2_demo::Interests::$flag); }
    }}

    if mode_all {
        interests = quote!(::source2_demo::Interests::all());
    }

    add_if!(has_demo, DEMO);
    add_if!(has_net, NET);
    add_if!(has_svc, SVC);
    add_if!(has_base_um, BASE_UM);
    add_if!(has_base_ge, BASE_GE);
    add_if!(has_tick_start, TICK_START);
    add_if!(has_tick_end, TICK_END);
    add_if!(has_entity, ENABLE_ENTITY);
    add_if!(has_entity_track, TRACK_ENTITY);
    add_if!(has_string_table, ENABLE_STRINGTAB);
    add_if!(has_string_table_track, TRACK_STRINGTAB);
    add_if!(has_stop, STOP);

    #[cfg(feature = "dota")]
    add_if!(has_dota_um, DOTA_UM);
    #[cfg(feature = "dota")]
    add_if!(has_combat_log, COMBAT_LOG);

    #[cfg(feature = "citadel")]
    add_if!(has_cita_um, CITA_UM);
    #[cfg(feature = "citadel")]
    add_if!(has_cita_ge, CITA_GE);

    #[cfg(feature = "cs2")]
    add_if!(has_cs2_um, CS2_UM);
    #[cfg(feature = "cs2")]
    add_if!(has_cs2_ge, CS2_GE);

    let ret = quote! {
        impl Observer for #struct_name {
            fn interests(&self) -> ::source2_demo::Interests { #interests }
            #obs_body
        }
        #input
    };

    TokenStream::from(ret)
}

fn get_arg_type(method: &syn::ImplIte1mFn, n: usize) -> (Type, bool) {
    if let Some(FnArg::Typed(pat_type)) = method.sig.inputs.iter().nth(n) {
        if let Type::Reference(x) = pat_type.ty.as_ref() {
            (*x.elem.clone(), true)
        } else {
            (*pat_type.ty.clone(), false)
        }
    } else {
        panic!("Expected argument")
    }
}

/// Marks a method as a protobuf message handler.
///
/// Use this to handle specific protobuf message types. The message type is inferred
/// from the method's parameter type, which should be a protobuf message struct.
///
/// The method will automatically decode binary message data and call your handler
/// with the decoded message object.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Access to current replay state
/// - Message parameter - The decoded protobuf message (inferred from parameter type)
/// - Message can be taken by value or reference
///
/// # Supported Message Types
///
/// - `CDotaUserMsgChatMessage` and other Dota 2 messages
/// - `CCitadelUserMsgChatMsg` and other Deadlock messages
/// - `CCSUserMessage_*` and other CS2 messages
/// - Any protobuf message type with a `decode` method
///
/// # Examples
///
/// ## Handle Dota 2 chat messages
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_message]
/// fn on_chat(&mut self, ctx: &Context, msg: CDotaUserMsgChatMessage) -> ObserverResult {
///     println!("[{}] {}", ctx.tick(), msg.message_text());
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Handle without context
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_message]
/// fn on_chat(&mut self, msg: CDotaUserMsgChatMessage) -> ObserverResult {
///     println!("Message: {}", msg.message_text());
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Handle by reference
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_message]
/// fn on_chat(&mut self, msg: &CDotaUserMsgChatMessage) -> ObserverResult {
///     println!("Message: {}", msg.message_text());
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a tick-start handler.
///
/// This handler is called at the beginning of each game tick. Use it for
/// per-tick logic like updating game state, tracking changes, or generating output.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Current replay state including tick number
///
/// # When It's Called
///
/// Called right after the tick number is incremented in the Context, but before
/// any other tick-specific events are processed.
///
/// # Examples
///
/// ## Track entity state every tick
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_tick_start]
/// fn on_tick_start(&mut self, ctx: &Context) -> ObserverResult {
///     if ctx.tick() % 30 == 0 {  // Every second (30 ticks/sec)
///         println!("Ticks processed: {}", ctx.tick());
///     }
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Without context
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_tick_start]
/// fn on_tick_start(&mut self) -> ObserverResult {
///     // Do something without needing context
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_tick_start(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a tick-end handler.
///
/// This handler is called at the end of each game tick. Use it to finalize
/// per-tick calculations, output results, or reset temporary state.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Current replay state
///
/// # When It's Called
///
/// Called after all events for the current tick have been processed.
///
/// # Examples
///
/// ## Flush buffered data at end of tick
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs {
/// #     buffer: Vec<String>,
/// # }
/// # impl MyObs {
/// #[on_tick_end]
/// fn on_tick_end(&mut self, ctx: &Context) -> ObserverResult {
///     // Output buffered data
///     for item in self.buffer.drain(..) {
///         println!("[{}] {}", ctx.tick(), item);
///     }
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_tick_end(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as an entity event handler.
///
/// This handler is called when entities are created, updated, or deleted.
/// Optionally filter to specific entity class names.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Current replay state
/// - `event: EntityEvents` (optional) - Type of entity event (Created, Updated, Deleted)
/// - `entity: &Entity` - The entity that changed
///
/// # Filtering
///
/// Pass a class name to only handle entities of that type:
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_entity("CDOTA_Unit_Hero_Axe")]
/// fn on_axe(&mut self, entity: &Entity) -> ObserverResult {
///     // Only called for Axe hero entity
///     Ok(())
/// }
/// # }
/// ```
///
/// # Examples
///
/// ## Track all entity changes
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs {
/// #     created: usize,
/// # }
/// # impl MyObs {
/// #[on_entity]
/// fn on_entity(&mut self, event: EntityEvents, entity: &Entity) -> ObserverResult {
///     if event == EntityEvents::Created {
///         self.created += 1;
///     }
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Track specific entity type
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_entity("CDOTA_PlayerResource")]
/// fn on_player_resource(&mut self, ctx: &Context, entity: &Entity) -> ObserverResult {
///     // Only called when player resource entity updates
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a game event handler.
///
/// This handler is called when game events occur (kills, deaths, item purchases, etc.).
/// Optionally filter to specific event names.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Current replay state
/// - `ge: &GameEvent` - The game event
///
/// # Filtering
///
/// Pass an event name to only handle that event type:
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_game_event("player_death")]
/// fn on_death(&mut self, ge: &GameEvent) -> ObserverResult {
///     // Only called for player_death events
///     Ok(())
/// }
/// # }
/// ```
///
/// # Examples
///
/// ## Handle all game events
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_game_event]
/// fn on_event(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
///     println!("[{}] Event: {}", ctx.tick(), ge.name());
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Handle specific event type
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs {
/// #     kill_count: u32,
/// # }
/// # impl MyObs {
/// #[on_game_event("dota_player_kill")]
/// fn on_kill(&mut self, ge: &GameEvent) -> ObserverResult {
///     self.kill_count += 1;
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_game_event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a string table update handler.
///
/// This handler is called when string tables are updated. String tables contain
/// game data like player names, modifiers, effects, etc.
/// Optionally filter to specific table names.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Current replay state
/// - `table: &StringTable` - The updated string table
/// - `modified: &[i32]` - Indices of rows that were modified
///
/// # Filtering
///
/// Pass a table name to only handle that table:
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_string_table("userinfo")]
/// fn on_userinfo(&mut self, table: &StringTable, modified: &[i32]) -> ObserverResult {
///     // Only called when userinfo table updates
///     Ok(())
/// }
/// # }
/// ```
///
/// # Examples
///
/// ## Track all string table updates
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_string_table]
/// fn on_table_update(&mut self, ctx: &Context, table: &StringTable, modified: &[i32]) -> ObserverResult {
///     println!("[{}] Table {} updated: {} rows", ctx.tick(), table.name(), modified.len());
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Monitor specific table
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_string_table("ActiveModifiers")]
/// fn on_modifiers(&mut self, table: &StringTable, modified: &[i32]) -> ObserverResult {
///     println!("Active modifiers changed: {} rows", modified.len());
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_string_table(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a replay stop handler.
///
/// This handler is called when the replay ends (CDemoStop message).
/// Use it to finalize results, output statistics, or clean up resources.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Final replay state
///
/// # Examples
///
/// ## Output final statistics
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs {
/// #     ticks: u32,
/// # }
/// # impl MyObs {
/// #[on_stop]
/// fn on_stop(&mut self, ctx: &Context) -> ObserverResult {
///     println!("Replay ended at tick {}", ctx.tick());
///     println!("Total ticks processed: {}", self.ticks);
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Without context
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # struct MyObs;
/// # impl MyObs {
/// #[on_stop]
/// fn on_stop(&mut self) -> ObserverResult {
///     println!("Replay complete");
///     Ok(())
/// }
/// # }
/// ```
#[proc_macro_attribute]
pub fn on_stop(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a method as a combat log handler (Dota 2 only).
///
/// This handler is called whenever a combat log entry is generated.
/// Combat log entries include damage, healing, kills, abilities, items, etc.
///
/// # Parameters
///
/// The handler can receive:
/// - `ctx: &Context` (optional) - Current replay state
/// - `cle: &CombatLogEntry` - The combat log entry
///
/// # Requires Feature
///
/// Only available when the `dota` feature is enabled.
///
/// # Examples
///
/// ## Track damage in real-time
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # use source2_demo::proto::DotaCombatlogTypes;
/// # struct MyObs {
/// #     total_damage: u32,
/// # }
/// # impl MyObs {
/// #[on_combat_log]
/// fn on_damage(&mut self, ctx: &Context, cle: &CombatLogEntry) -> ObserverResult {
///     if cle.r#type() == DotaCombatlogTypes::DotaCombatlogDamage {
///         if let Ok(damage) = cle.value() {
///             self.total_damage += damage;
///         }
///     }
///     Ok(())
/// }
/// # }
/// ```
///
/// ## Track kills
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// # use source2_demo::proto::DotaCombatlogTypes;
/// # struct MyObs {
/// #     kills: u32,
/// # }
/// # impl MyObs {
/// #[on_combat_log]
/// fn on_kill(&mut self, cle: &CombatLogEntry) -> ObserverResult {
///     if cle.r#type() == DotaCombatlogTypes::DotaCombatlogDeath {
///         self.kills += 1;
///     }
///     Ok(())
/// }
/// # }
/// ```
#[cfg(feature = "dota")]
#[proc_macro_attribute]
pub fn on_combat_log(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks the impl block to enable entity tracking.
///
/// When applied to an impl block or individual method, automatically enables
/// the `ENABLE_ENTITY` interest flag so entities are tracked during parsing.
///
/// # Examples
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[observer]
/// #[uses_entities]
/// impl MyObs {
///     // Now you can use #[on_entity] handlers
/// # fn dummy(&mut self) -> ObserverResult { Ok(()) }
/// }
/// ```
#[proc_macro_attribute]
pub fn uses_entities(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks the impl block to enable string table tracking.
///
/// When applied to an impl block or individual method, automatically enables
/// the `ENABLE_STRINGTAB` interest flag so string tables are tracked during parsing.
///
/// # Examples
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[observer]
/// #[uses_string_tables]
/// impl MyObs {
///     // Now you can use #[on_string_table] handlers
/// # fn dummy(&mut self) -> ObserverResult { Ok(()) }
/// }
/// ```
#[proc_macro_attribute]
pub fn uses_string_tables(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks the impl block to enable game event tracking.
///
/// When applied to an impl block or individual method, automatically enables
/// the `BASE_GE` interest flag so game events are tracked during parsing.
///
/// # Examples
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[observer]
/// #[uses_game_events]
/// impl MyObs {
///     // Now you can use #[on_game_event] handlers
/// # fn dummy(&mut self) -> ObserverResult { Ok(()) }
/// }
/// ```
#[proc_macro_attribute]
pub fn uses_game_events(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks the impl block to enable combat log tracking (Dota 2 only).
///
/// When applied to an impl block, automatically enables the `COMBAT_LOG` and
/// `ENABLE_STRINGTAB` interest flags for combat log parsing.
///
/// # Requires Feature
///
/// Only available when the `dota` feature is enabled.
///
/// # Examples
///
/// ```no_run
/// # use source2_demo::prelude::*;
/// #[observer]
/// #[uses_combat_log]
/// impl MyObs {
///     // Now you can use #[on_combat_log] handlers
/// # fn dummy(&mut self) -> ObserverResult { Ok(()) }
/// }
/// ```
#[cfg(feature = "dota")]
#[proc_macro_attribute]
pub fn uses_combat_log(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
