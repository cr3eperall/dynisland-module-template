use dynisland_core::{
    abi::{glib, gtk, log},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    graphics::widgets::rolling_char::RollingChar,
};
use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    types::StaticTypeExt,
    Object,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};

glib::wrapper! {
    pub struct Minimal(ObjectSubclass<MinimalPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/example/templateModule/minimal.ui")]
pub struct MinimalPriv {
    #[template_child]
    pub roll: TemplateChild<RollingChar>,
}

#[glib::object_subclass]
impl ObjectSubclass for MinimalPriv {
    const NAME: &'static str = "TemplateMinimalWidget";
    type Type = Minimal;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        RollingChar::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        // If you use template callbacks (for example running a function when a button is pressed), uncomment this
        // klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

// Example of implementing template callbacks
// #[gtk::template_callbacks]
// impl Minimal{
//     #[template_callback]
//     fn do_stuff(&self, _button: &gtk::Button) {
//         log::info!("Button pressed");
//     }
// }

impl ObjectImpl for MinimalPriv {
    fn constructed(&self) {
        self.parent_constructed();
    }
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
}

impl WidgetImpl for MinimalPriv {}

impl Minimal {
    /// registered properties:
    /// * `roll-char`: `char`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();

        // register the property if it doesn't exist
        // this way we can update multiple widgets with the same property
        let _ = activity.add_dynamic_property("roll-char", '0');

        let minimal = this.clone();
        activity
            .subscribe_to_property("roll-char", move |new_value| {
                let value_char = cast_dyn_any!(new_value, char).unwrap();
                log::trace!("char changed: {value_char}");
                minimal.imp().roll.set_current_char(value_char);
            })
            .unwrap();

        this
    }
}
