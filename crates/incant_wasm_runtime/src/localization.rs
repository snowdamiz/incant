//! Per-epoch queries see an immutable committed catalog/settings snapshot.
//! Settings changes are ordinary structural commands, never a second publisher.
use crate::BoundaryError as Error;
use incant_localization::{
    CalendarDate, LocaleSettings, LocalizeRequest, MessageArgument, MissingKind,
};
use incant_runtime::{NativeLocalization, RuntimeError, SceneError};
use incant_wasm_abi::{self as abi, localization as wire};

pub(crate) struct State {
    snapshot: NativeLocalization,
    queries: usize,
    units: usize,
    input: usize,
    output: usize,
    commands: usize,
    reply: Option<Vec<u8>>,
    pub rejected: bool,
}
impl State {
    pub fn new(snapshot: NativeLocalization) -> Self {
        Self {
            snapshot,
            queries: 0,
            units: 0,
            input: 0,
            output: 0,
            commands: 0,
            reply: None,
            rejected: false,
        }
    }
    pub fn reject(&mut self) {
        self.rejected = true;
        self.reply = None;
    }
    fn query(&mut self) -> Result<(), Error> {
        self.reply = None;
        if self.rejected {
            return Err(Error::Abi("rejected localization transaction"));
        }
        charge(
            &mut self.queries,
            1,
            abi::LOCALIZATION_MAX_QUERIES,
            "localization query quota",
        )?;
        charge(
            &mut self.units,
            abi::LOCALIZATION_QUERY_UNITS,
            abi::NATIVE_MAX_QUERY_UNITS,
            "query units",
        )
    }
    fn publish(
        &mut self,
        logical: usize,
        encode: impl FnOnce(&mut [u8]) -> wire::Result<usize>,
    ) -> Result<i32, Error> {
        charge(
            &mut self.output,
            logical,
            abi::LOCALIZATION_MAX_OUTPUT_BYTES,
            "localization output quota",
        )?;
        let mut bytes = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let len = encode(&mut bytes).map_err(codec)?;
        let result = i32::try_from(len).map_err(|_| Error::Abi("localization result length"))?;
        bytes.truncate(len);
        self.reply = Some(bytes);
        Ok(result)
    }
    pub fn locale(&mut self) -> Result<i32, Error> {
        self.query()?;
        // Cheap Arc snapshot clone avoids borrowing self while charging the result.
        let snapshot = self.snapshot.clone();
        let settings = snapshot.settings();
        let fallbacks = settings
            .fallbacks
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let logical = settings.locale.len() + fallbacks.iter().map(|s| s.len()).sum::<usize>() + 1;
        self.publish(logical, |bytes| {
            wire::encode_locale(
                wire::LocaleSettings {
                    locale: &settings.locale,
                    fallbacks: &fallbacks,
                    pseudo: settings.pseudo,
                },
                bytes,
            )
        })
    }
    pub fn format(&mut self, bytes: &[u8]) -> Result<i32, Error> {
        self.query()?;
        let locale = &self.snapshot.settings().locale;
        let result = match wire::decode_format(bytes).map_err(codec)? {
            wire::FormatRequest::Number(value) => incant_localization::format_number(locale, value),
            wire::FormatRequest::Date(date, length) => incant_localization::format_date(
                locale,
                &CalendarDate {
                    year: date.year,
                    month: date.month,
                    day: date.day,
                },
                match length {
                    abi::DateLength::Short => incant_localization::DateLength::Short,
                    abi::DateLength::Medium => incant_localization::DateLength::Medium,
                    abi::DateLength::Long => incant_localization::DateLength::Long,
                },
            ),
        }
        .map_err(|e| {
            Error::Runtime(RuntimeError::Scene(SceneError::Localization(e.to_string())))
        })?;
        // Standalone TypeScript formatting charges returned text only: primitive
        // inputs cost no logical input bytes, but ingress/copy transfer limits apply.
        self.publish(result.len(), |bytes| wire::encode_formatted(&result, bytes))
    }
    pub fn localize(&mut self, bytes: &[u8]) -> Result<i32, Error> {
        self.query()?;
        let input = wire::decode_request(bytes).map_err(codec)?;
        charge(
            &mut self.input,
            input.input_bytes,
            abi::LOCALIZATION_MAX_INPUT_BYTES,
            "localization input quota",
        )?;
        let arguments = input
            .arguments()
            .map(|arg| {
                (
                    arg.name.to_owned(),
                    match arg.value {
                        wire::MessageArgument::Text(value) => {
                            MessageArgument::Text(value.to_owned())
                        }
                        wire::MessageArgument::Number(value) => MessageArgument::Number(value),
                        wire::MessageArgument::Date(value) => MessageArgument::Date(CalendarDate {
                            year: value.year,
                            month: value.month,
                            day: value.day,
                        }),
                    },
                )
            })
            .collect();
        let value = self
            .snapshot
            .localize(&LocalizeRequest {
                table_id: input.table_id.to_owned(),
                key: input.key.to_owned(),
                arguments,
            })
            .map_err(|e| Error::Runtime(RuntimeError::Scene(e)))?;
        // Charge the same logical metadata as QuickJS, even where the copied wire
        // format avoids repeating requested/resolved locale in MissingString.
        let logical = value.text.len()
            + value.requested_locale.len()
            + value.resolved_locale.as_ref().map_or(0, String::len)
            + value.missing.as_ref().map_or(0, |m| {
                m.table_id.len()
                    + m.key.len()
                    + m.requested_locale.len()
                    + m.resolved_locale.as_ref().map_or(0, String::len)
                    + 16
            });
        self.publish(logical, |bytes| {
            wire::encode_localized(
                wire::LocalizedText {
                    text: &value.text,
                    requested_locale: &value.requested_locale,
                    resolved_locale: value.resolved_locale.as_deref(),
                    missing: value.missing.as_ref().map(|m| wire::MissingString {
                        table_id: &m.table_id,
                        key: &m.key,
                        kind: match m.kind {
                            MissingKind::Translation => wire::MissingKind::Translation,
                            MissingKind::Key => wire::MissingKind::Key,
                        },
                    }),
                },
                bytes,
            )
        })
    }
    pub fn settings(&mut self, bytes: &[u8]) -> Result<LocaleSettings, Error> {
        if self.rejected {
            return Err(Error::Abi("rejected localization transaction"));
        }
        let settings = wire::decode_settings(bytes).map_err(codec)?;
        let settings = LocaleSettings {
            locale: settings.locale.to_owned(),
            fallbacks: settings.fallbacks().map(str::to_owned).collect(),
            pseudo: settings.pseudo,
        };
        settings.validate().map_err(|e| {
            Error::Runtime(RuntimeError::Scene(SceneError::Localization(e.to_string())))
        })?;
        charge(
            &mut self.commands,
            1,
            abi::LOCALIZATION_MAX_COMMANDS,
            "locale command quota",
        )?;
        Ok(settings)
    }
    pub fn reply(&self) -> Result<&[u8], Error> {
        if self.rejected {
            return Err(Error::Abi("rejected localization transaction"));
        }
        self.reply
            .as_deref()
            .ok_or(Error::Abi("localization result unavailable"))
    }
}
fn charge(counter: &mut usize, amount: usize, max: usize, name: &'static str) -> Result<(), Error> {
    *counter = counter
        .checked_add(amount)
        .filter(|n| *n <= max)
        .ok_or(Error::Limit(name))?;
    Ok(())
}
pub(crate) fn codec(error: wire::Error) -> Error {
    match error {
        wire::Error::Bounds => Error::Limit("localization frame bounds"),
        wire::Error::Malformed => Error::Abi("malformed localization frame"),
    }
}
