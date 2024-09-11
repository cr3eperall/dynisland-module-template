use std::collections::HashMap;

use dynisland_core::{
    abi::{
        abi_stable::{
            external_types::crossbeam_channel::RSender,
            sabi_extern_fn,
            sabi_trait::TD_CanDowncast,
            std_types::{
                RBoxError,
                RResult::{self, RErr, ROk},
                RString,
            },
        },
        gdk, glib, gtk, log,
        module::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
};
use env_logger::Env;
use ron::ser::PrettyConfig;

use crate::{
    config::{DeTemplateConfigMain, TemplateConfig, TemplateConfigMain},
    NAME,
};

pub struct MusicModule {
    base_module: BaseModule<MusicModule>,
    producers_rt: ProducerRuntime,
    config: TemplateConfigMain,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(log::Level::Warn.as_str()))
        .init();
    if let Err(err) = gtk::gio::resources_register_include!("compiled.gresource") {
        return RErr(RBoxError::new(err));
    }

    let base_module = BaseModule::new(NAME, app_send.clone());
    let producers_rt = ProducerRuntime::new();
    let mut config = TemplateConfigMain::default();
    // if the module was loaded we want at least one activity
    config
        .windows
        .insert("".to_string(), vec![TemplateConfig::default()]);

    let this = MusicModule {
        base_module,
        producers_rt,
        config,
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
    // register the producers and the default css provider
    // this is called after the module is created but before gtk is initialized
    // so any code that uses gtk should be spawned on the main context
    fn init(&self) {
        self.base_module.register_producer(self::producer);

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
        log::trace!("config: {}", config);
        match serde_json::from_str::<DeTemplateConfigMain>(&config) {
            Ok(conf) => {
                let mut conf = conf.into_main_config();
                if conf.windows.is_empty() {
                    conf.windows
                        .insert("".to_string(), vec![conf.default_conf()]);
                };
                self.config = conf;
            }
            Err(err) => {
                log::error!("Failed to parse config into struct: {:#?}", err);
                return RErr(RBoxError::new(err));
            }
        }
        log::debug!("current config: {:#?}", self.config);
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        let config = TemplateConfigMain::default();
        // if the config has child_only properties we need to add a default config to the windows
        // config.windows.insert("".to_string(), vec![TemplateConfig::default()]);
        match ron::ser::to_string_pretty(&config, PrettyConfig::default()) {
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

// this function is called from the main gtk ui thread,
// so you can update gtk properties here
// (but not in the producer runtime, to do that you need to use dynamic properties).
// This function should only setup the runtime to update dynamic properties
// and should return as soon as possible
#[allow(unused_variables)]
fn producer(module: &MusicModule) {
    let config = &module.config;

    let activity_map = module.base_module.registered_activities();

    let current_activities = activity_map.blocking_lock().list_activities();
    let desired_activities: Vec<(&str, usize)> = config
        .windows
        .iter()
        .map(|(window_name, activities)| (window_name.as_str(), activities.len()))
        .collect();

    let (to_remove, to_add) = activities_to_update(&current_activities, &desired_activities);
    for activity_id in to_remove {
        // unregister the activity to remove
        module
            .base_module
            .unregister_activity(activity_id.activity());
    }
    for (window_name, idx) in to_add {
        // create a new dynamic activity and register it
        let actvity = crate::widget::get_activity(
            module.base_module.prop_send(),
            crate::NAME,
            "template-activity",
            window_name,
            idx,
        );
        module.base_module.register_activity(actvity).unwrap();
    }

    // now that only the configured activities remain, we can update their properties
    let activity_list = activity_map.blocking_lock().list_activities();

    // the updates need to be done on a different thread, this way the main ui thread is not blocked
    let rt = module.producers_rt.clone();

    for activity_id in activity_list {
        let idx = get_conf_idx(&activity_id);
        let window_name = activity_id.metadata().window_name().unwrap_or_default();
        let activity_config = config.get_for_window(&window_name, idx);

        // get the roll-char property outside of the runtime
        let roll_char = activity_map
            .blocking_lock()
            .get_property_any_blocking(activity_id.activity(), "roll-char")
            .unwrap();
        rt.handle().spawn(async move {
            loop {
                for char in activity_config.template_subconfig.sub_field1.chars() {
                    roll_char.lock().await.set(char).unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }
        });
    }
}

/// Returns the activities to add and remove to get from the current state to the desired state
///
/// # Arguments
///
/// * `current_state` - The current state of the activities,
/// this can be either the activities that are currently registered (`module.base_module.registered_activities().blocking_lock().list_activities()`) or
/// the activities from the last config update if you saved them in the module
///
/// * `desired_state` - The desired state of the activities,
/// it's a vector of tuples where the first element is the window name and the second element is the number of activities for that window
///
/// # Returns
///
/// `(to_remove, to_add)`
///
/// * `to_remove` - A vector of activities that should be removed
/// * `to_add` - A vector of tuples where the first element is the window name and the second element is the instance number of the activity
pub fn activities_to_update<'a>(
    current_state: &'a Vec<ActivityIdentifier>,
    desired_state: &'a Vec<(&'a str, usize)>,
) -> (Vec<&'a ActivityIdentifier>, Vec<(&'a str, usize)>) {
    // remove activities
    let mut to_remove = Vec::new();
    let mut current_windows = HashMap::new();
    for act in current_state {
        let idx = get_conf_idx(act);
        let window_name = act.metadata().window_name().unwrap_or_default();
        if desired_state
            .iter()
            .find(|(name, count)| *name == window_name && *count > idx)
            .is_none()
        {
            to_remove.push(act);
        }
        let max_idx: usize = *current_windows.get(&window_name).unwrap_or(&0).max(&idx);
        current_windows.insert(window_name, max_idx);
    }
    //add activities
    let mut to_add = Vec::new();
    for (window_name, count) in desired_state {
        if !current_windows.contains_key(&window_name.to_string()) {
            for i in 0..*count {
                to_add.push((*window_name, i));
            }
        } else {
            let current_idx = current_windows.get(*window_name).unwrap() + 1;
            for i in current_idx..*count {
                to_add.push((*window_name, i));
            }
        }
    }
    (to_remove, to_add)
}

/// Returns the instance number of the activity
pub(crate) fn get_conf_idx(id: &ActivityIdentifier) -> usize {
    id.metadata()
        .additional_metadata("instance")
        .unwrap()
        .parse::<usize>()
        .unwrap()
}
