use abi_stable::{
    external_types::crossbeam_channel::RSender,
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError,
        RResult::{self, RErr, ROk},
        RString,
    },
};
use anyhow::Context;
use dynisland_abi::module::{ModuleType, SabiModule, SabiModule_TO, UIServerCommand};
use env_logger::Env;
use log::Level;

use dynisland_core::base_module::{BaseModule, ProducerRuntime};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};


use crate::NAME;



#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TemplateConfig {

}

#[allow(clippy::derivable_impls)]
impl Default for TemplateConfig {
    fn default() -> Self {
        Self {}
    }
}

pub struct MusicModule {
    base_module: BaseModule<MusicModule>,
    producers_rt: ProducerRuntime,
    config: TemplateConfig,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();
    if let Err(err) = gtk::gio::resources_register_include!("compiled.gresource") {
        return RErr(RBoxError::new(err));
    }

    let base_module = BaseModule::new(NAME, app_send.clone());
    let producers_rt = ProducerRuntime::new();

    let this = MusicModule {
        base_module,
        producers_rt,
        config: TemplateConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
    fn init(&self) {
        let base_module = self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            base_module.register_producer(self::producer);
        });

        let fallback_provider = gtk::CssProvider::new();
        let css = grass::from_string(include_str!("../default.scss"), &grass::Options::default())
            .unwrap();
        fallback_provider.load_from_string(&css);
        glib::MainContext::default().spawn_local(async move {
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &fallback_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();
        match conf.into_rust() {
            Ok(conf) => {
                self.config = conf;
            }
            Err(err) => {
                log::error!("Failed to parse config into struct: {:#?}", err);
                return RErr(RBoxError::new(err));
            }
        }
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        match ron::ser::to_string_pretty(&TemplateConfig::default(), PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
    }

    fn restart_producers(&self) {
        self.producers_rt.shutdown_blocking();
        self.producers_rt.reset_blocking();
        //restart producers
        for producer in self
            .base_module
            .registered_producers()
            .blocking_lock()
            .iter()
        {
            producer(self);
        }
    }
}

#[allow(unused_variables)]
fn producer(module: &MusicModule) {
    let config = &module.config;
    todo!("do stuff")
}