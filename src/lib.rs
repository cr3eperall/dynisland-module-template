use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait};
use dynisland_core::abi::module::{ModuleBuilder, ModuleBuilderRef};

mod config;
mod module;
mod widget;

use module::new;

pub const NAME: &str = "TemplateModule";

#[export_root_module]
fn instantiate_root_module() -> ModuleBuilderRef {
    ModuleBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
