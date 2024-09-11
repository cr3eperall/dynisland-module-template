use dynisland_core::d_macro::{MultiWidgetConfig, OptDeserializeConfig};
use serde::{Deserialize, Serialize};

// MultiWidgetConfig and OptDeserializeConfig create a custom main config struct like this:
// {
//     default_options: ...,
//     windows: {
//         "window_name": [
//             {default_option_overrride: ...
//              child_only_options: ...},
//             ...
//         ],
//         "other_window_name": [
//             ...
//         ],
//         ...
//     }
// }
// where default_options are the options in this struct,
// this also handles setting the default values for missing fields,
// they are picked `default_option_overrride` >> `default_options` >> `Default impl` if there are missing fields
// this also works for subconfigs if they are marked with #[deserialize_struct(DeSubConfigName)]
#[derive(Debug, Serialize, MultiWidgetConfig, OptDeserializeConfig, Clone)]
#[serde(default)]
pub struct TemplateConfig {
    pub(crate) template_field: String,
    #[deserialize_struct(DeTemplateSubConfig)]
    pub(crate) template_subconfig: TemplateSubConfig,
    // if there are child only options, the other non-child-only options should be marked with #[serde(skip_serializing)]
    // to make the generated ron default config cleaner
    // #[child_only]
    // pub(crate) child_only_option: String,
}

#[derive(Debug, Serialize, Deserialize, OptDeserializeConfig, Clone)]
pub struct TemplateSubConfig {
    pub(crate) sub_field1: String,
    pub(crate) sub_field2: String,
}

#[allow(clippy::derivable_impls)]
impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            template_field: String::from("default"),
            template_subconfig: TemplateSubConfig {
                sub_field1: String::from("default subfield1"),
                sub_field2: String::from("default subfield2"),
            },
        }
    }
}
