use num_format::{Locale, ToFormattedString};

pub fn format_inr(amount: f64) -> String {
    let cents = (amount.abs() * 100.0).round() as u64;
    let whole = cents / 100;
    let decimals = cents % 100;
    let sign = if amount < 0.0 { "-" } else { "" };

    format!(
        "{}₹{}.{:02}",
        sign,
        whole.to_formatted_string(&Locale::en_IN),
        decimals
    )
}
