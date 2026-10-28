//! Minimal WKT and PROJJSON for geographic and projected CRS.
//!
//! [`crate::Proj::new`] accepts either text only when the conversion method is
//! one this crate already builds. A geocentric, compound, or bound CRS is an
//! error, as is a method that has no `+proj`.

use std::f64::consts::PI;

const DEG_TO_RAD: f64 = PI / 180.0;

/// The text is WKT or PROJJSON, not a PROJ string.
pub(crate) fn is_crs_text(text: &str) -> bool {
    let text = trim_bom(text);
    is_wkt_head(text) || text.starts_with('{')
}

/// PROJ string for a geographic or projected CRS.
///
/// # Errors
///
/// A string when the text is not a geographic or projected CRS, the ellipsoid
/// is missing, or the conversion method is not built.
pub(crate) fn to_proj(text: &str) -> Result<String, String> {
    let text = trim_bom(text);
    if text.starts_with('{') {
        return json_proj(text);
    }
    if is_wkt_head(text) {
        return wkt_proj(text);
    }
    Err("not WKT or PROJJSON".to_owned())
}

pub(crate) fn is_wkt(text: &str) -> bool {
    is_wkt_head(trim_bom(text))
}

fn trim_bom(text: &str) -> &str {
    text.trim().trim_start_matches('\u{feff}')
}

fn is_wkt_head(text: &str) -> bool {
    let head: String = text
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if head.is_empty() {
        return false;
    }
    let rest = text[head.len()..].trim_start();
    rest.starts_with('[') && wkt_keyword(&head)
}

fn wkt_keyword(head: &str) -> bool {
    matches!(
        head.to_ascii_uppercase().as_str(),
        "GEOGCS"
            | "GEOGCRS"
            | "GEODCRS"
            | "GEOGRAPHICCRS"
            | "BASEGEOGCRS"
            | "BASEGEODCRS"
            | "PROJCS"
            | "PROJCRS"
            | "PROJECTEDCRS"
            | "BASEPROJCRS"
            | "BOUNDCRS"
            | "COMPOUNDCRS"
            | "VERTCRS"
            | "VERT_CS"
            | "VERTICALCRS"
            | "GEOCCS"
            | "GEOCENTRICCRS"
            | "ENGCRS"
            | "ENGINEERINGCRS"
            | "TIMECRS"
            | "DERIVEDPROJCRS"
    )
}

fn wkt_proj(text: &str) -> Result<String, String> {
    let node = parse_wkt(text)?;
    if has_keyword(&node, "TOWGS84") {
        return Err("WKT datum shift is not built".to_owned());
    }
    match node.keyword.to_ascii_uppercase().as_str() {
        "GEOGCS" | "GEOGCRS" | "GEODCRS" | "GEOGRAPHICCRS" | "BASEGEOGCRS" | "BASEGEODCRS" => {
            let shape = ellipsoid_of(&node)?;
            greenwich(&node)?;
            Ok(format!("+proj=longlat {} +no_defs", shape.clause()))
        }
        "PROJCS" | "PROJCRS" | "PROJECTEDCRS" | "BASEPROJCRS" => projected_wkt(&node),
        _ => Err(format!(
            "{} is not a geographic or projected CRS",
            node.keyword
        )),
    }
}

fn projected_wkt(node: &Node) -> Result<String, String> {
    let shape = ellipsoid_of(node)?;
    greenwich(node)?;
    let method = find_one(node, &["PROJECTION", "METHOD"])
        .ok_or_else(|| format!("{} has no conversion method", node.keyword))?;
    let name = first_string(method).ok_or_else(|| "conversion method has no name".to_owned())?;
    let mut params = Vec::new();
    for parameter in find_all(node, "PARAMETER") {
        let (key, quantity) = param_key(first_string(parameter).unwrap_or(""))?;
        let (number, unit) = parameter_number(parameter)?;
        let value = apply_unit(number, quantity, unit)?;
        if params.iter().any(|(existing, _)| *existing == key) {
            return Err(format!("parameter `{key}` is repeated"));
        }
        params.push((key, value));
    }
    emit_method(name, &shape, &mut params)
}

