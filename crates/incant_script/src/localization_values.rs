//! Typed localization conversion. No JSON, borrowed guest storage or Rust
//! catalog/queue borrow survives a getter or result materialization.
use crate::query_values::{Kind, Request, ResultData};
use incant_localization::{
    CalendarDate, DateLength, LocaleSettings, LocalizationReportRequest, LocalizeRequest,
    MessageArgument, MissingKind, MissingString,
};
use rquickjs::{Array, Ctx, Exception, IntoJs, Object, Value, object::Property};
use std::collections::BTreeMap;

fn invalid(ctx: &Ctx<'_>) -> rquickjs::Error {
    Exception::throw_type(ctx, "invalid or excessive localization arguments")
}
fn text(ctx: &Ctx<'_>, value: &Value<'_>, max: usize) -> rquickjs::Result<String> {
    let value = value.as_string().ok_or_else(|| invalid(ctx))?;
    let bytes = value.clone().to_cstring()?;
    if bytes.len() > max {
        return Err(invalid(ctx));
    }
    drop(bytes);
    value.to_string().map_err(|_| invalid(ctx))
}
fn object<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<Object<'js>> {
    value
        .as_object()
        .filter(|_| !value.is_array())
        .cloned()
        .ok_or_else(|| invalid(ctx))
}
fn integer(ctx: &Ctx<'_>, value: &Value<'_>, min: f64, max: f64) -> rquickjs::Result<f64> {
    value
        .as_number()
        .filter(|n| n.is_finite() && n.fract() == 0. && (min..=max).contains(n))
        .ok_or_else(|| invalid(ctx))
}
fn date<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<CalendarDate> {
    let value = object(ctx, value)?;
    Ok(CalendarDate {
        year: integer(ctx, &value.get::<_, Value>("year")?, -9999., 9999.)? as i32,
        month: integer(ctx, &value.get::<_, Value>("month")?, 1., 12.)? as u8,
        day: integer(ctx, &value.get::<_, Value>("day")?, 1., 31.)? as u8,
    })
}
pub(crate) fn settings<'js>(
    ctx: &Ctx<'js>,
    value: &Value<'js>,
) -> rquickjs::Result<LocaleSettings> {
    let value = object(ctx, value)?;
    let locale = text(ctx, &value.get::<_, Value>("locale")?, 128)?;
    let raw = value.get::<_, Value>("fallbacks")?;
    let mut fallbacks = Vec::new();
    if !raw.is_undefined() {
        if !raw.is_array() {
            return Err(invalid(ctx));
        }
        let list = raw.as_object().unwrap();
        let len = integer(ctx, &list.get::<_, Value>("length")?, 0., 16.)? as u32;
        for i in 0..len {
            fallbacks.push(text(ctx, &list.get::<_, Value>(i)?, 128)?);
        }
    }
    let raw = value.get::<_, Value>("pseudo")?;
    let pseudo = if raw.is_undefined() {
        false
    } else {
        raw.as_bool().ok_or_else(|| invalid(ctx))?
    };
    let settings = LocaleSettings {
        locale,
        fallbacks,
        pseudo,
    };
    settings.validate().map_err(|_| invalid(ctx))?;
    Ok(settings)
}
fn localize<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<LocalizeRequest> {
    let value = object(ctx, value)?;
    let id = value.get::<_, Value>("table_id")?;
    let table_id = text(ctx, &id, 26)?;
    crate::commands::id(ctx, &id)?;
    let key = text(ctx, &value.get::<_, Value>("key")?, 128)?;
    let raw = value.get::<_, Value>("arguments")?;
    let mut arguments = BTreeMap::new();
    let mut bytes = 0;
    if !raw.is_undefined() {
        let values = object(ctx, &raw)?;
        for name in values.keys::<rquickjs::String>() {
            let name = text(ctx, &name?.into_value(), 128)?;
            if arguments.len() == 64 || name.len() > 128 {
                return Err(invalid(ctx));
            }
            let value = values.get::<_, Value>(name.as_str())?;
            let argument = if value.is_string() {
                let s = text(ctx, &value, 65_536 - bytes)?;
                bytes += s.len();
                MessageArgument::Text(s)
            } else if let Some(n) = value.as_number() {
                if !n.is_finite() || n.abs() > 9_007_199_254_740_991. {
                    return Err(invalid(ctx));
                }
                MessageArgument::Number(n)
            } else {
                MessageArgument::Date(date(ctx, &value)?)
            };
            arguments.insert(name, argument);
        }
    }
    Ok(LocalizeRequest {
        table_id,
        key,
        arguments,
    })
}
pub(super) fn parse<'js>(
    kind: Kind,
    ctx: &Ctx<'js>,
    args: &[Value<'js>],
) -> rquickjs::Result<Request> {
    match (kind, args) {
        (Kind::Localize, [value]) => Ok(Request::Localize(localize(ctx, value)?)),
        (Kind::Locale, []) => Ok(Request::Locale),
        (Kind::Number, [value]) => Ok(Request::Number(
            value
                .as_number()
                .filter(|n| n.is_finite() && n.abs() <= 9_007_199_254_740_991.)
                .ok_or_else(|| invalid(ctx))?,
        )),
        (Kind::Date, [value, rest @ ..]) if rest.len() <= 1 => {
            let length = if let Some(length) = rest.first().filter(|v| !v.is_undefined()) {
                match text(ctx, length, 6)?.as_str() {
                    "short" => DateLength::Short,
                    "medium" => DateLength::Medium,
                    "long" => DateLength::Long,
                    _ => return Err(invalid(ctx)),
                }
            } else {
                DateLength::Medium
            };
            Ok(Request::Date(date(ctx, value)?, length))
        }
        (Kind::Report, []) => Ok(Request::Report(LocalizationReportRequest::default())),
        (Kind::Report, [value]) => {
            let value = object(ctx, value)?;
            let offset = value.get::<_, Value>("offset")?;
            let limit = value.get::<_, Value>("limit")?;
            Ok(Request::Report(LocalizationReportRequest {
                offset: if offset.is_undefined() {
                    0
                } else {
                    integer(ctx, &offset, 0., 4096.)? as u32
                },
                limit: if limit.is_undefined() {
                    128
                } else {
                    integer(ctx, &limit, 1., 128.)? as u32
                },
            }))
        }
        _ => Err(invalid(ctx)),
    }
}
pub(super) fn input_bytes(request: &Request) -> usize {
    if let Request::Localize(r) = request {
        r.table_id.len()
            + r.key.len()
            + r.arguments
                .iter()
                .map(|(k, v)| {
                    k.len()
                        + match v {
                            MessageArgument::Text(t) => t.len(),
                            _ => 16,
                        }
                })
                .sum::<usize>()
    } else {
        0
    }
}
fn missing_bytes(m: &MissingString) -> usize {
    m.table_id.len()
        + m.key.len()
        + m.requested_locale.len()
        + m.resolved_locale.as_ref().map_or(0, String::len)
        + 16
}
pub(super) fn output_bytes(result: &ResultData) -> usize {
    match result {
        ResultData::Text(s) => s.len(),
        ResultData::Locale(s) => {
            s.locale.len() + s.fallbacks.iter().map(String::len).sum::<usize>() + 1
        }
        ResultData::Localized(s) => {
            s.text.len()
                + s.requested_locale.len()
                + s.resolved_locale.as_ref().map_or(0, String::len)
                + s.missing.as_ref().map_or(0, missing_bytes)
        }
        ResultData::Report(r) => 8 + r.entries.iter().map(missing_bytes).sum::<usize>(),
        _ => 0,
    }
}
fn nullable<'js>(ctx: &Ctx<'js>, value: Option<String>) -> rquickjs::Result<Value<'js>> {
    value.map_or_else(|| Ok(Value::new_null(ctx.clone())), |v| v.into_js(ctx))
}
fn append<'js, T: IntoJs<'js>>(array: &Array<'js>, index: usize, value: T) -> rquickjs::Result<()> {
    array.as_object().prop(
        index as u32,
        Property::from(value).enumerable().writable().configurable(),
    )
}
fn missing<'js>(ctx: &Ctx<'js>, m: MissingString) -> rquickjs::Result<Object<'js>> {
    let o = Object::new_proto(ctx.clone(), None)?;
    o.set("table_id", m.table_id)?;
    o.set("key", m.key)?;
    o.set("requested_locale", m.requested_locale)?;
    o.set("resolved_locale", nullable(ctx, m.resolved_locale)?)?;
    o.set(
        "kind",
        match m.kind {
            MissingKind::Key => "key",
            MissingKind::Translation => "translation",
        },
    )?;
    Ok(o)
}
pub(super) fn materialize<'js>(ctx: &Ctx<'js>, data: ResultData) -> rquickjs::Result<Value<'js>> {
    let o = Object::new_proto(ctx.clone(), None)?;
    match data {
        ResultData::Text(s) => return s.into_js(ctx),
        ResultData::Locale(s) => {
            o.set("locale", s.locale)?;
            o.set("pseudo", s.pseudo)?;
            let list = Array::new(ctx.clone())?;
            for (i, v) in s.fallbacks.into_iter().enumerate() {
                append(&list, i, v)?;
            }
            o.set("fallbacks", list)?;
        }
        ResultData::Localized(s) => {
            o.set("text", s.text)?;
            o.set("requested_locale", s.requested_locale)?;
            o.set("resolved_locale", nullable(ctx, s.resolved_locale)?)?;
            let m = match s.missing {
                Some(m) => missing(ctx, m)?.into_value(),
                None => Value::new_null(ctx.clone()),
            };
            o.set("missing", m)?;
        }
        ResultData::Report(r) => {
            o.set("total", r.total)?;
            let list = Array::new(ctx.clone())?;
            for (i, m) in r.entries.into_iter().enumerate() {
                append(&list, i, missing(ctx, m)?)?;
            }
            o.set("entries", list)?;
        }
        _ => unreachable!("localization result"),
    }
    Ok(o.into_value())
}
