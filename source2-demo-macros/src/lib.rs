#![doc = include_str!("../README.md")]
//!
//! # Procedural Macros for Source 2 Replay Parser
//!
//! This crate provides procedural macros that simplify implementing the `Observer`
//! and `DemoRewriter` traits.
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
//! ### `#[rewriter]` - Demo writer implementation macro
//!
//! Automatically implements the `DemoRewriter` trait. Packet message handlers can
//! either receive the raw packet payload or a decoded protobuf message. The
//! protobuf form infers the packet id from the message type, just like
//! `#[on_message]`.
//!
//! ```no_run
//! # use source2_demo::error::ParserError;
//! # use source2_demo::prelude::*;
//! # use source2_demo::proto::CDotaUserMsgChatMessage;
//! # use source2_demo::writer::*;
//! struct RemoveChat;
//!
//! #[rewriter]
//! impl RemoveChat {
//!     #[rewrite_packet_message]
//!     fn remove_chat(
//!         &mut self,
//!         _message: CDotaUserMsgChatMessage,
//!     ) -> Result<MessageRewrite, ParserError> {
//!         Ok(MessageRewrite::Drop)
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
use syn::{parse::Parser, parse_macro_input, punctuated::Punctuated, Expr, FnArg, Ident, ItemImpl, Lit, Token, Type};

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
    let mut errors = quote!();

    if !attr.is_empty() {
        let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
        match parser.parse(attr.clone()) {
            Ok(idents) => {
                for id in idents {
                    if id == "manual" {
                        mode_all = false;
                    } else if id == "all" {
                        mode_all = true;
                    } else {
                        errors.extend(syn::Error::new_spanned(id, "expected `manual` or `all`").to_compile_error());
                    }
                }
            }
            Err(error) => {
                errors.extend(error.to_compile_error());
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

                if let Some((arg_type, _)) = get_arg_type(method, 1) {
                    if arg_type.to_token_stream().to_string() == "Context" {
                        args.push(quote! { ctx })
                    }
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

                            if let Some((arg_type, is_ref)) = get_arg_type(method, args.len() + 1) {
                                if arg_type.to_token_stream().to_string() == "EntityEvents" {
                                    if is_ref {
                                        args.push(quote! { &event });
                                    } else {
                                        args.push(quote! { event });
                                    }
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
                            let Some((arg_type, is_ref)) = get_arg_type(method, args.len() + 1) else {
                                errors.extend(syn::Error::new_spanned(&method.sig, "#[on_message] needs a protobuf message argument").to_compile_error());
                                continue;
                            };
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

                            let known_message_root = match root {
                                "EDemoCommands" => {
                                    has_demo = true;
                                    true
                                }
                                "EBaseUserMessages" => {
                                    has_base_um = true;
                                    true
                                }
                                "EBaseGameEvents" => {
                                    has_base_ge = true;
                                    true
                                }
                                "SvcMessages" => {
                                    has_svc = true;
                                    true
                                }
                                "NetMessages" => {
                                    has_net = true;
                                    true
                                }
                                #[cfg(feature = "dota")]
                                "EDotaUserMessages" => {
                                    has_dota_um = true;
                                    true
                                }
                                #[cfg(feature = "citadel")]
                                "CitadelUserMessageIds" => {
                                    has_cita_um = true;
                                    true
                                }
                                #[cfg(feature = "citadel")]
                                "ECitadelGameEvents" => {
                                    has_cita_ge = true;
                                    true
                                }
                                #[cfg(feature = "cs2")]
                                "ECstrike15UserMessages" => {
                                    has_cs2_um = true;
                                    true
                                }
                                #[cfg(feature = "cs2")]
                                "ECsgoGameEvents" => {
                                    has_cs2_ge = true;
                                    true
                                }
                                _ => false,
                            };
                            if !known_message_root {
                                errors.extend(syn::Error::new_spanned(&arg_type, "unknown #[on_message] protobuf type; use a generated Source 2 protobuf message type").to_compile_error());
                                continue;
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

                                _ => {}
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
        #errors
    };

    TokenStream::from(ret)
}

/// Implements the `DemoRewriter` trait for your struct.
///
/// Apply this to an inherent `impl` block and mark methods with writer callback
/// attributes such as `#[rewrite_packet_message]`,
/// `#[rewrite_string_table_entry]`, `#[rewrite_field]`,
/// `#[replace_entity_field]`, and
/// `#[should_rewrite_entity]`.
///
/// Handler methods should use the same return type as the corresponding
/// `DemoRewriter` callback.
#[proc_macro_attribute]
pub fn rewriter(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let struct_name = &input.self_ty;

    let mut interests = quote!(::source2_demo::writer::RewriteInterests::empty());
    macro_rules! add_flag {
        ($flag:ident) => {
            interests = quote!(#interests | ::source2_demo::writer::RewriteInterests::$flag);
        };
    }

    let mut rewrite_demo_message_body = quote!();
    let mut rewrite_packet_message_body = quote!();
    let mut rewrite_packet_messages_body = quote!();
    let mut rewrite_demo_string_tables_body = quote!();
    let mut rewrite_string_table_entry_body = quote!();
    let mut rewrite_svc_create_string_table_body = quote!();
    let mut rewrite_svc_update_string_table_body = quote!();
    let mut replace_entity_field_body = quote!();
    let mut should_rewrite_entity_body = quote!();

    for item in &input.items {
        let syn::ImplItem::Fn(method) = item else {
            continue;
        };

        let method_name = method.sig.ident.clone();
        for attr in &method.attrs {
            let Some(ident) = attr.path().get_ident() else {
                continue;
            };

            match ident.to_string().as_str() {
                "rewrite_demo_message" => {
                    add_flag!(DEMO_MESSAGE);
                    let args = writer_callback_method_args(method, quote! {});
                    rewrite_demo_message_body.extend(quote! {
                        match self.#method_name(#args)? {
                            ::source2_demo::writer::MessageRewrite::Keep => {}
                            rewrite => return Ok(rewrite),
                        }
                    });
                }
                "rewrite_packet_message" => {
                    add_flag!(PACKET_MESSAGE);
                    if packet_message_method_is_raw(method) {
                        let args = writer_callback_method_args(method, quote! {});
                        rewrite_packet_message_body.extend(quote! {
                            match self.#method_name(#args)? {
                                ::source2_demo::writer::MessageRewrite::Keep => {}
                                rewrite => return Ok(rewrite),
                            }
                        });
                    } else {
                        match packet_message_method_type(method) {
                            Ok((arg_type, is_ref, is_mut_ref)) => {
                                extend_rewrite_packet_message_body(&mut rewrite_packet_message_body, method, &method_name, &arg_type, is_ref, is_mut_ref);
                            }
                            Err(error) => {
                                rewrite_packet_message_body.extend(error.to_compile_error());
                            }
                        }
                    }
                }
                "rewrite_packet_messages" => {
                    add_flag!(PACKET_MESSAGES);
                    let args = writer_callback_method_args(method, quote! {});
                    rewrite_packet_messages_body.extend(quote! {
                        self.#method_name(#args)?;
                    });
                }
                "rewrite_demo_string_tables" => {
                    add_flag!(DEMO_STRING_TABLES);
                    let args = writer_callback_method_args(method, quote! { message });
                    rewrite_demo_string_tables_body.extend(quote! {
                        match self.#method_name(#args)? {
                            ::source2_demo::writer::MessageRewrite::Keep => {}
                            rewrite => return Ok(rewrite),
                        }
                    });
                }
                "rewrite_string_table_entry" => {
                    add_flag!(STRING_TABLE_ENTRIES);
                    let args = writer_callback_method_args(method, quote! {});
                    rewrite_string_table_entry_body.extend(quote! {
                        self.#method_name(#args)?;
                    });
                }
                "rewrite_svc_create_string_table" => {
                    add_flag!(SVC_CREATE_STRING_TABLE);
                    let args = writer_callback_method_args(method, quote! { message });
                    rewrite_svc_create_string_table_body.extend(quote! {
                        match self.#method_name(#args)? {
                            ::source2_demo::writer::MessageRewrite::Keep => {}
                            rewrite => return Ok(rewrite),
                        }
                    });
                }
                "rewrite_svc_update_string_table" => {
                    add_flag!(SVC_UPDATE_STRING_TABLE);
                    let args = writer_callback_method_args(method, quote! { message });
                    rewrite_svc_update_string_table_body.extend(quote! {
                        match self.#method_name(#args)? {
                            ::source2_demo::writer::MessageRewrite::Keep => {}
                            rewrite => return Ok(rewrite),
                        }
                    });
                }
                "replace_entity_field" => {
                    add_flag!(ENTITY_FIELDS);
                    let args = replace_entity_field_method_args(method);
                    replace_entity_field_body.extend(quote! {
                        if let Some(value) = self.#method_name(#args) {
                            return Some(value);
                        }
                    });
                }
                "rewrite_field" => {
                    add_flag!(ENTITY_FIELDS);
                    match rewrite_field_body(attr, method) {
                        Ok(tokens) => replace_entity_field_body.extend(tokens),
                        Err(error) => replace_entity_field_body.extend(error.to_compile_error()),
                    }
                }
                "should_rewrite_entity" => {
                    add_flag!(ENTITY_FIELDS);
                    let args = should_rewrite_entity_method_args(method);
                    should_rewrite_entity_body.extend(quote! {
                        if !self.#method_name(#args) {
                            return false;
                        }
                    });
                }
                _ => {}
            }
        }
    }

    let ret = quote! {
        impl ::source2_demo::writer::DemoRewriter for #struct_name {
            fn interests(&self) -> ::source2_demo::writer::RewriteInterests {
                #interests
            }

            fn rewrite_demo_message(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                msg_type: ::source2_demo::proto::EDemoCommands,
                payload: &[u8],
            ) -> Result<::source2_demo::writer::MessageRewrite, ::source2_demo::error::ParserError> {
                #rewrite_demo_message_body
                Ok(::source2_demo::writer::MessageRewrite::Keep)
            }

            fn rewrite_packet_message(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                msg_type: i32,
                payload: &[u8],
            ) -> Result<::source2_demo::writer::MessageRewrite, ::source2_demo::error::ParserError> {
                #rewrite_packet_message_body
                Ok(::source2_demo::writer::MessageRewrite::Keep)
            }

            fn rewrite_packet_messages(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                messages: &mut Vec<::source2_demo::writer::PacketMessage>,
            ) -> Result<(), ::source2_demo::error::ParserError> {
                #rewrite_packet_messages_body
                Ok(())
            }

            fn rewrite_demo_string_tables(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                message: &mut ::source2_demo::proto::CDemoStringTables,
            ) -> Result<::source2_demo::writer::MessageRewrite, ::source2_demo::error::ParserError> {
                #rewrite_demo_string_tables_body
                Ok(::source2_demo::writer::MessageRewrite::Keep)
            }

            fn rewrite_string_table_entry(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                table_name: &str,
                entry: &mut ::source2_demo::writer::StringTableEntryUpdate,
            ) -> Result<(), ::source2_demo::error::ParserError> {
                #rewrite_string_table_entry_body
                Ok(())
            }

            fn rewrite_svc_create_string_table(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                message: &mut ::source2_demo::proto::CSvcMsgCreateStringTable,
            ) -> Result<::source2_demo::writer::MessageRewrite, ::source2_demo::error::ParserError> {
                #rewrite_svc_create_string_table_body
                Ok(::source2_demo::writer::MessageRewrite::Keep)
            }

            fn rewrite_svc_update_string_table(
                &mut self,
                ctx: &::source2_demo::Context,
                tick: u32,
                message: &mut ::source2_demo::proto::CSvcMsgUpdateStringTable,
            ) -> Result<::source2_demo::writer::MessageRewrite, ::source2_demo::error::ParserError> {
                #rewrite_svc_update_string_table_body
                Ok(::source2_demo::writer::MessageRewrite::Keep)
            }

            fn replace_entity_field(
                &mut self,
                ctx: &::source2_demo::Context,
                event: ::source2_demo::EntityEvents,
                entity: &::source2_demo::Entity,
                field_name: &str,
                value: &::source2_demo::FieldValue,
            ) -> Option<::source2_demo::FieldValue> {
                #replace_entity_field_body
                None
            }

            fn should_rewrite_entity(
                &mut self,
                ctx: &::source2_demo::Context,
                event: ::source2_demo::EntityEvents,
                entity: &::source2_demo::Entity,
            ) -> bool {
                #should_rewrite_entity_body
                true
            }

        }
        #input
    };

    TokenStream::from(ret)
}

