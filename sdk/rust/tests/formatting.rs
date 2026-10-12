use incant_module::localization::{CalendarDate, DateLength};
use incant_module::{__private, Result};
#[test]
fn ignored_invalid_primitives_refuse_callback_before_any_import() {
    for value in [f64::NAN, f64::INFINITY, 9_007_199_254_740_992.] {
        assert!(
            std::panic::catch_unwind(|| __private::run(1, 0.1, |tick, _| -> Result<()> {
                let _ignored = tick.format_number(value, &mut [0; 128]);
                Ok(())
            }))
            .is_err()
        );
    }
    assert!(
        std::panic::catch_unwind(|| __private::run(1, 0.1, |tick, _| -> Result<()> {
            let _ignored = tick.format_date(
                CalendarDate {
                    year: 10000,
                    month: 1,
                    day: 1,
                },
                DateLength::Medium,
                &mut [0; 128],
            );
            Ok(())
        }))
        .is_err()
    );
}