fn json_proj(text: &str) -> Result<String, String> {
    let value = parse_json(text)?;
    let root = value
        .as_object()
        .ok_or_else(|| "PROJJSON root is not an object".to_owned())?;
    let kind = root
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "PROJJSON has no type".to_owned())?;
    match kind {
        "GeographicCRS" => {
            let shape = json_ellipsoid(root)?;
            Ok(format!("+proj=longlat {} +no_defs", shape.clause()))
        }
        "ProjectedCRS" => {
            let shape = json_ellipsoid(root)?;
            let conversion = root
                .get("conversion")
                .and_then(Value::as_object)
                .ok_or_else(|| "ProjectedCRS has no conversion".to_owned())?;
            let name = json_method(conversion)?;
            let mut params = Vec::new();
            if let Some(items) = conversion.get("parameters").and_then(Value::as_array) {
                for item in items {
                    let object = item
                        .as_object()
                        .ok_or_else(|| "PROJJSON parameter is not an object".to_owned())?;
                    let pname = object
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "PROJJSON parameter has no name".to_owned())?;
                    let (key, quantity) = param_key(pname)?;
                    let number = object
                        .get("value")
                        .and_then(Value::as_f64)
                        .ok_or_else(|| format!("parameter `{pname}` has no value"))?;
                    let value = match object.get("unit") {
                        Some(unit) => apply_json_unit(number, quantity, unit)?,
                        None => number,
                    };
                    if params.iter().any(|(existing, _)| *existing == key) {
                        return Err(format!("parameter `{key}` is repeated"));
                    }
                    params.push((key, value));
                }
            }
            emit_method(name, &shape, &mut params)
        }
        other => Err(format!("{other} is not a geographic or projected CRS")),
    }
}

fn json_method(conversion: &Object) -> Result<&str, String> {
    match conversion.get("method") {
        Some(Value::String(text)) => Ok(text),
        Some(Value::Object(object)) => object
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| "conversion method has no name".to_owned()),
        _ => Err("conversion method has no name".to_owned()),
    }
}

fn json_ellipsoid(root: &Object) -> Result<Shape, String> {
    let mut found = None;
    walk_ellipsoid(&Value::Object(root.clone()), &mut found);
    found.ok_or_else(|| "CRS has no ellipsoid".to_owned())
}

