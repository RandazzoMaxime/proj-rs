//! `PJ_OBJ_LIST`: the lists of objects that searches and the operation factory return, and
//! `proj_identify`.

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{AuthorityFactory, AuthorityObject, UserObject};
use proj_rs::unstable::Datum;

use crate::context::{context, Ctx};
use crate::ffi::*;
use crate::iso::Obj;
use crate::pj::{hand_over, pj, Pj};
use crate::util::{self, guard};

/// The objects behind a `PJ_OBJ_LIST *`.
#[derive(Debug)]
pub(crate) struct ObjList {
    pub(crate) objects: Vec<Obj>,
    /// Shared with the items: in PROJ a string read from an item lives as long as the list.
    arena: util::Arena,
}

/// Gives a list of objects to C.
pub(crate) fn object_list(_context: &mut Ctx, objects: Vec<Obj>) -> *mut PJ_OBJ_LIST {
    util::leak(ObjList {
        objects,
        arena: util::Arena::default(),
    })
    .cast::<PJ_OBJ_LIST>()
}

pub(crate) fn list<'a>(pointer: *const PJ_OBJ_LIST) -> Option<&'a ObjList> {
    util::deref(pointer.cast::<ObjList>())
}

/// An object of the authority factory, as an ISO object. Extents and units are not objects of
/// `proj.h`.
pub(crate) fn from_authority(object: AuthorityObject) -> Option<Obj> {
    Some(Obj::User(match object {
        AuthorityObject::PrimeMeridian(value) => UserObject::PrimeMeridian(value),
        AuthorityObject::Ellipsoid(value) => UserObject::Ellipsoid(value),
        AuthorityObject::Datum(Datum::Geodetic(value)) => UserObject::GeodeticDatum(value),
        AuthorityObject::Datum(Datum::Vertical(value)) => UserObject::VerticalDatum(value),
        AuthorityObject::Datum(Datum::Engineering(value)) => UserObject::EngineeringDatum(value),
        AuthorityObject::DatumEnsemble(value) => UserObject::Ensemble(value),
        AuthorityObject::Crs(value) => UserObject::Crs(value),
        AuthorityObject::Operation(value) => UserObject::Operation(value),
        AuthorityObject::Extent(_) | AuthorityObject::Unit(_) => return None,
    }))
}

/// The `PJ` of an object of a list: an operation is instantiated, as `proj_list_get` does.
pub(crate) fn object_pj(context: &mut Ctx, object: &Obj, arena: &util::Arena) -> *mut PJ {
    let pointer = std::ptr::from_mut(context);
    let proj = match object {
        Obj::User(UserObject::Operation(operation)) => {
            let Some(database) = context.database() else {
                return std::ptr::null_mut();
            };
            context
                .builder()
                .proj_from_operation(operation, &database)
                .ok()
        }
        _ => object.engine_proj(context),
    };
    let mut made = Pj::with_obj(pointer, object.clone(), proj);
    made.arena = arena.clone();
    hand_over(made)
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_get_count(result: *const PJ_OBJ_LIST) -> c_int {
    guard(-1, || {
        list(result).map_or(-1, |found| {
            c_int::try_from(found.objects.len()).unwrap_or(c_int::MAX)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_get(
    ctx: *mut PJ_CONTEXT,
    result: *const PJ_OBJ_LIST,
    index: c_int,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(found) = list(result) else {
            return std::ptr::null_mut();
        };
        match usize::try_from(index)
            .ok()
            .and_then(|at| found.objects.get(at))
        {
            Some(object) => object_pj(context, object, &found.arena),
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_list_destroy(result: *mut PJ_OBJ_LIST) {
    guard((), || util::release(result.cast::<ObjList>()));
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_identify(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    auth_name: *const c_char,
    options: *const *const c_char,
    out_confidence: *mut *mut c_int,
) -> *mut PJ_OBJ_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        util::put(out_confidence, std::ptr::null_mut());
        let _ = options;
        let Some(Obj::User(UserObject::Crs(crs))) = pj(obj).and_then(|object| object.obj.as_ref())
        else {
            return std::ptr::null_mut();
        };
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let authority = util::text(auth_name).unwrap_or("");
        let factory = AuthorityFactory::create(&database, authority);
        let found = crs.identify_with(Some(&factory));
        let confidences: Vec<c_int> = found.iter().map(|(_, confidence)| *confidence).collect();
        util::put(out_confidence, util::give_ints(&confidences));
        object_list(
            context,
            found
                .into_iter()
                .map(|(crs, _)| Obj::User(UserObject::Crs(crs)))
                .collect(),
        )
    })
}
