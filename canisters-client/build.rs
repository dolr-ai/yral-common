use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs,
    io::BufReader,
    path::PathBuf,
    sync::LazyLock,
};

use anyhow::Result;
use candid_parser::{
    candid::types::{TypeEnv, TypeInner as CandidTypeInner},
    check_prog, IDLProg, Principal,
};
use convert_case::{Case, Casing};
use serde::Deserialize;

use quote::format_ident;
use syn::{FnArg, ImplItem, Item, Pat, Visibility};

static DID_WHITELIST: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    #[allow(unused_mut)]
    let mut whitelist = HashSet::new();

    #[cfg(feature = "individual-user")]
    whitelist.insert("individual_user_template");
    #[cfg(feature = "platform-orchestrator")]
    whitelist.insert("platform_orchestrator");
    #[cfg(feature = "post-cache")]
    whitelist.insert("post_cache");
    #[cfg(feature = "user-index")]
    whitelist.insert("user_index");

    #[cfg(feature = "sns-governance")]
    whitelist.insert("sns_governance");
    #[cfg(feature = "sns-ledger")]
    whitelist.insert("sns_ledger");
    #[cfg(feature = "sns-root")]
    whitelist.insert("sns_root");
    #[cfg(feature = "sns-swap")]
    whitelist.insert("sns_swap");
    #[cfg(feature = "sns-index")]
    whitelist.insert("sns_index");

    whitelist
});

#[derive(Deserialize)]
struct CanId {
    ic: Principal,
    local: Principal,
}

fn read_candid_ids() -> Result<HashMap<String, CanId>> {
    let can_ids_file = fs::File::open("did/canister_ids.json")?;
    let reader = BufReader::new(can_ids_file);
    Ok(serde_json::from_reader(reader)?)
}

fn generate_canister_id_mod(can_ids: Vec<(String, Principal)>) -> String {
    let mut canister_id_mod = String::new();
    for (canister, can_id) in can_ids {
        let can_upper = canister.to_case(Case::UpperSnake);
        // CANISTER_NAME_ID: Principal = Principal::from_slice(&[..]);
        canister_id_mod.push_str(&format!(
            "pub const {can_upper}_ID: candid::Principal = candid::Principal::from_slice(&{:?});\n",
            can_id.as_slice()
        ));
    }
    canister_id_mod
}

fn build_canister_ids(out_dir: &str) -> Result<()> {
    let can_ids = read_candid_ids()?;
    let mut local_can_ids = Vec::<(String, Principal)>::new();
    let mut ic_can_ids = Vec::<(String, Principal)>::new();
    let whitelist = DID_WHITELIST.clone();
    for (canister, can_id) in can_ids {
        if !whitelist.contains(canister.as_str()) {
            continue;
        }

        local_can_ids.push((canister.clone(), can_id.local));
        ic_can_ids.push((canister, can_id.ic));
    }

    let local_canister_id_mod = generate_canister_id_mod(local_can_ids);
    let ic_canister_id_mod = generate_canister_id_mod(ic_can_ids);

    let canister_id_mod_contents = format!(
        r#"
    pub mod local {{
        {local_canister_id_mod}
    }}

    pub mod ic {{
        {ic_canister_id_mod}
    }}
"#
    );
    let canister_id_mod_path = PathBuf::from(out_dir).join("canister_ids.rs");
    fs::write(canister_id_mod_path, canister_id_mod_contents)?;

    Ok(())
}

