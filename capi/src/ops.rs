//! The operation factory and what an operation is made of (`proj_create_operations`,
//! `proj_operation_factory_context_*`, `proj_coordoperation_*`).

use std::ffi::{c_char, c_int};

use proj_rs::iso19111::{
    AlternativeGridNames, BallparkUse, BoundingBox, CoordinateOperation,
    CoordinateOperationContext, CrsExtentUse, GridAvailabilityUse, IntermediateCrsUse, NameMatch,
    ObjectDomain, ParameterValue, SpatialCriterion, SupersededOperations, UserObject,
};
use proj_rs::unstable::Proj;

use crate::context::{context, Ctx};
use crate::ffi::*;
use crate::iso::{derived, Obj};
use crate::list::{list, object_list};
use crate::pj::{pj, Pj};
use crate::util::{self, guard};

/// The object behind a `PJ_OPERATION_FACTORY_CONTEXT *`.
#[derive(Debug)]
pub(crate) struct FactoryContext {
    inner: CoordinateOperationContext,
}

fn factory<'a>(pointer: *const PJ_OPERATION_FACTORY_CONTEXT) -> Option<&'a mut FactoryContext> {
    util::deref_mut(pointer.cast_mut().cast::<FactoryContext>())
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_operation_factory_context(
    ctx: *mut PJ_CONTEXT,
    authority: *const c_char,
) -> *mut PJ_OPERATION_FACTORY_CONTEXT {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(database) = context.database() else {
            return std::ptr::null_mut();
        };
        let inner =
            CoordinateOperationContext::create(&database, util::text(authority).unwrap_or(""));
        util::leak(FactoryContext { inner }).cast::<PJ_OPERATION_FACTORY_CONTEXT>()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_destroy(ctx: *mut PJ_OPERATION_FACTORY_CONTEXT) {
    guard((), || util::release(ctx.cast::<FactoryContext>()));
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_desired_accuracy(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    accuracy: f64,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            target.inner.set_desired_accuracy(accuracy);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_area_of_interest(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    west_lon_degree: f64,
    south_lat_degree: f64,
    east_lon_degree: f64,
    north_lat_degree: f64,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            let bbox = BoundingBox::new(
                west_lon_degree,
                south_lat_degree,
                east_lon_degree,
                north_lat_degree,
            );
            target
                .inner
                .set_area_of_interest(Some(ObjectDomain::from_bounding_box(bbox)));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_area_of_interest_name(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    area_name: *const c_char,
) {
    guard((), || {
        let context = context(ctx);
        let (Some(target), Some(name), Some(database)) = (
            factory(factory_ctx),
            util::text(area_name),
            context.database(),
        ) else {
            return;
        };
        let found = proj_rs::iso19111::AuthorityFactory::create(&database, "")
            .list_area_of_use_from_name(name, NameMatch::Exact);
        let Ok(list) = found else {
            return;
        };
        let [(authority, code)] = list.as_slice() else {
            context.fail(
                PROJ_ERR_OTHER as c_int,
                "proj_operation_factory_context_set_area_of_interest_name: no unique area",
            );
            return;
        };
        if let Ok(domain) =
            proj_rs::iso19111::AuthorityFactory::create(&database, authority).create_extent(code)
        {
            target.inner.set_area_of_interest(Some(domain));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_crs_extent_use(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    use_: PROJ_CRS_EXTENT_USE,
) {
    guard((), || {
        let _ = context(ctx);
        let Some(target) = factory(factory_ctx) else {
            return;
        };
        let value = match use_ {
            PJ_CRS_EXTENT_NONE => CrsExtentUse::Ignore,
            PJ_CRS_EXTENT_BOTH => CrsExtentUse::Both,
            PJ_CRS_EXTENT_INTERSECTION => CrsExtentUse::Intersection,
            _ => CrsExtentUse::Smallest,
        };
        target.inner.set_crs_extent_use(value);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_spatial_criterion(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    criterion: PROJ_SPATIAL_CRITERION,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            target.inner.set_spatial_criterion(
                if criterion == PROJ_SPATIAL_CRITERION_PARTIAL_INTERSECTION {
                    SpatialCriterion::PartialIntersection
                } else {
                    SpatialCriterion::StrictContainment
                },
            );
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_grid_availability_use(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    use_: PROJ_GRID_AVAILABILITY_USE,
) {
    guard((), || {
        let _ = context(ctx);
        let Some(target) = factory(factory_ctx) else {
            return;
        };
        target.inner.set_grid_availability_use(match use_ {
            PROJ_GRID_AVAILABILITY_DISCARD_OPERATION_IF_MISSING_GRID => {
                GridAvailabilityUse::DiscardIfMissing
            }
            PROJ_GRID_AVAILABILITY_IGNORED => GridAvailabilityUse::IgnoreGridAvailability,
            PROJ_GRID_AVAILABILITY_KNOWN_AVAILABLE => GridAvailabilityUse::KnownAvailable,
            _ => GridAvailabilityUse::UseForSorting,
        });
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_use_proj_alternative_grid_names(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    usePROJNames: c_int,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            target
                .inner
                .set_alternative_grid_names(if usePROJNames != 0 {
                    AlternativeGridNames::Proj
                } else {
                    AlternativeGridNames::Official
                });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_allow_use_intermediate_crs(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    use_: PROJ_INTERMEDIATE_CRS_USE,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            target.inner.set_intermediate_crs_use(match use_ {
                PROJ_INTERMEDIATE_CRS_USE_ALWAYS => IntermediateCrsUse::Always,
                PROJ_INTERMEDIATE_CRS_USE_NEVER => IntermediateCrsUse::Never,
                _ => IntermediateCrsUse::Allow,
            });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_allowed_intermediate_crs(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    list_of_auth_name_codes: *const *const c_char,
) {
    guard((), || {
        let _ = context(ctx);
        let Some(target) = factory(factory_ctx) else {
            return;
        };
        let mut pairs = Vec::new();
        if !list_of_auth_name_codes.is_null() {
            let mut index = 0;
            // The NULL that ends the list is read before the slot after it: reading the code of
            // a NULL authority went past the end of the array (found by pyproj, a segfault).
            while let Some(authority) =
                util::get(list_of_auth_name_codes.wrapping_add(index)).and_then(util::text)
            {
                let Some(code) =
                    util::get(list_of_auth_name_codes.wrapping_add(index + 1)).and_then(util::text)
                else {
                    break;
                };
                pairs.push((authority.to_owned(), code.to_owned()));
                index += 2;
            }
        }
        target.inner.set_intermediate_crs(pairs);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_discard_superseded(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    discard: c_int,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            target.inner.set_superseded_operations(if discard != 0 {
                SupersededOperations::Discard
            } else {
                SupersededOperations::Keep
            });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_operation_factory_context_set_allow_ballpark_transformations(
    ctx: *mut PJ_CONTEXT,
    factory_ctx: *mut PJ_OPERATION_FACTORY_CONTEXT,
    allow: c_int,
) {
    guard((), || {
        let _ = context(ctx);
        if let Some(target) = factory(factory_ctx) {
            target.inner.set_ballpark_use(if allow != 0 {
                BallparkUse::Allow
            } else {
                BallparkUse::Forbid
            });
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_create_operations(
    ctx: *mut PJ_CONTEXT,
    source_crs: *const PJ,
    target_crs: *const PJ,
    operationContext: *const PJ_OPERATION_FACTORY_CONTEXT,
) -> *mut PJ_OBJ_LIST {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let (Some(source), Some(target), Some(factory_context)) = (
            pj(source_crs),
            pj(target_crs),
            factory(operationContext.cast_mut()),
        ) else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_operations: missing required input",
            );
            return std::ptr::null_mut();
        };
        // `source_crs is not a CRS or a CoordinateMetadata` (`c_api.cpp:8581-8605`), through
        // `proj_log_error`. A coordinate metadata carries an epoch here, which the factory of this
        // library does not take: NULL, as before.
        let mut crs = [None, None];
        for (slot, (object, role)) in crs
            .iter_mut()
            .zip([(source, "source_crs"), (target, "target_crs")])
        {
            let found = object.obj.as_ref();
            *slot = found.and_then(Obj::crs);
            let metadata = matches!(found, Some(Obj::User(UserObject::CoordinateMetadata(_))));
            if slot.is_none() && !metadata {
                context.fail_unless_set(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_create_operations: {role} is not a CRS or a CoordinateMetadata"),
                );
                return std::ptr::null_mut();
            }
        }
        let [Some(from), Some(to)] = crs else {
            context.fail(
                PROJ_ERR_OTHER_API_MISUSE as c_int,
                "proj_create_operations: not a CRS",
            );
            return std::ptr::null_mut();
        };
        match context
            .builder()
            .create_operations(from, to, &factory_context.inner)
        {
            Ok(found) => object_list(
                context,
                found
                    .into_iter()
                    .map(|operation| Obj::User(UserObject::Operation(operation)))
                    .collect(),
            ),
            Err(error) => {
                context.fail(
                    PROJ_ERR_OTHER as c_int,
                    &format!("proj_create_operations: {error}"),
                );
                std::ptr::null_mut()
            }
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_get_suggested_operation(
    ctx: *mut PJ_CONTEXT,
    operations: *mut PJ_OBJ_LIST,
    direction: PJ_DIRECTION,
    coord: PJ_COORD,
) -> c_int {
    guard(-1, || {
        let context = context(ctx);
        let Some(found) = list(operations) else {
            return -1;
        };
        let items: Vec<CoordinateOperation> = found
            .objects
            .iter()
            .filter_map(|object| object.operation().cloned())
            .collect();
        let Some(first) = items.first() else {
            return -1;
        };
        let source = if direction >= 0 {
            crate::inspect::operation_source(first)
        } else {
            crate::inspect::operation_target(first)
        };
        let Some(source) = source else {
            return -1;
        };
        let [x, y, z, _] = crate::trans::components(coord);
        let Some(database) = context.database() else {
            return -1;
        };
        let chosen = proj_rs::iso19111::CoordinateOperationFactory::create()
            .suggested_operation_xyz(&items, &source, (x, y, z));
        let _ = database;
        match chosen {
            Ok(Some(index)) => c_int::try_from(index).unwrap_or(-1),
            _ => -1,
        }
    })
}

// ---------------------------------------------------------------------------
// What an operation is.

fn operation_of<'a>(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
    who: &str,
) -> Option<(&'a mut Pj, &'a CoordinateOperation)> {
    let context = context(ctx);
    let Some(object) = pj(obj) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: null object"),
        );
        return None;
    };
    let Some(found) = object.obj.as_ref().and_then(Obj::operation) else {
        context.fail(
            PROJ_ERR_OTHER_API_MISUSE as c_int,
            &format!("{who}: Object is not a CoordinateOperation"),
        );
        return None;
    };
    let reference: &'a CoordinateOperation = util::deref(std::ptr::from_ref(found))?;
    Some((object, reference))
}

/// `considerKnownGridsAsAvailable` of `isPROJInstantiable` and `gridsNeeded` in the C API: a
/// grid the database knows counts as available when the network is on (c_api.cpp:7552, 7925).
fn known_grids(context: &Ctx) -> GridAvailabilityUse {
    if context.builder.network_enabled() {
        GridAvailabilityUse::KnownAvailable
    } else {
        GridAvailabilityUse::UseForSorting
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_is_instantiable(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some((_, operation)) =
            operation_of(ctx, coordoperation, "proj_coordoperation_is_instantiable")
        else {
            return 0;
        };
        let Some(database) = context.database() else {
            return 0;
        };
        c_int::from(operation.is_proj_instantiable(&database, known_grids(context)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_has_ballpark_transformation(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
) -> c_int {
    guard(0, || {
        operation_of(
            ctx,
            coordoperation,
            "proj_coordoperation_has_ballpark_transformation",
        )
        .map_or(0, |(_, operation)| {
            c_int::from(operation.has_ballpark_transformation())
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_requires_per_coordinate_input_time(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
) -> c_int {
    guard(0, || {
        operation_of(
            ctx,
            coordoperation,
            "proj_coordoperation_requires_per_coordinate_input_time",
        )
        .map_or(0, |(_, operation)| {
            c_int::from(operation.requires_per_coordinate_input_time())
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_accuracy(ctx: *mut PJ_CONTEXT, obj: *const PJ) -> f64 {
    guard(-1.0, || {
        operation_of(ctx, obj, "proj_coordoperation_get_accuracy")
            .map_or(-1.0, |(_, operation)| operation.accuracy().unwrap_or(-1.0))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_create_inverse(
    ctx: *mut PJ_CONTEXT,
    obj: *const PJ,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let context = context(ctx);
        let Some(object) = pj(obj) else {
            return std::ptr::null_mut();
        };
        let native = object
            .proj
            .as_ref()
            .is_some_and(|proj| proj.native_operation().is_some());
        if let (false, Some(operation)) = (native, object.obj.as_ref().and_then(Obj::operation)) {
            return derived(
                object,
                Obj::User(UserObject::Operation(operation.inverse())),
            );
        }
        match object
            .proj
            .as_ref()
            .map(Proj::coordoperation_create_inverse)
        {
            Some(Ok(inverse)) => {
                let mut copy = object.copy_in(object.ctx);
                copy.proj = Some(inverse);
                crate::pj::hand_over(copy)
            }
            Some(Err(error)) => {
                context.fail(
                    error.code(),
                    &format!("proj_coordoperation_create_inverse: {error}"),
                );
                std::ptr::null_mut()
            }
            None => std::ptr::null_mut(),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_concatoperation_get_step_count(
    ctx: *mut PJ_CONTEXT,
    concatoperation: *const PJ,
) -> c_int {
    guard(0, || {
        match operation_of(ctx, concatoperation, "proj_concatoperation_get_step_count") {
            Some((_, CoordinateOperation::Concatenated(concatenated)))
                if !concatenated.is_proj_based() =>
            {
                c_int::try_from(concatenated.steps().len()).unwrap_or(0)
            }
            _ => 0,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_concatoperation_get_step(
    ctx: *mut PJ_CONTEXT,
    concatoperation: *const PJ,
    i_step: c_int,
) -> *mut PJ {
    guard(std::ptr::null_mut(), || {
        let Some((object, CoordinateOperation::Concatenated(concatenated))) =
            operation_of(ctx, concatoperation, "proj_concatoperation_get_step")
        else {
            return std::ptr::null_mut();
        };
        if concatenated.is_proj_based() {
            return std::ptr::null_mut();
        }
        match usize::try_from(i_step)
            .ok()
            .and_then(|at| concatenated.steps().get(at))
        {
            Some(step) => derived(
                object,
                Obj::User(UserObject::Operation(step.operation().clone())),
            ),
            None => std::ptr::null_mut(),
        }
    })
}

fn method_and_parameters(
    operation: &CoordinateOperation,
) -> Option<(
    &proj_rs::iso19111::OperationMethod,
    &[proj_rs::iso19111::Parameter],
)> {
    match operation {
        CoordinateOperation::Conversion(value) => Some((value.method(), value.parameters())),
        CoordinateOperation::Transformation(value) => Some((value.method(), value.parameters())),
        CoordinateOperation::Concatenated(_) => None,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_method_info(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
    out_method_name: *mut *const c_char,
    out_method_auth_name: *mut *const c_char,
    out_method_code: *mut *const c_char,
) -> c_int {
    guard(0, || {
        let Some((object, operation)) =
            operation_of(ctx, coordoperation, "proj_coordoperation_get_method_info")
        else {
            return 0;
        };
        // A PROJ-based operation is read through the transformation PROJ holds for it.
        let exposed = match operation {
            CoordinateOperation::Concatenated(concatenated) => concatenated
                .proj_based_transformation()
                .ok()
                .flatten()
                .map(CoordinateOperation::Transformation),
            _ => None,
        };
        let Some((method, _)) = method_and_parameters(exposed.as_ref().unwrap_or(operation)) else {
            return 0;
        };
        let (name, space, code) = (
            method.name().to_owned(),
            method.code_space().to_owned(),
            method.code().to_owned(),
        );
        util::put(out_method_name, object.arena.put(&name));
        util::put(
            out_method_auth_name,
            object
                .arena
                .put_opt((!space.is_empty()).then_some(space.as_str())),
        );
        util::put(
            out_method_code,
            object
                .arena
                .put_opt((!code.is_empty()).then_some(code.as_str())),
        );
        1
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_param_count(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
) -> c_int {
    guard(0, || {
        operation_of(ctx, coordoperation, "proj_coordoperation_get_param_count")
            .and_then(|(_, operation)| method_and_parameters(operation))
            .map_or(0, |(_, parameters)| {
                c_int::try_from(parameters.len()).unwrap_or(0)
            })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_param_index(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
    name: *const c_char,
) -> c_int {
    guard(-1, || {
        let (Some((_, operation)), Some(name)) = (
            operation_of(ctx, coordoperation, "proj_coordoperation_get_param_index"),
            util::text(name),
        ) else {
            return -1;
        };
        let Some((_, parameters)) = method_and_parameters(operation) else {
            return -1;
        };
        parameters
            .iter()
            .position(|parameter| proj_rs::iso19111::equivalent_name(parameter.name(), name))
            .and_then(|at| c_int::try_from(at).ok())
            .unwrap_or(-1)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_param(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
    index: c_int,
    out_name: *mut *const c_char,
    out_auth_name: *mut *const c_char,
    out_code: *mut *const c_char,
    out_value: *mut f64,
    out_value_string: *mut *const c_char,
    out_unit_conv_factor: *mut f64,
    out_unit_name: *mut *const c_char,
    out_unit_auth_name: *mut *const c_char,
    out_unit_code: *mut *const c_char,
    out_unit_category: *mut *const c_char,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some((object, operation)) =
            operation_of(ctx, coordoperation, "proj_coordoperation_get_param")
        else {
            return 0;
        };
        let Some((_, parameters)) = method_and_parameters(operation) else {
            return 0;
        };
        let Some(parameter) = usize::try_from(index)
            .ok()
            .and_then(|at| parameters.get(at))
        else {
            context.fail(PROJ_ERR_OTHER_API_MISUSE as c_int, "Invalid index");
            return 0;
        };
        let arena = &mut object.arena;
        let optional = |text: &str| (!text.is_empty()).then(|| text.to_owned());
        util::put(out_name, arena.put(parameter.name()));
        util::put(
            out_auth_name,
            arena.put_opt(optional(parameter.code_space()).as_deref()),
        );
        util::put(
            out_code,
            arena.put_opt(optional(parameter.code()).as_deref()),
        );
        util::put(out_value, f64::NAN);
        util::put(out_value_string, std::ptr::null());
        util::put(out_unit_conv_factor, 0.0);
        util::put(out_unit_name, std::ptr::null());
        util::put(out_unit_auth_name, std::ptr::null());
        util::put(out_unit_code, std::ptr::null());
        util::put(out_unit_category, std::ptr::null());
        match parameter.value() {
            ParameterValue::Measure(measure) => {
                let unit = measure.unit();
                util::put(out_value, measure.value());
                util::put(out_unit_conv_factor, unit.conversion_to_si());
                util::put(out_unit_name, arena.put(unit.name()));
                util::put(
                    out_unit_auth_name,
                    arena.put_opt(optional(unit.code_space()).as_deref()),
                );
                util::put(
                    out_unit_code,
                    arena.put_opt(optional(unit.code()).as_deref()),
                );
                util::put(
                    out_unit_category,
                    arena.put(crate::inspect::category_name(unit.kind())),
                );
            }
            ParameterValue::String(text) | ParameterValue::Filename(text) => {
                util::put(out_value_string, arena.put(text));
            }
            _ => {}
        }
        1
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_grid_used_count(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some((_, operation)) = operation_of(
            ctx,
            coordoperation,
            "proj_coordoperation_get_grid_used_count",
        ) else {
            return 0;
        };
        let Some(database) = context.database() else {
            return 0;
        };
        operation
            .grids_needed(&database, known_grids(context))
            .map_or(0, |grids| c_int::try_from(grids.len()).unwrap_or(0))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn proj_coordoperation_get_grid_used(
    ctx: *mut PJ_CONTEXT,
    coordoperation: *const PJ,
    index: c_int,
    out_short_name: *mut *const c_char,
    out_full_name: *mut *const c_char,
    out_package_name: *mut *const c_char,
    out_url: *mut *const c_char,
    out_direct_download: *mut c_int,
    out_open_license: *mut c_int,
    out_available: *mut c_int,
) -> c_int {
    guard(0, || {
        let context = context(ctx);
        let Some((object, operation)) =
            operation_of(ctx, coordoperation, "proj_coordoperation_get_grid_used")
        else {
            return 0;
        };
        let Some(database) = context.database() else {
            return 0;
        };
        let endpoint = context.builder.get_url_endpoint().unwrap_or_default();
        let Ok(grids) = operation.grids_needed_at(&database, known_grids(context), &endpoint)
        else {
            return 0;
        };
        let Some(grid) = usize::try_from(index).ok().and_then(|at| grids.get(at)) else {
            context.fail(PROJ_ERR_OTHER_API_MISUSE as c_int, "Invalid index");
            return 0;
        };
        let arena = &mut object.arena;
        util::put(out_short_name, arena.put(&grid.short_name));
        util::put(out_full_name, arena.put(&grid.info.full_name));
        util::put(out_package_name, arena.put(&grid.info.package_name));
        util::put(out_url, arena.put(&grid.info.url));
        util::put(out_direct_download, c_int::from(grid.info.direct_download));
        util::put(out_open_license, c_int::from(grid.info.open_license));
        util::put(out_available, c_int::from(grid.info.available));
        1
    })
}