fn walk_ellipsoid(value: &Value, found: &mut Option<Shape>) {
    if found.is_some() {
        return;
    }
    match value {
        Value::Object(object) => {
            if let Some(axis) = object.get("semi_major_axis").and_then(Value::as_f64) {
                let rf = object.get("inverse_flattening").and_then(Value::as_f64);
                let b = object.get("semi_minor_axis").and_then(Value::as_f64);
                if let Some(shape) = Shape::new(axis, rf, b) {
                    *found = Some(shape);
                    return;
                }
            }
            for (_, child) in &object.0 {
                walk_ellipsoid(child, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk_ellipsoid(item, found);
            }
        }
        _ => {}
    }
}

fn emit_method(
    name: &str,
    shape: &Shape,
    params: &mut Vec<(&'static str, f64)>,
) -> Result<String, String> {
    let method = norm(name);
    let (proj, sphere, one_sp, merc_origin) = match method.as_str() {
        "transverse mercator" => ("tmerc", false, false, false),
        "lambert conic conformal (2sp)"
        | "lambert conic conformal 2sp"
        | "lambert conformal conic 2sp"
        | "lambert conformal conic (2sp)" => ("lcc", false, false, false),
        "lambert conic conformal (1sp)"
        | "lambert conic conformal 1sp"
        | "lambert conformal conic 1sp"
        | "lambert conformal conic (1sp)" => ("lcc", false, true, false),
        "mercator (variant a)"
        | "mercator variant a"
        | "mercator (1sp)"
        | "mercator 1sp"
        | "mercator" => ("merc", false, false, true),
        "mercator (variant b)" | "mercator variant b" | "mercator (2sp)" | "mercator 2sp" => {
            ("merc", false, false, false)
        }
        "popular visualisation pseudo mercator" | "mercator auxiliary sphere" => {
            ("merc", true, false, false)
        }
        "albers equal area" | "albers conic equal area" => ("aea", false, false, false),
        "lambert azimuthal equal area" => ("laea", false, false, false),
        "cassini soldner" => ("cass", false, false, false),
        "oblique stereographic" => ("sterea", false, false, false),
        "polar stereographic (variant a)"
        | "polar stereographic variant a"
        | "polar stereographic (variant b)"
        | "polar stereographic variant b"
        | "stereographic" => ("stere", false, false, false),
        "american polyconic" => ("poly", false, false, false),
        "krovak" | "krovak north orientated" | "krovak (north orientated)" => {
            ("krovak", false, false, false)
        }
        "new zealand map grid" => ("nzmg", false, false, false),
        "equidistant cylindrical" | "equirectangular" => ("eqc", false, false, false),
        "lambert cylindrical equal area" => ("cea", false, false, false),
        "bonne" => ("bonne", false, false, false),
        "orthographic" => ("ortho", false, false, false),
        "azimuthal equidistant" => ("aeqd", false, false, false),
        "hotine oblique mercator (variant a)"
        | "hotine oblique mercator variant a"
        | "hotine oblique mercator (variant b)"
        | "hotine oblique mercator variant b" => ("omerc", false, false, false),
        _ => return Err(format!("conversion method `{name}` is not built")),
    };
    if one_sp && !params.iter().any(|(key, _)| *key == "lat_1") {
        if let Some((_, lat)) = params.iter().find(|(key, _)| *key == "lat_0") {
            params.push(("lat_1", *lat));
        }
    }
    if merc_origin {
        if let Some((_, lat)) = params.iter().find(|(key, _)| *key == "lat_0") {
            if lat.abs() > 1e-12 {
                return Err(format!("Mercator latitude of origin {lat} is not built"));
            }
        }
        params.retain(|(key, _)| *key != "lat_0");
    }
    let mut parts = vec![format!("+proj={proj}")];
    for (key, value) in params {
        if merc_origin && *key == "lat_0" {
            continue;
        }
        parts.push(format!("+{key}={}", format_number(*value)));
    }
    if sphere {
        parts.push(format!(
            "+a={} +b={}",
            format_number(shape.a),
            format_number(shape.a)
        ));
    } else {
        parts.push(shape.clause());
    }
    parts.push("+no_defs".to_owned());
    Ok(parts.join(" "))
}

fn param_key(name: &str) -> Result<(&'static str, Quantity), String> {
    let key = match norm(name).as_str() {
        "latitude of natural origin" | "latitude of origin" | "latitude of false origin" => {
            ("lat_0", Quantity::Angle)
        }
        "longitude of natural origin"
        | "longitude of origin"
        | "longitude of false origin"
        | "central meridian" => ("lon_0", Quantity::Angle),
        "scale factor at natural origin"
        | "scale factor"
        | "scale factor at false origin"
        | "scale factor on initial line" => ("k_0", Quantity::Scale),
        "false easting" | "easting at false origin" => ("x_0", Quantity::Length),
        "false northing" | "northing at false origin" => ("y_0", Quantity::Length),
        "latitude of 1st standard parallel" | "standard parallel 1" => ("lat_1", Quantity::Angle),
        "latitude of 2nd standard parallel" | "standard parallel 2" => ("lat_2", Quantity::Angle),
        "latitude of pseudo standard parallel" | "latitude of standard parallel" => {
            ("lat_ts", Quantity::Angle)
        }
        "azimuth of initial line" | "azimuth" => ("alpha", Quantity::Angle),
        "angle from rectified to skewed grid" => ("gamma", Quantity::Angle),
        "rectified grid angle" => ("gamma", Quantity::Angle),
        _ => return Err(format!("parameter `{name}` is not built")),
    };
    Ok(key)
}

fn norm(name: &str) -> String {
    let mut out = String::new();
    let mut space = false;
    for ch in name.chars() {
        let ch = if ch == '_' || ch == '-' {
            ' '
        } else {
            ch.to_ascii_lowercase()
        };
        if ch.is_ascii_whitespace() {
            if !space && !out.is_empty() {
                out.push(' ');
            }
            space = true;
        } else {
            space = false;
            out.push(ch);
        }
    }
    out
}

#[derive(Clone, Copy)]
enum Quantity {
    Angle,
    Length,
    Scale,
}

struct Shape {
    a: f64,
    rf: Option<f64>,
    b: Option<f64>,
}

impl Shape {
    fn new(a: f64, rf: Option<f64>, b: Option<f64>) -> Option<Self> {
        if !a.is_finite() || a <= 0.0 {
            return None;
        }
        if rf.is_none() && b.is_none() {
            return None;
        }
        Some(Self { a, rf, b })
    }

    fn clause(&self) -> String {
        if let Some(rf) = self.rf.filter(|rf| *rf > 0.0) {
            format!("+a={} +rf={}", format_number(self.a), format_number(rf))
        } else if let Some(b) = self.b {
            format!("+a={} +b={}", format_number(self.a), format_number(b))
        } else {
            format!("+a={} +b={}", format_number(self.a), format_number(self.a))
        }
    }
}

fn ellipsoid_of(node: &Node) -> Result<Shape, String> {
    let spheroid = find_one(node, &["SPHEROID", "ELLIPSOID"])
        .ok_or_else(|| "CRS has no ellipsoid".to_owned())?;
    let numbers = numbers_of(spheroid);
    let a = *numbers
        .first()
        .ok_or_else(|| "ellipsoid has no semi-major axis".to_owned())?;
    let rf = numbers.get(1).copied();
    Shape::new(a, rf, None).ok_or_else(|| "ellipsoid axes are not usable".to_owned())
}

fn greenwich(node: &Node) -> Result<(), String> {
    let Some(prime) = find_one(node, &["PRIMEM", "PRIMEMERIDIAN"]) else {
        return Ok(());
    };
    let Some(longitude) = numbers_of(prime).first().copied() else {
        return Ok(());
    };
    if longitude.abs() > 1e-8 {
        return Err(format!("prime meridian {longitude} is not built"));
    }
    Ok(())
}

fn parameter_number(node: &Node) -> Result<(f64, Option<&Node>), String> {
    let mut number = None;
    let mut unit = None;
    let mut seen_name = false;
    for item in &node.values {
        match item {
            Item::String(_) if !seen_name => seen_name = true,
            Item::Number(value) if number.is_none() => number = Some(*value),
            Item::Node(child) if number.is_some() && unit.is_none() => unit = Some(child),
            _ => {}
        }
    }
    number
        .map(|value| (value, unit))
        .ok_or_else(|| "parameter has no value".to_owned())
}

fn apply_unit(value: f64, quantity: Quantity, unit: Option<&Node>) -> Result<f64, String> {
    let Some(unit) = unit else {
        return Ok(value);
    };
    let factor = numbers_of(unit)
        .first()
        .copied()
        .ok_or_else(|| "parameter unit has no factor".to_owned())?;
    scale_quantity(value, quantity, factor)
}

fn apply_json_unit(value: f64, quantity: Quantity, unit: &Value) -> Result<f64, String> {
    let factor = match unit {
        Value::Number(factor) => *factor,
        Value::String(name) => named_factor(name)?,
        Value::Object(object) => {
            if let Some(factor) = object.get("conversion_factor").and_then(Value::as_f64) {
                factor
            } else if let Some(name) = object.get("name").and_then(Value::as_str) {
                named_factor(name)?
            } else {
                return Err("parameter unit is not built".to_owned());
            }
        }
        _ => return Err("parameter unit is not built".to_owned()),
    };
    scale_quantity(value, quantity, factor)
}

fn named_factor(name: &str) -> Result<f64, String> {
    match norm(name).as_str() {
        "degree" | "degrees" | "deg" => Ok(DEG_TO_RAD),
        "radian" | "radians" | "rad" => Ok(1.0),
        "grad" | "grads" | "gon" => Ok(PI / 200.0),
        "metre" | "meter" | "m" | "unity" => Ok(1.0),
        _ => Err(format!("parameter unit `{name}` is not built")),
    }
}

fn scale_quantity(value: f64, quantity: Quantity, factor: f64) -> Result<f64, String> {
    if !factor.is_finite() || factor == 0.0 {
        return Err("parameter unit is not built".to_owned());
    }
    let scaled = match quantity {
        Quantity::Angle => value * factor / DEG_TO_RAD,
        Quantity::Length | Quantity::Scale => value * factor,
    };
    if scaled.is_finite() {
        Ok(scaled)
    } else {
        Err("parameter value is not finite".to_owned())
    }
}

fn format_number(value: f64) -> String {
    let mut text = format!("{value:.15}");
    if let Some(dot) = text.find('.') {
        while text.ends_with('0') && text.len() > dot + 2 {
            text.pop();
        }
    }
    text
}

struct Node {
    keyword: String,
    values: Vec<Item>,
}

enum Item {
    String(String),
    Number(f64),
    Node(Node),
}

fn has_keyword(node: &Node, keyword: &str) -> bool {
    if node.keyword.eq_ignore_ascii_case(keyword) {
        return true;
    }
    node.values.iter().any(|item| match item {
        Item::Node(child) => has_keyword(child, keyword),
        _ => false,
    })
}

fn find_one<'a>(node: &'a Node, keywords: &[&str]) -> Option<&'a Node> {
    for keyword in keywords {
        let found = find_all(node, keyword);
        if let Some(first) = found.first() {
            return Some(*first);
        }
    }
    None
}