fn get_arg_type(method: &syn::ImplItemFn, n: usize) -> Option<(Type, bool)> {
    if let Some(FnArg::Typed(pat_type)) = method.sig.inputs.iter().nth(n) {
        if let Type::Reference(x) = pat_type.ty.as_ref() {
            Some((*x.elem.clone(), true))
        } else {
            Some((*pat_type.ty.clone(), false))
        }
    } else {
        None
    }
}

fn rewrite_field_body(attr: &syn::Attribute, method: &syn::ImplItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let method_name = &method.sig.ident;
    let mut class = None;
    let mut field = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("class") {
            let value = meta.value()?;
            class = Some(value.parse::<Expr>()?);
            Ok(())
        } else if meta.path.is_ident("field") {
            let value = meta.value()?;
            field = Some(value.parse::<Expr>()?);
            Ok(())
        } else {
            Err(meta.error("expected `class = ...` or `field = ...`"))
        }
    })?;

    let class = class.ok_or_else(|| syn::Error::new_spanned(attr, "missing `class = ...`"))?;
    let class_predicate = string_predicate(&quote! { entity.class().name() }, &class)?;
    let class_only = field.is_none();
    let field_predicate = match field {
        Some(field) => string_predicate(&quote! { field_name }, &field)?,
        None => quote! { true },
    };
    if class_only {
        ensure_rewrite_field_raw_value(method)?;
    }
    let value_predicate = rewrite_field_value_predicate(method)?;
    let args = rewrite_field_method_args(method)?;

    Ok(quote! {
        if (#class_predicate) && (#field_predicate) && (#value_predicate) {
            let replacement = self.#method_name(#args);
            if let Some(value) = ::source2_demo::FieldRewriteResult::into_field_rewrite_result(replacement) {
                return Some(value);
            }
        }
    })
}

