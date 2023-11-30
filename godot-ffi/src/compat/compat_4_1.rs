/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Modern 4.1+ API
//!
//! The extension entry point is passed `get_proc_address` function pointer, which can be used to load all other
//! GDExtension FFI functions dynamically. This is a departure from the previous struct-based approach.
//!
//! Relevant upstream PR: <https://github.com/godotengine/godot/pull/76406>.

use crate as sys;
use crate::compat::BindingCompat;

pub type InitCompat = sys::GDExtensionInterfaceGetProcAddress;

impl BindingCompat for sys::GDExtensionInterfaceGetProcAddress {
    // In WebAssembly, function references and data pointers live in different memory spaces, so trying to read the "memory"
    // at a function pointer (an index into a table) to heuristically determine which API we have (as is done below) won't work.
    #[cfg(target_family = "wasm")]
    fn ensure_static_runtime_compatibility(&self) {}

    #[cfg(not(target_family = "wasm"))]
    fn ensure_static_runtime_compatibility(&self) {
        // In Godot 4.0.x, before the new GetProcAddress mechanism, the init function looked as follows.
        // In place of the `get_proc_address` function pointer, the `p_interface` data pointer was passed.
        //
        // typedef GDExtensionBool (*GDExtensionInitializationFunction)(
        //     const GDExtensionInterface *p_interface,
        //     GDExtensionClassLibraryPtr p_library,
        //     GDExtensionInitialization *r_initialization
        // );
        //
        // Also, the GDExtensionInterface struct was beginning with these fields:
        //
        // typedef struct {
        //     uint32_t version_major;
        //     uint32_t version_minor;
        //     uint32_t version_patch;
        //     const char *version_string;
        //     ...
        // } GDExtensionInterface;
        //
        // As a result, we can try to interpret the function pointer as a legacy GDExtensionInterface data pointer and check if the
        // first fields have values version_major=4 and version_minor=0. This might be deep in UB territory, but the alternative is
        // to not be able to detect Godot 4.0.x at all, and run into UB anyway.
        let get_proc_address = self.expect("get_proc_address unexpectedly null");

        let static_version_str = crate::GdextBuild::godot_static_version_string();

        // Strictly speaking, this is NOT the type GDExtensionGodotVersion but a 4.0 legacy version of it. They have the exact same
        // layout, and due to GDExtension's compatibility promise, the 4.1+ struct won't change; so we can reuse the type.
        // We thus read u32 pointers (field by field).
        let data_ptr = get_proc_address as *const u32; // crowbar it via `as` cast

        // SAFETY: borderline UB, but on Desktop systems, we should be able to reinterpret function pointers as data.
        // On 64-bit systems, a function pointer is typically 8 bytes long, meaning we can interpret 8 bytes of it.
        // On 32-bit systems, we can only read the first 4 bytes safely. If that happens to have value 4 (exceedingly unlikely for
        // a function pointer), it's likely that it's the actual version and we run 4.0.x. In that case, read 4 more bytes.
        let major = unsafe { data_ptr.read() };
        if major == 4 {
            // SAFETY: see above.
            let minor = unsafe { data_ptr.offset(1).read() };
            if minor == 0 {
                // SAFETY: at this point it's reasonably safe to say that we are indeed dealing with that version struct; read the whole.
                let data_ptr = get_proc_address as *const sys::GDExtensionGodotVersion;
                let runtime_version_str = unsafe { read_version_string(&data_ptr.read()) };

                panic!(
                    "gdext was compiled against a newer Godot version: {static_version_str}\n\
                    but loaded by legacy Godot binary, with version:  {runtime_version_str}\n\
                    \n\
                    Update your Godot engine version, or read https://godot-rust.github.io/book/toolchain/compatibility.html.\n\
                    \n"
                );
            }
        }

        // From here we can assume Godot 4.1+. We need to make sure that the runtime version is >= static version.
        // Lexicographical tuple comparison does that.
        let static_version = crate::GdextBuild::godot_static_version_triple();
        let runtime_version_raw = self.runtime_version();

        // SAFETY: Godot provides this version struct.
        let runtime_version = (
            runtime_version_raw.major as u8,
            runtime_version_raw.minor as u8,
            runtime_version_raw.patch as u8,
        );

        if runtime_version < static_version {
            let runtime_version_str = read_version_string(&runtime_version_raw);

            panic!(
                "gdext was compiled against newer Godot version: {static_version_str}\n\
                but loaded by older Godot binary, with version: {runtime_version_str}\n\
                \n\
                Update your Godot engine version, or compile gdext against an older version.\n\
                For more information, read https://godot-rust.github.io/book/toolchain/compatibility.html.\n\
                \n"
            );
        }
    }

    fn runtime_version(&self) -> sys::GDExtensionGodotVersion {
        unsafe {
            let get_proc_address = self.expect("get_proc_address unexpectedly null");
            let get_godot_version = get_proc_address(sys::c_str(b"get_godot_version\0")); //.expect("get_godot_version unexpectedly null");

            let get_godot_version =
                crate::cast_fn_ptr!(get_godot_version as sys::GDExtensionInterfaceGetGodotVersion);

            let mut version = std::mem::MaybeUninit::<sys::GDExtensionGodotVersion>::zeroed();
            get_godot_version(version.as_mut_ptr());
            version.assume_init()
        }
    }

    fn load_interface(&self) -> sys::GDExtensionInterface {
        unsafe { sys::GDExtensionInterface::load(*self) }
    }
}

fn read_version_string(version_ptr: &sys::GDExtensionGodotVersion) -> String {
    let char_ptr = version_ptr.string;

    // SAFETY: `version_ptr` points to a layout-compatible version struct.
    let c_str = unsafe { std::ffi::CStr::from_ptr(char_ptr) };

    String::from_utf8_lossy(c_str.to_bytes())
        .as_ref()
        .strip_prefix("Godot Engine ")
        .unwrap_or(&String::from_utf8_lossy(c_str.to_bytes()))
        .to_string()
}