fn find_all<'a>(node: &'a Node, keyword: &str) -> Vec<&'a Node> {
    let mut out = Vec::new();
    collect(node, keyword, &mut out);
    out
}

fn collect<'a>(node: &'a Node, keyword: &str, out: &mut Vec<&'a Node>) {
    if node.keyword.eq_ignore_ascii_case(keyword) {
        out.push(node);
    }
    for item in &node.values {
        if let Item::Node(child) = item {
            collect(child, keyword, out);
        }
    }
}

fn first_string(node: &Node) -> Option<&str> {
    node.values.iter().find_map(|item| match item {
        Item::String(text) => Some(text.as_str()),
        _ => None,
    })
}

fn numbers_of(node: &Node) -> Vec<f64> {
    node.values
        .iter()
        .filter_map(|item| match item {
            Item::Number(value) => Some(*value),
            _ => None,
        })
        .collect()
}

fn parse_wkt(text: &str) -> Result<Node, String> {
    let mut parser = WktParser { text, index: 0 };
    let node = parser.node()?;
    parser.skip();
    if parser.index != parser.text.len() {
        return Err("WKT has trailing text".to_owned());
    }
    Ok(node)
}

struct WktParser<'a> {
    text: &'a str,
    index: usize,
}

impl<'a> WktParser<'a> {
    fn skip(&mut self) {
        while self.text[self.index..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_whitespace())
        {
            self.index += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.skip();
        self.text[self.index..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.text[self.index..].chars().next()?;
        self.index += ch.len_utf8();
        Some(ch)
    }

    fn node(&mut self) -> Result<Node, String> {
        self.skip();
        let start = self.index;
        while self.text[self.index..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            self.index += 1;
        }
        if self.index == start {
            return Err("WKT is missing a keyword".to_owned());
        }
        let keyword = self.text[start..self.index].to_owned();
        self.skip();
        if self.bump() != Some('[') {
            return Err(format!("WKT keyword {keyword} has no body"));
        }
        let mut values = Vec::new();
        self.skip();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(Node { keyword, values });
        }
        loop {
            values.push(self.item()?);
            self.skip();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    break;
                }
                _ => return Err(format!("WKT keyword {keyword} is not closed")),
            }
        }
        Ok(Node { keyword, values })
    }

    fn item(&mut self) -> Result<Item, String> {
        match self.peek() {
            Some('"') => Ok(Item::String(self.string()?)),
            Some(ch) if ch.is_ascii_alphabetic() => Ok(Item::Node(self.node()?)),
            Some(ch) if ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.' => {
                Ok(Item::Number(self.number()?))
            }
            _ => Err("WKT value is not a string, number, or node".to_owned()),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        if self.bump() != Some('"') {
            return Err("WKT string is not quoted".to_owned());
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => {
                    if self.text[self.index..].starts_with('"') {
                        self.index += 1;
                        out.push('"');
                    } else {
                        return Ok(out);
                    }
                }
                Some(ch) => out.push(ch),
                None => return Err("WKT string is not closed".to_owned()),
            }
        }
    }

    fn number(&mut self) -> Result<f64, String> {
        self.skip();
        let start = self.index;
        if matches!(self.text[self.index..].chars().next(), Some('+' | '-')) {
            self.index += 1;
        }
        let digits = self.index;
        while self.text[self.index..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            self.index += 1;
        }
        if self.index == digits {
            return Err("WKT number is empty".to_owned());
        }
        if self.text[self.index..].starts_with('.') {
            self.index += 1;
            let fraction = self.index;
            while self.text[self.index..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
            {
                self.index += 1;
            }
            if self.index == fraction {
                return Err("WKT number is empty".to_owned());
            }
        }
        if self.text[self.index..]
            .chars()
            .next()
            .is_some_and(|ch| ch == 'e' || ch == 'E')
        {
            let mark = self.index;
            self.index += 1;
            if matches!(self.text[self.index..].chars().next(), Some('+' | '-')) {
                self.index += 1;
            }
            let exp = self.index;
            while self.text[self.index..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
            {
                self.index += 1;
            }
            if self.index == exp {
                self.index = mark;
            }
        }
        self.text[start..self.index]
            .parse()
            .map_err(|_| "WKT number is not finite".to_owned())
    }
}

#[derive(Clone)]
#[expect(
    dead_code,
    reason = "JSON literals are accepted so a PROJJSON document can be parsed"
)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Object),
}

