//! Classic projections registered with the PROJ string builder.

mod adams;
mod aea;
mod aeqd;
mod affine;
mod airocean;
mod airy;
mod axisswap;
mod bipc;
mod bonne;
mod calcofi;
mod cass;
mod ccon;
mod cea;
mod chamb;
mod col_urban;
mod conversions;
pub(crate) use conversions::linear_proj_unit;
mod ellip;
mod eqc;
mod eqdc;
mod geos;
mod gnom;
mod gridshift;
mod guyou;
mod healpix;
mod hourglass;
mod imw_p;
mod interrupt;
mod interrupted;
mod krovak;
mod labrd;
mod laea;
mod lagrng;
mod lcca;
mod misc;
mod mod_ster;
mod models;
mod more;
mod nsper;
mod nzmg;
mod ob_tran;
mod ocea;
mod oea;
mod omerc;
mod ortho;
mod patterson;
mod peirce;
mod polyhedral;
mod qsc;
mod robin;
mod rouss;
mod s2;
mod sch;
mod simple;
mod som;
mod sphere2;
mod sphere3;
mod sphere_ops;
mod spheroid;
mod spilhaus;
mod stere;
mod sterea;
mod tmerczoned;
mod topocentric;
mod tpeqd;

use crate::engine::{CreateError, Frame, Params};

pub(crate) fn extra_keys(name: &str) -> Option<&'static [&'static str]> {
    if let Some(keys) = simple::extra_keys(name)
        .or_else(|| adams::extra_keys(name))
        .or_else(|| aeqd::extra_keys(name))
        .or_else(|| affine::extra_keys(name))
        .or_else(|| airocean::extra_keys(name))
        .or_else(|| axisswap::extra_keys(name))
        .or_else(|| airy::extra_keys(name))
        .or_else(|| bipc::extra_keys(name))
        .or_else(|| calcofi::extra_keys(name))
        .or_else(|| cass::extra_keys(name))
        .or_else(|| ccon::extra_keys(name))
        .or_else(|| cea::extra_keys(name))
        .or_else(|| chamb::extra_keys(name))
        .or_else(|| col_urban::extra_keys(name))
        .or_else(|| conversions::extra_keys(name))
        .or_else(|| ellip::extra_keys(name))
        .or_else(|| geos::extra_keys(name))
        .or_else(|| gnom::extra_keys(name))
        .or_else(|| gridshift::extra_keys(name))
        .or_else(|| guyou::extra_keys(name))
        .or_else(|| imw_p::extra_keys(name))
        .or_else(|| healpix::extra_keys(name))
        .or_else(|| hourglass::extra_keys(name))
        .or_else(|| interrupt::extra_keys(name))
        .or_else(|| interrupted::extra_keys(name))
        .or_else(|| krovak::extra_keys(name))
        .or_else(|| labrd::extra_keys(name))
        .or_else(|| laea::extra_keys(name))
        .or_else(|| lcca::extra_keys(name))
        .or_else(|| lagrng::extra_keys(name))
        .or_else(|| misc::extra_keys(name))
        .or_else(|| more::extra_keys(name))
        .or_else(|| mod_ster::extra_keys(name))
        .or_else(|| models::extra_keys(name))
        .or_else(|| nsper::extra_keys(name))
        .or_else(|| nzmg::extra_keys(name))
        .or_else(|| ob_tran::extra_keys(name))
        .or_else(|| ocea::extra_keys(name))
        .or_else(|| oea::extra_keys(name))
        .or_else(|| omerc::extra_keys(name))
        .or_else(|| patterson::extra_keys(name))
        .or_else(|| peirce::extra_keys(name))
        .or_else(|| polyhedral::extra_keys(name))
        .or_else(|| qsc::extra_keys(name))
        .or_else(|| robin::extra_keys(name))
        .or_else(|| rouss::extra_keys(name))
        .or_else(|| s2::extra_keys(name))
        .or_else(|| sch::extra_keys(name))
        .or_else(|| som::extra_keys(name))
        .or_else(|| sphere2::extra_keys(name))
        .or_else(|| sphere3::extra_keys(name))
        .or_else(|| sphere_ops::extra_keys(name))
        .or_else(|| spheroid::extra_keys(name))
        .or_else(|| spilhaus::extra_keys(name))
        .or_else(|| stere::extra_keys(name))
        .or_else(|| sterea::extra_keys(name))
        .or_else(|| tmerczoned::extra_keys(name))
        .or_else(|| tpeqd::extra_keys(name))
        .or_else(|| topocentric::extra_keys(name))
    {
        return Some(keys);
    }
    match name {
        "aea" | "leac" => Some(aea::KEYS),
        "bonne" => Some(bonne::KEYS),
        "eqc" => Some(eqc::KEYS),
        "eqdc" => Some(eqdc::KEYS),
        "ortho" => Some(ortho::KEYS),
        _ => None,
    }
}

