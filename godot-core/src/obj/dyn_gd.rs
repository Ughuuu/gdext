/*
 * Copyright (c) godot-rust; Bromeon and contributors.
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::engine;
use crate::obj::{bounds, Bounds, Gd, GdDynMut, GodotClass, Inherits};
use std::marker::PhantomData;

pub struct DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    obj: Gd<T>,
    //rc: rc::Weak<B>
    // dyn_ptr: *mut B,
    erased_downcast: ErasedFn<D>,
}

// type ErasedFn<D> = fn(&Gd<engine::Object>) -> *mut D;
type ErasedFn<D> = Box<dyn FnMut(&Gd<engine::Object>) -> *mut D>;

impl<T, D> DynGd<T, D>
where
    T: GodotClass,
    D: ?Sized,
{
    fn dbind_mut(&mut self) -> GdDynMut<T, D> {
        todo!()
    }
}

// fn dynamic_cast<T: GodotClass, D: ?Sized>(obj: &mut T) -> &mut D {
//     todo!()
// }

fn make_fn<T, D>(
    _inferred_type: &Gd<T>,
    _known_type: PhantomData<D>,
    ref_converter: fn(&mut T) -> &mut D,
) -> ErasedFn<D>
where
    T: GodotClass + Bounds<Declarer = bounds::DeclUser> + Inherits<engine::Object>,
    D: ?Sized+'static,
{
    let dynamic_cast = move |obj: &Gd<engine::Object>| {
        let mut obj: Gd<T> = obj.clone().cast(); // TODO optimize as unchecked, no-clone downcast
        let mut guard = obj.bind_mut();
        let obj = ref_converter(&mut *guard);
        obj as *mut D
    };

    Box::new(dynamic_cast)
}

#[allow(unused_macros)]
macro_rules! dyn_gd {
    ($Trait:ty; $obj:expr) => {{
        use ::godot::engine::Object;
        use ::godot::obj::Gd;
        let obj = Gd::from_object($obj);

        // fn downcast<T>(obj: Gd<Object>) -> &$Trait {
        //     let concrete: Gd<T> = obj.cast::<T>();
        //     concrete.bind()
        // }

        let downcast = make_fn(&obj, PhantomData::<$Trait>);
    }};
}