fn string_predicate(target: &proc_macro2::TokenStream, expr: &Expr) -> syn::Result<proc_macro2::TokenStream> {
    match expr {
        Expr::Lit(expr_lit) => {
            let Lit::Str(value) = &expr_lit.lit else {
                return Err(syn::Error::new_spanned(expr, "expected string literal"));
            };
            Ok(quote! { #target == #value })
        }
        Expr::Call(call) => {
            let Expr::Path(path) = call.func.as_ref() else {
                return Err(syn::Error::new_spanned(&call.func, "expected predicate name"));
            };
            let Some(ident) = path.path.get_ident() else {
                return Err(syn::Error::new_spanned(&path.path, "expected predicate name"));
            };
            let name = ident.to_string();
            match name.as_str() {
                "exact" => {
                    let value = single_string_arg(expr, &call.args)?;
                    Ok(quote! { #target == #value })
                }
                "starts_with" => {
                    let value = single_string_arg(expr, &call.args)?;
                    Ok(quote! { #target.starts_with(#value) })
                }
                "ends_with" => {
                    let value = single_string_arg(expr, &call.args)?;
                    Ok(quote! { #target.ends_with(#value) })
                }
                "contains" => {
                    let value = single_string_arg(expr, &call.args)?;
                    Ok(quote! { #target.contains(#value) })
                }
                "any" => {
                    if call.args.is_empty() {
                        return Err(syn::Error::new_spanned(expr, "`any` needs at least one argument"));
                    }
                    let predicates = call.args.iter().map(|arg| string_predicate(target, arg)).collect::<syn::Result<Vec<_>>>()?;
                    Ok(quote! { false #( || (#predicates) )* })
                }
                "all" => {
                    if call.args.is_empty() {
                        return Err(syn::Error::new_spanned(expr, "`all` needs at least one argument"));
                    }
                    let predicates = call.args.iter().map(|arg| string_predicate(target, arg)).collect::<syn::Result<Vec<_>>>()?;
                    Ok(quote! { true #( && (#predicates) )* })
                }
                "not" => {
                    if call.args.len() != 1 {
                        return Err(syn::Error::new_spanned(expr, "`not` needs exactly one argument"));
                    }
                    let Some(arg) = call.args.first() else {
                        return Err(syn::Error::new_spanned(expr, "`not` needs exactly one argument"));
                    };
                    let predicate = string_predicate(target, arg)?;
                    Ok(quote! { !(#predicate) })
                }
                _ => Err(syn::Error::new_spanned(ident, "unknown predicate; expected exact, starts_with, ends_with, contains, any, all, or not")),
            }
        }
        _ => Err(syn::Error::new_spanned(expr, "expected string literal or predicate call")),
    }
}

fn single_string_arg<'a>(expr: &Expr, args: &'a Punctuated<Expr, Token![,]>) -> syn::Result<&'a syn::LitStr> {
    if args.len() != 1 {
        return Err(syn::Error::new_spanned(expr, "predicate needs exactly one argument"));
    }
    let Some(arg) = args.first() else {
        return Err(syn::Error::new_spanned(expr, "predicate needs exactly one argument"));
    };
    let Expr::Lit(expr_lit) = arg else {
        return Err(syn::Error::new_spanned(expr, "predicate argument must be a string literal"));
    };
    let Lit::Str(value) = &expr_lit.lit else {
        return Err(syn::Error::new_spanned(expr, "predicate argument must be a string literal"));
    };
    Ok(value)
}

fn stringify_type(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

fn is_context_type(value: &str) -> bool {
    matches!(value, "& :: source2_demo :: Context" | "& source2_demo :: Context" | "& Context")
}

fn is_entity_type(value: &str) -> bool {
    matches!(value, "& :: source2_demo :: Entity" | "& source2_demo :: Entity" | "& Entity")
}

fn is_entity_events_type(value: &str) -> bool {
    matches!(value, ":: source2_demo :: EntityEvents" | "source2_demo :: EntityEvents" | "EntityEvents")
}

fn is_field_value_ref_type(value: &str) -> bool {
    matches!(value, "& :: source2_demo :: FieldValue" | "& source2_demo :: FieldValue" | "& FieldValue")
}

fn is_field_value_type(value: &str) -> bool {
    matches!(value, ":: source2_demo :: FieldValue" | "source2_demo :: FieldValue" | "FieldValue")
}

fn is_raw_field_value_type(value: &str) -> bool {
    is_field_value_ref_type(value) || is_field_value_type(value)
}

fn is_demo_command_type(value: &str) -> bool {
    matches!(value, ":: source2_demo :: proto :: EDemoCommands" | "source2_demo :: proto :: EDemoCommands" | "EDemoCommands")
}

fn is_packet_messages_type(value: &str) -> bool {
    matches!(value, "& mut Vec < :: source2_demo :: writer :: PacketMessage >" | "& mut Vec < source2_demo :: writer :: PacketMessage >" | "& mut Vec < PacketMessage >")
}

fn is_string_table_entry_type(value: &str) -> bool {
    matches!(value, "& mut :: source2_demo :: writer :: StringTableEntryUpdate" | "& mut source2_demo :: writer :: StringTableEntryUpdate" | "& mut StringTableEntryUpdate")
}

fn is_rewrite_field_value_type(value: &str) -> bool {
    matches!(value, "& str" | "String" | "& String" | "bool" | "f32" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "[f32 ; 2]" | "[f32 ; 3]" | "[f32 ; 4]") || is_raw_field_value_type(value)
}

fn rewrite_field_method_args(method: &syn::ImplItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let value_arg_index = rewrite_field_value_arg_index(method)?;
    let mut args = Vec::new();

    for (index, input) in method.sig.inputs.iter().enumerate().skip(1) {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };
        if index == value_arg_index {
            args.push(rewrite_field_value_arg(pat_type.ty.as_ref())?);
            continue;
        }

        let type_string = stringify_type(pat_type.ty.as_ref());
        let arg = if is_entity_events_type(&type_string) {
            quote! { event }
        } else if is_context_type(&type_string) {
            quote! { ctx }
        } else if is_entity_type(&type_string) {
            quote! { entity }
        } else if type_string == "& str" {
            quote! { field_name }
        } else if is_field_value_ref_type(&type_string) {
            quote! { value }
        } else {
            return Err(syn::Error::new_spanned(pat_type, "unsupported #[rewrite_field] argument"));
        };
        args.push(arg);
    }

    Ok(quote! { #(#args),* })
}

fn replace_entity_field_method_args(method: &syn::ImplItemFn) -> proc_macro2::TokenStream {
    let mut args = Vec::new();

    for input in method.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };
        let type_string = stringify_type(pat_type.ty.as_ref());
        let arg = if is_context_type(&type_string) {
            quote! { ctx }
        } else if is_entity_events_type(&type_string) {
            quote! { event }
        } else if is_entity_type(&type_string) {
            quote! { entity }
        } else if type_string == "& str" {
            quote! { field_name }
        } else if is_field_value_ref_type(&type_string) {
            quote! { value }
        } else {
            quote! { compile_error!("unsupported #[replace_entity_field] argument") }
        };
        args.push(arg);
    }

    quote! { #(#args),* }
}

fn should_rewrite_entity_method_args(method: &syn::ImplItemFn) -> proc_macro2::TokenStream {
    let mut args = Vec::new();

    for input in method.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };
        let type_string = stringify_type(pat_type.ty.as_ref());
        let arg = if is_context_type(&type_string) {
            quote! { ctx }
        } else if is_entity_events_type(&type_string) {
            quote! { event }
        } else if is_entity_type(&type_string) {
            quote! { entity }
        } else {
            quote! { compile_error!("unsupported #[should_rewrite_entity] argument") }
        };
        args.push(arg);
    }

    quote! { #(#args),* }
}

fn rewrite_field_value_arg_index(method: &syn::ImplItemFn) -> syn::Result<usize> {
    for (index, input) in method.sig.inputs.iter().enumerate().skip(1).rev() {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };
        let type_string = stringify_type(pat_type.ty.as_ref());
        if is_rewrite_field_value_type(&type_string) {
            return Ok(index);
        }
    }

    Err(syn::Error::new_spanned(&method.sig, "#[rewrite_field] needs a value argument"))
}

fn ensure_rewrite_field_raw_value(method: &syn::ImplItemFn) -> syn::Result<()> {
    let value_arg_index = rewrite_field_value_arg_index(method)?;
    let Some(FnArg::Typed(pat_type)) = method.sig.inputs.iter().nth(value_arg_index) else {
        return Ok(());
    };
    let type_string = stringify_type(pat_type.ty.as_ref());
    if is_raw_field_value_type(&type_string) {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(pat_type, "class-only #[rewrite_field] handlers must use FieldValue"))
    }
}

fn rewrite_field_value_arg(ty: &Type) -> syn::Result<proc_macro2::TokenStream> {
    let type_string = stringify_type(ty);
    match type_string.as_str() {
        "& str" => Ok(quote! { <&::source2_demo::FieldValue as ::std::convert::TryInto<String>>::try_into(value).ok()?.as_str() }),
        "String" => Ok(quote! { <&::source2_demo::FieldValue as ::std::convert::TryInto<String>>::try_into(value).ok()? }),
        "& String" => Ok(quote! { &<&::source2_demo::FieldValue as ::std::convert::TryInto<String>>::try_into(value).ok()? }),
        value if is_field_value_ref_type(value) => Ok(quote! { value }),
        value if is_field_value_type(value) => Ok(quote! { value.clone() }),
        _ => Ok(quote! { <&::source2_demo::FieldValue as ::std::convert::TryInto<#ty>>::try_into(value).ok()? }),
    }
}

fn rewrite_field_value_predicate(method: &syn::ImplItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let value_arg_index = rewrite_field_value_arg_index(method)?;
    let Some(FnArg::Typed(pat_type)) = method.sig.inputs.iter().nth(value_arg_index) else {
        return Ok(quote! { true });
    };
    let ty = pat_type.ty.as_ref();
    let type_string = stringify_type(ty);
    match type_string.as_str() {
        value if is_raw_field_value_type(value) => Ok(quote! { true }),
        "& str" | "String" | "& String" => Ok(quote! {
            <&::source2_demo::FieldValue as ::std::convert::TryInto<String>>::try_into(value).is_ok()
        }),
        _ => Ok(quote! {
            <&::source2_demo::FieldValue as ::std::convert::TryInto<#ty>>::try_into(value).is_ok()
        }),
    }
}

fn writer_callback_method_args(method: &syn::ImplItemFn, message_arg: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let mut args = Vec::new();

    for input in method.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };
        let type_string = stringify_type(pat_type.ty.as_ref());
        let arg = writer_callback_arg(&type_string, message_arg.clone());
        args.push(arg);
    }

    quote! { #(#args),* }
}

fn writer_callback_arg(type_string: &str, message_arg: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    if is_context_type(type_string) {
        quote! { ctx }
    } else if type_string == "u32" {
        quote! { tick }
    } else if type_string == "i32" || is_demo_command_type(type_string) {
        quote! { msg_type }
    } else if type_string == "& [u8]" {
        quote! { payload }
    } else if type_string == "& str" {
        quote! { table_name }
    } else if is_packet_messages_type(type_string) {
        quote! { messages }
    } else if is_string_table_entry_type(type_string) {
        quote! { entry }
    } else {
        message_arg
    }
}

fn packet_message_method_is_raw(method: &syn::ImplItemFn) -> bool {
    packet_message_first_payload_arg(method).is_some_and(|type_string| type_string == "i32" || type_string == "& [u8]")
}

fn packet_message_method_type(method: &syn::ImplItemFn) -> syn::Result<(Type, bool, bool)> {
    for input in method.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = input else {
            continue;
        };
        let type_string = stringify_type(pat_type.ty.as_ref());
        if is_context_type(&type_string) || type_string == "u32" {
            continue;
        }
        if let Type::Reference(reference) = pat_type.ty.as_ref() {
            return Ok((*reference.elem.clone(), true, reference.mutability.is_some()));
        }
        return Ok((*pat_type.ty.clone(), false, false));
    }
    Err(syn::Error::new_spanned(&method.sig, "#[rewrite_packet_message] needs a message argument"))
}

fn packet_message_first_payload_arg(method: &syn::ImplItemFn) -> Option<String> {
    method.sig.inputs.iter().skip(1).find_map(|input| {
        let FnArg::Typed(pat_type) = input else {
            return None;
        };
        let type_string = stringify_type(pat_type.ty.as_ref());
        if is_context_type(&type_string) || type_string == "u32" {
            None
        } else {
            Some(type_string)
        }
    })
}

fn extend_rewrite_packet_message_body(body: &mut proc_macro2::TokenStream, method: &syn::ImplItemFn, method_name: &Ident, arg_type: &Type, is_ref: bool, is_mut_ref: bool) {
    let enum_type = get_enum_from_struct(arg_type.to_token_stream().to_string().as_str());
    let message_arg = if is_ref {
        if is_mut_ref {
            quote! { &mut message }
        } else {
            quote! { &message }
        }
    } else {
        quote! { message }
    };
    let method_args = writer_callback_method_args(method, message_arg);

    if is_ref {
        body.extend(quote! {
            if msg_type == #enum_type as i32 {
                let mut message = <#arg_type as ::source2_demo::proto::Message>::decode(payload)?;
                match self.#method_name(#method_args)? {
                    ::source2_demo::writer::MessageRewrite::Keep => {}
                    ::source2_demo::writer::MessageRewrite::Rewrite => {
                        return Ok(::source2_demo::writer::MessageRewrite::Replace(
                            ::source2_demo::proto::Message::encode_to_vec(&message),
                        ));
                    }
                    rewrite => return Ok(rewrite),
                }
            }
        });
    } else {
        body.extend(quote! {
            if msg_type == #enum_type as i32 {
                let message = <#arg_type as ::source2_demo::proto::Message>::decode(payload)?;
                match self.#method_name(#method_args)? {
                    ::source2_demo::writer::MessageRewrite::Keep
                    | ::source2_demo::writer::MessageRewrite::Rewrite => {}
                    rewrite => return Ok(rewrite),
                }
            }
        });
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

/// Marks a `#[rewriter]` method as an outer demo message rewriter.
#[proc_macro_attribute]
pub fn rewrite_demo_message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as a packet message rewriter.
#[proc_macro_attribute]
pub fn rewrite_packet_message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as a packet message list rewriter.
#[proc_macro_attribute]
pub fn rewrite_packet_messages(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as a demo string table rewriter.
#[proc_macro_attribute]
pub fn rewrite_demo_string_tables(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as a string table entry rewriter.
#[proc_macro_attribute]
pub fn rewrite_string_table_entry(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as an `svc_CreateStringTable` rewriter.
#[proc_macro_attribute]
pub fn rewrite_svc_create_string_table(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as an `svc_UpdateStringTable` rewriter.
#[proc_macro_attribute]
pub fn rewrite_svc_update_string_table(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as an entity field replacer.
#[proc_macro_attribute]
pub fn replace_entity_field(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a typed `#[rewriter]` method as an entity field replacer.
///
/// Supports exact string filters and simple string predicates:
/// `exact`, `starts_with`, `ends_with`, `contains`, `any`, `all`, and `not`.
#[proc_macro_attribute]
pub fn rewrite_field(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Marks a `#[rewriter]` method as an entity rewrite filter.
#[proc_macro_attribute]
pub fn should_rewrite_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