fn build_did_intfs(out_dir: &str) -> Result<()> {
    println!("cargo:rerun-if-changed=./did/*");

    let mut candid_config = candid_parser::bindings::rust::Config::new();
    candid_config.set_target(candid_parser::bindings::rust::Target::Agent);
    candid_config
        .set_type_attributes("#[derive(CandidType, Deserialize, Debug, Clone, PartialEq)]".into());
    let mut did_mod_contents = String::new();
    let whitelist = DID_WHITELIST.clone();

    eprintln!("cargo:warning=Active DID Whitelist: {:?}", whitelist);
    eprintln!("cargo:warning=Reading from ./did directory");

    let did_out_path = PathBuf::from(&out_dir).join("did");
    fs::create_dir_all(&did_out_path)?;

    for did_entry in fs::read_dir("did")? {
        let did_file_info = did_entry?;
        let did_path = did_file_info.path();

        if did_path.extension() != Some(OsStr::new("did")) {
            continue;
        }

        let file_name_os_str = did_path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("File has no stem: {:?}", did_path))?;
        let file_name = file_name_os_str
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Filename not valid UTF-8: {:?}", file_name_os_str))?;

        if !whitelist.contains(file_name) {
            continue;
        }

        eprintln!("cargo:warning=Processing DID file: {}", file_name);

        let service_name_pascal = file_name.to_case(Case::Pascal);
        candid_config.set_service_name(service_name_pascal.clone());

        let did_content_str = match fs::read_to_string(&did_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!(
                    "cargo:warning=Failed to read DID file {} to string: {}. Skipping.",
                    did_path.display(),
                    e
                );
                continue;
            }
        };

        let ast: IDLProg = match did_content_str.parse() {
            Ok(parsed_ast) => parsed_ast,
            Err(e) => {
                eprintln!(
                    "cargo:warning=Failed to parse DID content from {} into AST: {}. Skipping.",
                    did_path.display(),
                    e
                );
                continue;
            }
        };

        let mut type_env = TypeEnv::new();
        let actor_type_opt = match check_prog(&mut type_env, &ast) {
            Ok(actor) => actor,
            Err(e) => {
                eprintln!(
                    "cargo:warning=Failed to type check AST from {}: {}. Skipping.",
                    did_path.display(),
                    e
                );
                continue;
            }
        };

        let actor = match &actor_type_opt {
            Some(actor_ref) => actor_ref,
            None => {
                eprintln!(
                    "cargo:warning=No actor (service definition) found after check_prog for DID file: {}. Skipping code generation for this file.",
                    did_path.display()
                );
                continue;
            }
        };

        let mut methods_to_wrap = HashSet::<String>::new();

        let mut service_methods_opt: Option<&Vec<(String, candid_parser::candid::types::Type)>> =
            None;

        match actor.as_ref() {
            CandidTypeInner::Service(service_def) => {
                service_methods_opt = Some(service_def);
            }
            CandidTypeInner::Class(_, service_constructor_type) => {
                if let CandidTypeInner::Service(service_def) = service_constructor_type.as_ref() {
                    service_methods_opt = Some(service_def);
                } else {
                    eprintln!(
                        "cargo:warning=  Class constructor type for {} is not Service: {:?}",
                        did_path.display(),
                        service_constructor_type.as_ref()
                    );
                }
            }
            other_type => {
                eprintln!("cargo:warning=Actor for {} is an unexpected type: {:?}. Cannot extract service methods.", did_path.display(), other_type);
            }
        }

        if let Some(service_def) = service_methods_opt {
            for (method_name, method_type) in service_def {
                if let CandidTypeInner::Func(_func_details) = method_type.as_ref() {
                    methods_to_wrap.insert(method_name.to_case(Case::Snake));
                }
            }
        } else {
            eprintln!("cargo:warning=Could not extract service methods for {}. methods_to_wrap will be empty.", did_path.display());
        }

        let original_bindings_str =
            candid_parser::bindings::rust::compile(&candid_config, &type_env, &actor_type_opt);

        let mut ast_code: syn::File = match syn::parse_str(&original_bindings_str) {
            Ok(parsed_ast) => parsed_ast,
            Err(e) => {
                eprintln!("cargo:warning=Failed to parse generated bindings for {}: {}. Using original bindings.", file_name, e);
                let binding_file_path = did_out_path.join(format!("{}.rs", file_name));
                if let Err(write_err) = fs::write(&binding_file_path, &original_bindings_str) {
                    eprintln!("cargo:warning=Failed to write original bindings for {} after parse error: {}", file_name, write_err);
                }
                did_mod_contents.push_str(&format!(
                    "#[path = \"{}\"] pub mod {};\n",
                    binding_file_path.to_string_lossy().replace("\\", "/"),
                    file_name
                ));
                continue;
            }
        };

        for item in &mut ast_code.items {
            if let Item::Impl(item_impl) = item {
                if let syn::Type::Path(type_path) = &*item_impl.self_ty {
                    if let Some(segment) = type_path.path.segments.first() {
                        if segment.ident != service_name_pascal {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }

                let mut new_impl_items = Vec::new();
                for impl_item_ref in &item_impl.items {
                    if let ImplItem::Fn(method_fn) = impl_item_ref {
                        let current_method_name = method_fn.sig.ident.to_string();
                        let is_async = method_fn.sig.asyncness.is_some();
                        let is_eligible_for_retry = methods_to_wrap.contains(&current_method_name);

                        if is_async && is_eligible_for_retry {
                            let mut original_method_ast = method_fn.clone();
                            let impl_method_name_ident =
                                format_ident!("{}_impl", current_method_name);
                            original_method_ast.sig.ident = impl_method_name_ident.clone();
                            original_method_ast.vis = Visibility::Inherited;

                            let wrapper_method_name_ident = method_fn.sig.ident.clone();

                            let arg_passing_code: Vec<proc_macro2::TokenStream> = method_fn
                                .sig
                                .inputs
                                .iter()
                                .filter_map(|fn_arg| match fn_arg {
                                    FnArg::Receiver(_) => None,
                                    FnArg::Typed(pat_type) => {
                                        if let Pat::Ident(pat_ident) = &*pat_type.pat {
                                            let arg_name = &pat_ident.ident;
                                            Some(quote::quote! { #arg_name.clone() })
                                        } else {
                                            let pat_tokens = quote::quote! {#pat_type.pat};
                                            Some(quote::quote! { #pat_tokens.clone()})
                                        }
                                    }
                                })
                                .collect();

                            let arg_definitions = method_fn.sig.inputs.clone();
                            let return_type = method_fn.sig.output.clone();
                            let visibility = method_fn.vis.clone();
                            let asyncness = method_fn.sig.asyncness;
                            let unsafety = method_fn.sig.unsafety;
                            let abi = method_fn.sig.abi.clone();
                            let generics = &method_fn.sig.generics;

                            let new_method_toks = quote::quote! {
                                #visibility #unsafety #asyncness #abi fn #wrapper_method_name_ident #generics (#arg_definitions) #return_type {
                                    let base_delay = ::std::time::Duration::from_millis(200);
                                    let max_retries: u32 = 5;
                                    let mut attempts: u32 = 0;

                                    loop {
                                        match self.#impl_method_name_ident(#(#arg_passing_code),*).await {
                                            Ok(res) => return Ok(res),
                                            Err(e) => {
                                                match e{
                                                    ::ic_agent::AgentError::TransportError(_) | ::ic_agent::AgentError::CertifiedReject(_) => {
                                                        attempts += 1;
                                                        if attempts > max_retries {
                                                            return Err(e);
                                                        }
                                                        let delay_multiplier = 2_u64.pow(attempts.saturating_sub(1));
                                                        let current_delay_ms = base_delay.as_millis() as u64 * delay_multiplier;
                                                        let capped_delay_ms = ::std::cmp::min(current_delay_ms, 10_000);
                                                        let actual_delay = ::std::time::Duration::from_millis(capped_delay_ms);
                                                        ::tokio::time::sleep(actual_delay).await;
                                                    },
                                                    _ => {
                                                        return Err(e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            };

                            new_impl_items.push(ImplItem::Fn(original_method_ast));
                            match syn::parse2::<ImplItem>(new_method_toks) {
                                Ok(parsed_item) => new_impl_items.push(parsed_item),
                                Err(e) => {
                                    eprintln!("cargo:warning=Failed to parse wrapped method for {} in {}: {}. Keeping original.", current_method_name, file_name, e);
                                    new_impl_items.push(ImplItem::Fn(method_fn.clone()));
                                }
                            }
                        } else {
                            new_impl_items.push(ImplItem::Fn(method_fn.clone()));
                        }
                    } else {
                        new_impl_items.push(impl_item_ref.clone());
                    }
                }
                item_impl.items = new_impl_items;
            }
        }

        let modified_code_str = quote::quote! { #ast_code }.to_string();
        let pretty_modified_code_str = match syn::parse_file(&modified_code_str) {
            Ok(file_ast) => prettyplease::unparse(&file_ast),
            Err(_) => modified_code_str,
        };

        let binding_file_path = did_out_path.join(format!("{}.rs", file_name));
        fs::write(&binding_file_path, pretty_modified_code_str)?;

        did_mod_contents.push_str(&format!(
            "#[path = \"{}\"] pub mod {};\n",
            binding_file_path.to_string_lossy().replace("\\", "/"),
            file_name
        ));
    }

    let binding_mod_file = did_out_path.join("mod.rs");
    fs::write(binding_mod_file, &did_mod_contents)?;

    Ok(())
}

fn main() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:warning=OUT_DIR is: {}", out_dir);

    build_did_intfs(&out_dir)?;
    build_canister_ids(&out_dir)?;

    Ok(())
}
