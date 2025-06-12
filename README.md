# quark
OpenXR rust runtime/api layer implementation library

## Creating an OpenXR API Layer with Quark

### 1. Set up the project

Create a new Cargo project:

```bash
cargo new my_api_layer --lib
cd my_api_layer
```

Update `Cargo.toml`:

```toml
[package]
name = "my_api_layer"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
quark = { path = "../path/to/quark" }
phf = { version = "0.11.3", features = ["macros"] }
```

### 2. Create the OpenXR JSON manifest

Create a file named `XR_APILAYER_NOVENDOR_my_layer.json`:

```json
{
    "file_format_version": "1.0.0",
    "api_layer": {
        "name": "XR_APILAYER_NOVENDOR_my_layer",
        "library_path": "./target/debug/libmy_api_layer.so",
        "api_version": "1.0",
        "implementation_version": "1",
        "description": "My Quark API Layer"
    }
}
```

### 3. Set up a justfile for easy testing

Create a `justfile`:

```make
build:
    cargo build

test:
    XR_ENABLE_API_LAYERS=XR_APILAYER_NOVENDOR_my_layer XR_API_LAYER_PATH=$PWD hello_xr -G Vulkan

build-and-test:
    just build && just test
```

### 4. Implement the API Layer

In `src/lib.rs`:

```rust
use quark::{
    openxr,
    prelude::*,
    Low,
};

// Define your instance data
pub struct InstanceData {
    pub app_name: String,
}

// Factory for creating InstanceData
pub struct InstanceFactory;

unsafe impl quark::Factory<InstanceData> for InstanceFactory {
    unsafe fn create(
        args: quark::CreateArgs<openxr::sys::Instance>,
    ) -> Result<(openxr::Instance, InstanceData), XrErr> {
        let (info, api_layer_info, instance) = args;

        // Parse application info
        let instance_info = unsafe { &*info };
        let app_info = &instance_info.application_info;
        let app_name = unsafe {
            std::ffi::CStr::from_ptr(app_info.application_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        };

        println!("Creating instance for app: {}", app_name);

        // Call the next layer in the chain
        let layer_info = unsafe { &*api_layer_info };
        let result = unsafe {
            ((*layer_info.next_info).next_create_api_layer_instance)(
                info,
                api_layer_info,
                instance,
            )
        };

        if result != XrErr::SUCCESS {
            return Err(result);
        }

        // Create high-level instance wrapper
        let (high, _create_info) = unsafe { (*instance).into_high(args) }?;

        let instance_data = InstanceData { app_name };

        Ok((high, instance_data))
    }
}

impl quark::Hook for InstanceData {
    type Target = openxr::sys::Instance;
    type Factory = InstanceFactory;
}

// Define the API layer
quark::api_layer! {
    hooks: {
        Instance: InstanceData,
    },
    override_fns: {
        // Add functions to override here
        // xrBeginFrame: my_begin_frame,
    }
}

// Implement override functions as needed
// pub unsafe extern "system" fn my_begin_frame(
//     session: openxr::sys::Session,
//     frame_begin_info: *const openxr::sys::FrameBeginInfo,
// ) -> XrErr {
//     // Your custom logic here
//     println!("Beginning frame");
//
//     // Call the original function
//     let instance = quark::find_instance(session)?;
//     let instance_obj = instance.registered()?;
//     unsafe { (instance_obj.fp().begin_frame)(session, frame_begin_info) }
// }
```

## Understanding Hooks vs Function Overrides

Quark provides two ways to extend OpenXR functionality:

### Hooks - Automatic Object Lifecycle Management

Hooks automatically track when OpenXR objects are created and destroyed:

```rust
pub struct SessionData {
    pub system_id: openxr::SystemId,
}

impl quark::Hook for SessionData {
    type Target = openxr::sys::Session;
    type Factory = SessionFactory;

    // Called automatically when a session is created
    fn on_create(
        _handle: &openxr::Session,
        create_info: quark::types::SessionCreateInfo,
    ) -> Result<Self, XrErr> {
        println!("Session created for system: {:?}", create_info.system_id);
        Ok(SessionData {
            system_id: create_info.system_id,
        })
    }
}

// Add to your hooks
quark::api_layer! {
    hooks: {
        Instance: InstanceData,
        Session: SessionData,
    },
    override_fns: {}
}
```

**When to use Hooks:**
- ✅ Track object creation/destruction
- ✅ Store data associated with OpenXR handles
- ✅ Automatic data management and type safety
- ✅ Observe what's happening without changing behavior

### Function Overrides - Custom Behavior

Override specific OpenXR functions to modify their behavior:

```rust
quark::api_layer! {
    hooks: {
        Instance: InstanceData,
        Session: SessionData,
    },
    override_fns: {
        xrBeginFrame: my_begin_frame,
        xrEndFrame: my_end_frame,
    }
}

pub unsafe extern "system" fn my_begin_frame(
    session: openxr::sys::Session,
    frame_begin_info: *const openxr::sys::FrameBeginInfo,
) -> XrErr {
    // Get your session data
    if let Ok(session_data) = session.registered_with_hook::<SessionData>() {
        println!("Beginning frame for system: {:?}", session_data.system_id);
    }

    // Call the original function
    let instance = match quark::find_instance(session) {
        Ok(instance) => instance,
        Err(e) => return e,
    };

    let instance_obj = match instance.registered() {
        Ok(obj) => obj,
        Err(_) => return XrErr::ERROR_HANDLE_INVALID,
    };

    unsafe { (instance_obj.fp().begin_frame)(session, frame_begin_info) }
}
```

**When to use Function Overrides:**
- ✅ Modify function parameters or return values
- ✅ Inject custom logic into the call flow
- ✅ Intercept and transform data
- ✅ Add side effects to existing functions

### Accessing Your Data

Retrieve hook data from handles:

```rust
// Get instance data
let instance_data = instance.registered()?;
println!("App name: {}", instance_data.app_name);

// Get session data with type safety
let session_data = session.registered_with_hook::<SessionData>()?;
println!("System ID: {:?}", session_data.system_id);
```

### 5. Build and test

Run the following command to build and test your API layer:

```bash
just build-and-test
```

### 6. Advanced Example

For a complete working example, see `api_layer_test/src/lib.rs` in this repository. It demonstrates:

- Instance and ActionSet hooks
- Proper error handling
- Data retrieval patterns
- Integration with the OpenXR object lifecycle

### 7. Important Notes

- **Automatic Functions**: Don't override `xrCreate*` functions for objects you're hooking - quark generates these automatically
- **Error Handling**: Use `XrErr` for OpenXR error codes and `XrResult<T>` for functions that return values
- **Thread Safety**: Quark handles thread-safe data storage automatically
- **Memory Management**: Hook data is automatically cleaned up when objects are destroyed

### 8. Debugging

Enable logging to see what's happening:

```bash
RUST_LOG=debug just build-and-test
```

This will show you when hooks are called, objects are created/destroyed, and function overrides are invoked.
