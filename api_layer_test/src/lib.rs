use quark::{openxr, prelude::*, Low};

// Current API layer macro pattern - uses hooks instead of instance_data
quark::api_layer! {
    hooks: {
        Instance: InstanceData,
        ActionSet: ActionSetData,
    },
    override_fns: {
    }
}

// Instance data structure matching current pattern
pub struct InstanceData {
    pub app_name: String,
    pub action_sets_created: u32,
}

// Factory for creating InstanceData
pub struct InstanceFactory;

unsafe impl quark::Factory<InstanceData> for InstanceFactory {
    unsafe fn create(
        args: quark::CreateArgs<openxr::sys::Instance>,
    ) -> Result<(openxr::Instance, InstanceData), XrErr> {
        let (info, api_layer_info, instance) = args;

        // Get instance create info
        let instance_info = unsafe { &*info };
        let app_info = &instance_info.application_info;
        let app_name = unsafe {
            std::ffi::CStr::from_ptr(app_info.application_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        println!("API Layer Test: Creating instance for app: {}", app_name);

        // Call the next layer
        let layer_info = unsafe { &*api_layer_info };
        let result = unsafe {
            ((*layer_info.next_info).next_create_api_layer_instance)(info, api_layer_info, instance)
        };

        if result != XrErr::SUCCESS {
            return Err(result);
        }

        // Create high-level instance wrapper
        let (high, _create_info) = unsafe { (*instance).into_high(args) }?;

        let instance_data = InstanceData {
            app_name,
            action_sets_created: 0,
        };

        Ok((high, instance_data))
    }
}

impl quark::Hook for InstanceData {
    type Target = openxr::sys::Instance;
    type Factory = InstanceFactory;
}

// ActionSet data structure
pub struct ActionSetData {
    pub name: String,
    pub localized_name: String,
    pub priority: u32,
}

// Factory for creating ActionSetData
pub struct ActionSetFactory;

impl quark::Hook for ActionSetData {
    type Target = openxr::sys::ActionSet;
    type Factory = ActionSetFactory;

    fn on_create(
        _handle: &openxr::ActionSet,
        create_info: quark::types::ActionSetCreateInfo,
    ) -> Result<Self, XrErr> {
        println!(
            "API Layer Test: Hook creating action set: {} ({})",
            create_info.action_set_name, create_info.localized_action_set_name
        );

        Ok(ActionSetData {
            name: create_info.action_set_name,
            localized_name: create_info.localized_action_set_name,
            priority: create_info.priority,
        })
    }
}

unsafe impl quark::Factory<ActionSetData> for ActionSetFactory {
    unsafe fn create(
        args: quark::CreateArgs<openxr::sys::ActionSet>,
    ) -> Result<(openxr::ActionSet, ActionSetData), XrErr> {
        let (instance, create_info, action_set) = args;
        let instance_obj = instance.registered()?;
        let create_info = unsafe { quark::types::ActionSetCreateInfo::from_raw(create_info)? };

        let openxr_action_set = instance_obj.create_action_set(
            &create_info.action_set_name,
            &create_info.localized_action_set_name,
            create_info.priority,
        )?;

        unsafe { *action_set = openxr_action_set.as_raw() };

        let data = ActionSetData {
            name: create_info.action_set_name,
            localized_name: create_info.localized_action_set_name,
            priority: create_info.priority,
        };

        Ok((openxr_action_set, data))
    }
}

// Since xrCreateAction is auto-generated, we demonstrate hook usage instead
// The ActionSetData Hook will automatically be called when action sets are created
