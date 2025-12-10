mod protobuf_map;

use crate::protobuf_map::get_enum_from_struct;
use proc_macro::{TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse::Parser, punctuated::Punctuated, Ident, ItemImpl, Token, Type, FnArg};

#[allow(unused_mut)]
#[proc_macro_attribute]
pub fn observer(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut mode_all = false;

    if !attr.is_empty() {
        let parser = Punctuated::<Ident, Token![,]>::parse_terminated;
        if let Ok(idents) = parser.parse(attr.clone()) {
            for id in idents {
                if id == "manual" { mode_all = false; }
                if id == "all" { mode_all = true; }
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
        if a.path().is_ident("uses_entities") { add_flag!(ENABLE_ENTITY); }
        if a.path().is_ident("uses_string_tables") { add_flag!(ENABLE_STRINGTAB); }
        if a.path().is_ident("uses_game_events") { add_flag!(BASE_GE); }
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
                    if a.path().is_ident("uses_entities") { add_flag!(ENABLE_ENTITY); }
                    if a.path().is_ident("uses_string_tables") { add_flag!(ENABLE_STRINGTAB); }
                    if a.path().is_ident("uses_game_events") { add_flag!(BASE_GE); }
                    #[cfg(feature = "dota")]
                    if a.path().is_ident("uses_combat_log") { add_flag!(DOTA_UM); }
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

    add_if!(has_demo,       DEMO);
    add_if!(has_net,        NET);
    add_if!(has_svc,        SVC);
    add_if!(has_base_um,    BASE_UM);
    add_if!(has_base_ge,    BASE_GE);
    add_if!(has_tick_start, TICK_START);
    add_if!(has_tick_end,   TICK_END);
    add_if!(has_entity,     ENABLE_ENTITY);
    add_if!(has_entity_track,     TRACK_ENTITY);
    add_if!(has_string_table,  ENABLE_STRINGTAB);
    add_if!(has_string_table_track,  TRACK_STRINGTAB);
    add_if!(has_stop,       STOP);

    #[cfg(feature = "dota")]
    add_if!(has_dota_um,    DOTA_UM);
    #[cfg(feature = "dota")]
    add_if!(has_combat_log, COMBAT_LOG);

    #[cfg(feature = "citadel")]
    add_if!(has_cita_um,    CITA_UM);
    #[cfg(feature = "citadel")]
    add_if!(has_cita_ge,    CITA_GE);

    #[cfg(feature = "cs2")]
    add_if!(has_cs2_um,     CS2_UM);
    #[cfg(feature = "cs2")]
    add_if!(has_cs2_ge,     CS2_GE);

    let ret = quote! {
        impl Observer for #struct_name {
            fn interests(&self) -> ::source2_demo::Interests { #interests }
            #obs_body
        }
        #input
    };

    TokenStream::from(ret)
}

fn get_arg_type(method: &syn::ImplItemFn, n: usize) -> (Type, bool) {
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

/// A method wrapped with `#[on_message]` macro is called whenever a specified protobuf message appears in replay.
///
/// # Examples
///
/// ```no_compile
/// #[on_message]
/// fn message(&mut self, ctx: &Context, message: &CCitadelUserMsgChatMsg) -> ObserverResult {
///     Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_message]
/// fn message(&mut self, message: CCitadelUserMsgChatMsg) -> ObserverResult {
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn on_message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_tick_start]` macro is called at the start of each tick.
///
/// # Examples
///
/// ```no_compile
/// #[on_tick_start]
/// fn tick_start(&mut self, ctx: &Context) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_tick_start]
/// fn tick_start(&mut self) -> ObserverResult {
///    Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn on_tick_start(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_tick_end]` macro is called at the end of each tick.
///
/// # Examples
///
/// ```no_compile
/// #[on_tick_end]
/// fn tick_end(&mut self, ctx: &Context) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_tick_end]
/// fn tick_end(&mut self) -> ObserverResult {
///    Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn on_tick_end(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_entity]` macro is called whenever an entity is created, updated or deleted.
///
/// # Examples
///
/// ```no_compile
/// #[on_entity]
/// fn entity(&mut self, ctx: &Context, event: EntityEvents, entity: &Entity) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_entity("CCitadelPlayerPawn")] // Will be called for entities with class "CCitadelPlayerPawn"
/// fn entity(&mut self, ctx: &Context, entity: &Entity) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_entity]
/// fn entity(&mut self, entity: &Entity) -> ObserverResult {
///    Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn on_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_game_event]` macro is called whenever CSvcMsgGameEvent appears in replay.
///
/// # Examples
///
/// ```no_compile
/// #[on_game_event] // Will be called for all game events
/// fn event(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_game_event("player_death")] // Will be called for "player_death" event only
/// fn event(&mut self, ctx: &Context, ge: &GameEvent) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_game_event]
/// fn event(&mut self, ge: &GameEvent) -> ObserverResult {
///    Ok(())
/// }
#[proc_macro_attribute]
pub fn on_game_event(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_string_table]` macro is called when string table is updated.
///
/// # Examples
///
/// ```no_compile
/// #[on_string_table] // Will be called when any string table is updated
/// fn string_table(&mut self, ctx: &Context, table: &StringTable, modified: &[i32]) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_string_table("EntityNames")] // Will be called when "EntityNames" table is updated
/// fn string_table(&mut self, ctx: &Context, table: &StringTable, modified: &[i32]) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_string_table]
/// fn string_table(&mut self, table: &StringTable, modified: &[i32]) -> ObserverResult {
///    Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn on_string_table(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_stop]` macro is called when CDemoStop appears in replay.
///
/// # Examples
///
/// ```no_compile
/// #[on_stop]
/// fn stop(&mut self, ctx: &Context) -> ObserverResult {
///    Ok(())
/// }
/// ```
///
/// ```no_compile
/// #[on_stop]
/// fn stop(&mut self) -> ObserverResult {
///    Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn on_stop(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// A method wrapped with `#[on_combat_log]` macro is called whenever CMsgDotaCombatLogEntry appears in replay.
///
/// # Examples
///
/// ```no_compile
/// #[on_combat_log]
/// fn combat_log(&mut self, ctx: &Context, cle: &CombatLogEntry) -> ObserverResult {
///    Ok(())
/// }
///
/// ```no_compile
/// #[on_combat_log]
/// fn combat_log(&mut self, cle: &CombatLogEntry) -> ObserverResult {
///    Ok(())
/// }
#[cfg(feature = "dota")]
#[proc_macro_attribute]
pub fn on_combat_log(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn uses_entities(_attr: TokenStream, item: TokenStream) -> TokenStream { item }

#[proc_macro_attribute]
pub fn uses_string_tables(_attr: TokenStream, item: TokenStream) -> TokenStream { item }

#[proc_macro_attribute]
pub fn uses_game_events(_attr: TokenStream, item: TokenStream) -> TokenStream { item }

#[cfg(feature = "dota")]
#[proc_macro_attribute]
pub fn uses_combat_log(_attr: TokenStream, item: TokenStream) -> TokenStream { item }