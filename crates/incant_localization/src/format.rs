use crate::message::Node;
use crate::{
    CalendarDate, DateLength, LocalizationError, MAX_OUTPUT_BYTES, MessageArgument,
    MessageArguments,
};
use icu_datetime::{DateTimeFormatter, fieldsets::YMD, input::Date};
use icu_decimal::{DecimalFormatter, input::Decimal};
use icu_plurals::{PluralCategory, PluralRules};
use writeable::Writeable;

pub fn format_number(locale: &str, value: f64) -> Result<String, LocalizationError> {
    valid_number(value)?;
    let number: Decimal = value
        .to_string()
        .parse()
        .map_err(|_| LocalizationError::Format("decimal outside formatter limits".into()))?;
    let formatter =
        DecimalFormatter::try_new(crate::types::locale(locale)?.into(), Default::default())
            .map_err(|e| LocalizationError::Format(e.to_string()))?;
    Ok(formatter.format(&number).write_to_string().into_owned())
}
pub fn format_date(
    locale: &str,
    value: &CalendarDate,
    length: DateLength,
) -> Result<String, LocalizationError> {
    if !(-9999..=9999).contains(&value.year) {
        return Err(LocalizationError::Argument(
            "date year outside -9999..9999".into(),
        ));
    }
    let date = Date::try_new_iso(value.year, value.month, value.day)
        .map_err(|e| LocalizationError::Argument(e.to_string()))?;
    let fields = match length {
        DateLength::Short => YMD::short(),
        DateLength::Medium => YMD::medium(),
        DateLength::Long => YMD::long(),
    };
    let formatter = DateTimeFormatter::try_new(crate::types::locale(locale)?.into(), fields)
        .map_err(|e| LocalizationError::Format(e.to_string()))?;
    Ok(formatter.format(&date).write_to_string().into_owned())
}
pub(crate) fn valid_number(number: f64) -> Result<(), LocalizationError> {
    if !number.is_finite() || number.abs() > 9_007_199_254_740_991. {
        return Err(LocalizationError::Argument(
            "number must be finite and within JavaScript's safe integer range".into(),
        ));
    }
    Ok(())
}
fn number(arguments: &MessageArguments, name: &str) -> Result<f64, LocalizationError> {
    if let Some(MessageArgument::Number(value)) = arguments.get(name) {
        valid_number(*value)?;
        Ok(*value)
    } else {
        Err(LocalizationError::Argument(name.into()))
    }
}
fn append(out: &mut String, text: &str) -> Result<(), LocalizationError> {
    if out.len() + text.len() > MAX_OUTPUT_BYTES {
        return Err(LocalizationError::OutputLimit);
    }
    out.push_str(text);
    Ok(())
}
fn pseudo(text: &str) -> String {
    let plain = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let accented: Vec<char> = "áƀçďéƒğĥíĵķľɱñóþɋŕšţúṽŵẋýžÁɃÇĎÉƑĞĤÍĴĶĽṀÑÓÞɊŔŠŢÚṼŴẊÝŽ"
        .chars()
        .collect();
    let mut result = String::new();
    for c in text.chars() {
        if let Some(i) = plain.find(c) {
            result.push(accented[i]);
            if "aeiouAEIOU".contains(c) {
                result.push(accented[i]);
            }
        } else {
            result.push(c);
        }
    }
    result
}
pub(crate) fn evaluate(
    nodes: &[Node],
    locale: &str,
    args: &MessageArguments,
    use_pseudo: bool,
    pound: Option<f64>,
    out: &mut String,
) -> Result<(), LocalizationError> {
    for node in nodes {
        match node {
            Node::Text(text) => append(
                out,
                &if use_pseudo {
                    pseudo(text)
                } else {
                    text.clone()
                },
            )?,
            Node::Pound => append(
                out,
                &format_number(
                    locale,
                    pound
                        .ok_or_else(|| LocalizationError::Format("pound outside plural".into()))?,
                )?,
            )?,
            Node::Argument(name) => match args.get(name) {
                Some(MessageArgument::Text(text)) => append(out, text)?,
                Some(MessageArgument::Number(value)) => {
                    append(out, &format_number(locale, *value)?)?
                }
                Some(MessageArgument::Date(date)) => {
                    append(out, &format_date(locale, date, DateLength::Medium)?)?
                }
                None => return Err(LocalizationError::Argument(name.clone())),
            },
            Node::Number(name) => append(out, &format_number(locale, number(args, name)?)?)?,
            Node::Date(name, length) => {
                if let Some(MessageArgument::Date(date)) = args.get(name) {
                    append(out, &format_date(locale, date, *length)?)?;
                } else {
                    return Err(LocalizationError::Argument(name.clone()));
                }
            }
            Node::Select { name, branches } => {
                let Some(MessageArgument::Text(value)) = args.get(name) else {
                    return Err(LocalizationError::Argument(name.clone()));
                };
                evaluate(
                    branches.get(value).unwrap_or(&branches["other"]),
                    locale,
                    args,
                    use_pseudo,
                    pound,
                    out,
                )?;
            }
            Node::Plural {
                name,
                ordinal,
                offset,
                exact,
                branches,
            } => {
                let value = number(args, name)?;
                let adjusted = value - f64::from(*offset);
                let exact = exact
                    .iter()
                    .find(|(n, _)| *n == value)
                    .map(|(_, nodes)| nodes);
                let selected = if let Some(exact) = exact {
                    exact
                } else {
                    let prefs = crate::types::locale(locale)?.into();
                    let rules = if *ordinal {
                        PluralRules::try_new_ordinal(prefs)
                    } else {
                        PluralRules::try_new_cardinal(prefs)
                    }
                    .map_err(|e| LocalizationError::Format(e.to_string()))?;
                    let operands: Decimal = adjusted
                        .to_string()
                        .parse()
                        .map_err(|_| LocalizationError::Argument(name.clone()))?;
                    let category = match rules.category_for(&operands) {
                        PluralCategory::Zero => "zero",
                        PluralCategory::One => "one",
                        PluralCategory::Two => "two",
                        PluralCategory::Few => "few",
                        PluralCategory::Many => "many",
                        PluralCategory::Other => "other",
                    };
                    branches.get(category).unwrap_or(&branches["other"])
                };
                evaluate(selected, locale, args, use_pseudo, Some(adjusted), out)?;
            }
        }
    }
    Ok(())
}