#[derive(Clone)]
struct Object(Vec<(String, Value)>);

impl Object {
    fn get(&self, key: &str) -> Option<&Value> {
        self.0
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }
}

impl Value {
    fn as_object(&self) -> Option<&Object> {
        match self {
            Self::Object(object) => Some(object),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[Value]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::String(text) => text.parse().ok(),
            _ => None,
        }
    }
}

fn parse_json(text: &str) -> Result<Value, String> {
    let mut parser = JsonParser {
        bytes: text.as_bytes(),
        index: 0,
    };
    let value = parser.value(0)?;
    parser.skip();
    if parser.index != parser.bytes.len() {
        return Err("PROJJSON has trailing text".to_owned());
    }
    Ok(value)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn skip(&mut self) {
        while self
            .bytes
            .get(self.index)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.index += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.bytes.get(self.index).copied()
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > 32 {
            return Err("PROJJSON is too deep".to_owned());
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err("PROJJSON value is not valid".to_owned()),
        }
    }

    fn literal(&mut self, text: &[u8], value: Value) -> Result<Value, String> {
        self.skip();
        if self
            .bytes
            .get(self.index..)
            .is_some_and(|rest| rest.starts_with(text))
        {
            self.index += text.len();
            Ok(value)
        } else {
            Err("PROJJSON literal is not valid".to_owned())
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.eat(b'{')?;
        let mut pairs = Vec::new();
        if self.peek() == Some(b'}') {
            self.index += 1;
            return Ok(Value::Object(Object(pairs)));
        }
        loop {
            let key = self.string()?;
            self.eat(b':')?;
            let value = self.value(depth + 1)?;
            pairs.push((key, value));
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                _ => return Err("PROJJSON object is not closed".to_owned()),
            }
        }
        Ok(Value::Object(Object(pairs)))
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.index += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                _ => return Err("PROJJSON array is not closed".to_owned()),
            }
        }
        Ok(Value::Array(items))
    }

    fn eat(&mut self, expected: u8) -> Result<(), String> {
        if self.peek() == Some(expected) {
            self.index += 1;
            Ok(())
        } else {
            Err("PROJJSON syntax is not valid".to_owned())
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out = String::new();
        loop {
            let byte = *self
                .bytes
                .get(self.index)
                .ok_or_else(|| "PROJJSON string is not closed".to_owned())?;
            self.index += 1;
            match byte {
                b'"' => return Ok(out),
                b'\\' => {
                    let escaped = *self
                        .bytes
                        .get(self.index)
                        .ok_or_else(|| "PROJJSON string is not closed".to_owned())?;
                    self.index += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => out.push(char::from(escaped)),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hex = self
                                .bytes
                                .get(self.index..self.index + 4)
                                .ok_or_else(|| "PROJJSON string escape is short".to_owned())?;
                            self.index += 4;
                            let text = std::str::from_utf8(hex)
                                .map_err(|_| "PROJJSON string escape is not hex".to_owned())?;
                            let code = u32::from_str_radix(text, 16)
                                .map_err(|_| "PROJJSON string escape is not hex".to_owned())?;
                            let ch = char::from_u32(code).ok_or_else(|| {
                                "PROJJSON string escape is not a character".to_owned()
                            })?;
                            out.push(ch);
                        }
                        _ => return Err("PROJJSON string escape is not valid".to_owned()),
                    }
                }
                0x20..=0x7e => out.push(char::from(byte)),
                _ => {
                    self.index -= 1;
                    let rest = std::str::from_utf8(
                        self.bytes
                            .get(self.index..)
                            .ok_or_else(|| "PROJJSON string is not closed".to_owned())?,
                    )
                    .map_err(|_| "PROJJSON string is not UTF-8".to_owned())?;
                    let ch = rest
                        .chars()
                        .next()
                        .ok_or_else(|| "PROJJSON string is not closed".to_owned())?;
                    out.push(ch);
                    self.index += ch.len_utf8();
                }
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        self.skip();
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }
        let digits = self.index;
        while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
            self.index += 1;
        }
        if self.index == digits {
            return Err("PROJJSON number is empty".to_owned());
        }
        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            let fraction = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if self.index == fraction {
                return Err("PROJJSON number is empty".to_owned());
            }
        }
        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+' | b'-')) {
                self.index += 1;
            }
            let exp = self.index;
            while self.bytes.get(self.index).is_some_and(u8::is_ascii_digit) {
                self.index += 1;
            }
            if self.index == exp {
                return Err("PROJJSON number is empty".to_owned());
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.index])
            .map_err(|_| "PROJJSON number is not UTF-8".to_owned())?;
        let value = text
            .parse()
            .map_err(|_| "PROJJSON number is not finite".to_owned())?;
        Ok(Value::Number(value))
    }
}