pub(crate) fn build(
    name: &str,
    params: &Params,
    frame: &Frame,
) -> Result<Option<Box<dyn crate::engine::Step>>, CreateError> {
    if let Some(step) = simple::build(name, params, frame)?
        .or(adams::build(name, params, frame)?)
        .or(aeqd::build(name, params, frame)?)
        .or(affine::build(name, params, frame)?)
        .or(airocean::build(name, params, frame)?)
        .or(axisswap::build(name, params, frame)?)
        .or(airy::build(name, params, frame)?)
        .or(bipc::build(name, params, frame)?)
        .or(calcofi::build(name, params, frame)?)
        .or(cass::build(name, params, frame)?)
        .or(ccon::build(name, params, frame)?)
        .or(cea::build(name, params, frame)?)
        .or(chamb::build(name, params, frame)?)
        .or(col_urban::build(name, params, frame)?)
        .or(conversions::build(name, params, frame)?)
        .or(ellip::build(name, params, frame)?)
        .or(geos::build(name, params, frame)?)
        .or(gnom::build(name, params, frame)?)
        .or(gridshift::build(name, params, frame)?)
        .or(guyou::build(name, params, frame)?)
        .or(imw_p::build(name, params, frame)?)
        .or(healpix::build(name, params, frame)?)
        .or(hourglass::build(name, params, frame)?)
        .or(interrupt::build(name, params, frame)?)
        .or(interrupted::build(name, params, frame)?)
        .or(krovak::build(name, params, frame)?)
        .or(labrd::build(name, params, frame)?)
        .or(laea::build(name, params, frame)?)
        .or(lcca::build(name, params, frame)?)
        .or(lagrng::build(name, params, frame)?)
        .or(misc::build(name, params, frame)?)
        .or(more::build(name, params, frame)?)
        .or(mod_ster::build(name, params, frame)?)
        .or(models::build(name, params, frame)?)
        .or(nsper::build(name, params, frame)?)
        .or(nzmg::build(name, params, frame)?)
        .or(ob_tran::build(name, params, frame)?)
        .or(ocea::build(name, params, frame)?)
        .or(oea::build(name, params, frame)?)
        .or(omerc::build(name, params, frame)?)
        .or(patterson::build(name, params, frame)?)
        .or(peirce::build(name, params, frame)?)
        .or(polyhedral::build(name, params, frame)?)
        .or(qsc::build(name, params, frame)?)
        .or(robin::build(name, params, frame)?)
        .or(rouss::build(name, params, frame)?)
        .or(s2::build(name, params, frame)?)
        .or(sch::build(name, params, frame)?)
        .or(som::build(name, params, frame)?)
        .or(sphere2::build(name, params, frame)?)
        .or(sphere3::build(name, params, frame)?)
        .or(sphere_ops::build(name, params, frame)?)
        .or(spheroid::build(name, params, frame)?)
        .or(spilhaus::build(name, params, frame)?)
        .or(stere::build(name, params, frame)?)
        .or(sterea::build(name, params, frame)?)
        .or(tpeqd::build(name, params, frame)?)
        .or(tmerczoned::build(name, params, frame)?)
        .or(topocentric::build(name, params, frame)?)
    {
        return Ok(Some(step));
    }
    match name {
        "aea" => Ok(Some(aea::build(params, frame, false)?)),
        "leac" => Ok(Some(aea::build(params, frame, true)?)),
        "bonne" => Ok(Some(bonne::build(params, frame)?)),
        "eqc" => Ok(Some(eqc::build(params, frame)?)),
        "eqdc" => Ok(Some(eqdc::build(params, frame)?)),
        "ortho" => Ok(Some(ortho::build(params, frame)?)),
        _ => Ok(None),
    }
}
