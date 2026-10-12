use incant_wasm_abi::{self as abi, localization::*};
#[test]
fn formatting_frames_preserve_signed_numbers_all_lengths_and_exact_domains() {
    let mut scratch = [0; abi::LOCALIZATION_FORMAT_BYTES];
    for value in [
        0.,
        -0.,
        -1234.5,
        9_007_199_254_740_991.,
        -9_007_199_254_740_991.,
    ] {
        let request = FormatRequest::Number(value);
        assert_eq!(encode_format(request, &mut scratch).unwrap(), 24);
        assert_eq!(decode_format(&scratch).unwrap(), request);
        assert_eq!(&scratch[16..], &value.to_le_bytes());
    }
    for year in [-9999, 0, 2024, 9999] {
        for length in [DateLength::Short, DateLength::Medium, DateLength::Long] {
            let request = FormatRequest::Date(
                CalendarDate {
                    year,
                    month: 2,
                    day: 29,
                },
                length,
            );
            encode_format(request, &mut scratch).unwrap();
            assert_eq!(decode_format(&scratch).unwrap(), request);
        }
    }
    for value in [
        f64::NAN,
        f64::INFINITY,
        -f64::INFINITY,
        9_007_199_254_740_992.,
    ] {
        let before = scratch;
        assert!(encode_format(FormatRequest::Number(value), &mut scratch).is_err());
        assert_eq!(scratch, before);
    }
    for date in [
        CalendarDate {
            year: 10000,
            month: 1,
            day: 1,
        },
        CalendarDate {
            year: 1,
            month: 0,
            day: 1,
        },
        CalendarDate {
            year: 1,
            month: 1,
            day: 0,
        },
    ] {
        assert!(
            encode_format(FormatRequest::Date(date, DateLength::Medium), &mut scratch).is_err()
        );
    }
    let mut short = [77; 23];
    assert!(encode_format(FormatRequest::Number(1.), &mut short).is_err());
    assert_eq!(short, [77; 23]);
}
#[test]
fn exact_versioned_frames_and_result_kinds_fail_closed() {
    let mut buffer = [0; 128];
    let n = encode_format(
        FormatRequest::Date(
            CalendarDate {
                year: 2024,
                month: 2,
                day: 29,
            },
            DateLength::Long,
        ),
        &mut buffer,
    )
    .unwrap();
    for offset in [0, 8, 12, 20, 21, 22, 23] {
        let mut bad = buffer[..n].to_vec();
        bad[offset] = 255;
        assert!(decode_format(&bad).is_err(), "offset {offset}");
    }
    for n in 0..24 {
        assert!(decode_format(&buffer[..n]).is_err());
    }
    assert!(decode_format(&buffer[..25]).is_err());
    let n = encode_formatted("雪🙂\0 1\u{202f}234,5", &mut buffer).unwrap();
    assert_eq!(
        decode_formatted(&buffer[..n]).unwrap(),
        "雪🙂\0 1\u{202f}234,5"
    );
    assert!(decode_locale(&buffer[..n]).is_err());
    assert!(decode_localized(&buffer[..n]).is_err());
    assert!(decode_formatted(&buffer[..n + 1]).is_err());
    for end in 0..n {
        assert!(decode_formatted(&buffer[..end]).is_err());
    }
    buffer[20] = 255;
    assert!(decode_formatted(&buffer[..n]).is_err());
    let n = encode_locale(
        LocaleSettings {
            locale: "en",
            fallbacks: &[],
            pseudo: false,
        },
        &mut buffer,
    )
    .unwrap();
    assert!(decode_formatted(&buffer[..n]).is_err());
}
