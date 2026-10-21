//! The few strings the backend shows itself (tray menu, notifications). The
//! rest of the UI is translated in the frontend.

use rust_decimal::Decimal;

use crate::alerts::{Condition, Trigger};

pub struct Strings {
    pub open: &'static str,
    pub sync_now: &'static str,
    pub hide_balances: &'static str,
    pub quit: &'static str,
}

const EN: Strings = Strings {
    open: "Open Chainfolio",
    sync_now: "Sync now",
    hide_balances: "Hide balances",
    quit: "Quit",
};

const PT_BR: Strings = Strings {
    open: "Abrir Chainfolio",
    sync_now: "Sincronizar agora",
    hide_balances: "Ocultar saldos",
    quit: "Sair",
};

pub fn strings(locale: &str) -> &'static Strings {
    match locale {
        "pt-BR" => &PT_BR,
        _ => &EN,
    }
}

/// Title and body of the notification for one alert trigger. `wallet` is the
/// label of a balance-change wallet; `hidden` leaves amounts out (privacy mode).
pub fn notification(
    locale: &str,
    trigger: &Trigger,
    wallet: Option<&str>,
    hidden: bool,
) -> (String, String) {
    let pt = locale == "pt-BR";
    let value = trigger.value;
    match &trigger.condition {
        Condition::PriceAbove { symbol, price, .. }
        | Condition::PriceBelow { symbol, price, .. } => {
            let above = matches!(trigger.condition, Condition::PriceAbove { .. });
            let title = match (pt, above) {
                (false, true) => format!("{symbol} above {}", usd(*price, locale)),
                (false, false) => format!("{symbol} below {}", usd(*price, locale)),
                (true, true) => format!("{symbol} acima de {}", usd(*price, locale)),
                (true, false) => format!("{symbol} abaixo de {}", usd(*price, locale)),
            };
            (title, trading_at(pt, symbol, value, locale))
        }
        Condition::PortfolioChange { ratio, .. } => {
            let change = percent(value.abs(), locale);
            let up = value >= Decimal::ZERO;
            let title = match (pt, up) {
                (false, true) => format!("Portfolio up {change} in 24h"),
                (false, false) => format!("Portfolio down {change} in 24h"),
                (true, true) => format!("Portfólio subiu {change} em 24h"),
                (true, false) => format!("Portfólio caiu {change} em 24h"),
            };
            let threshold = percent(*ratio, locale);
            let body = if pt {
                format!("Seu alerta foi definido em {threshold}.")
            } else {
                format!("Your alert was set at {threshold}.")
            };
            (title, body)
        }
        Condition::BalanceChange { .. } => {
            let wallet = wallet.unwrap_or("?");
            let incoming = value >= Decimal::ZERO;
            let title = match (pt, incoming) {
                (false, true) => format!("Funds received in {wallet}"),
                (false, false) => format!("Funds sent from {wallet}"),
                (true, true) => format!("Fundos recebidos em {wallet}"),
                (true, false) => format!("Fundos enviados de {wallet}"),
            };
            let amount = usd(value.abs(), locale);
            let body = match (pt, hidden, incoming) {
                (false, true, _) => "Open Chainfolio to see the details.".to_string(),
                (true, true, _) => "Abra o Chainfolio para ver os detalhes.".to_string(),
                (false, false, true) => format!("{amount} arrived since the last sync."),
                (false, false, false) => format!("{amount} left since the last sync."),
                (true, false, true) => format!("{amount} entraram desde a última sincronização."),
                (true, false, false) => format!("{amount} saíram desde a última sincronização."),
            };
            (title, body)
        }
        Condition::Depeg { .. } => {
            let symbol = trigger.subject.as_deref().unwrap_or("?");
            let title = if pt {
                format!("{symbol} perdeu a paridade")
            } else {
                format!("{symbol} lost its peg")
            };
            (title, trading_at(pt, symbol, value, locale))
        }
    }
}

/// One notification standing in for many triggers of the same sync.
pub fn notification_summary(locale: &str, titles: &[String]) -> (String, String) {
    let title = if locale == "pt-BR" {
        format!("{} alertas disparados", titles.len())
    } else {
        format!("{} alerts fired", titles.len())
    };
    (title, titles.join(" · "))
}

fn trading_at(pt: bool, symbol: &str, price: Decimal, locale: &str) -> String {
    if pt {
        format!("{symbol} está cotado a {}.", usd(price, locale))
    } else {
        format!("{symbol} is trading at {}.", usd(price, locale))
    }
}

/// "$1,234.56" / "US$ 1.234,56"; four decimals below $1 so depegs show.
pub fn usd(value: Decimal, locale: &str) -> String {
    let places = if value.abs() < Decimal::ONE { 4 } else { 2 };
    let (prefix, group, point) = if locale == "pt-BR" {
        ("US$ ", '.', ',')
    } else {
        ("$", ',', '.')
    };
    let sign = if value < Decimal::ZERO { "-" } else { "" };
    let digits = format!("{:.*}", places, value.abs().round_dp(places as u32));
    let (int, frac) = digits.split_once('.').unwrap_or((&digits, ""));
    format!("{sign}{prefix}{}{point}{frac}", grouped(int, group))
}

/// A ratio as a percentage with one decimal: 0.125 -> "12.5%" / "12,5%".
pub fn percent(ratio: Decimal, locale: &str) -> String {
    let text = (ratio * Decimal::ONE_HUNDRED)
        .round_dp(1)
        .normalize()
        .to_string();
    let text = if locale == "pt-BR" {
        text.replace('.', ",")
    } else {
        text
    };
    format!("{text}%")
}

fn grouped(int: &str, separator: char) -> String {
    let mut out = String::with_capacity(int.len() + int.len() / 3);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(separator);
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alerts::Direction;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    #[test]
    fn formats_money_and_percentages_per_locale() {
        assert_eq!(usd(d("1234567.891"), "en"), "$1,234,567.89");
        assert_eq!(usd(d("1234567.891"), "pt-BR"), "US$ 1.234.567,89");
        assert_eq!(usd(d("0.97512"), "en"), "$0.9751");
        assert_eq!(usd(d("-250"), "en"), "-$250.00");
        assert_eq!(percent(d("0.125"), "en"), "12.5%");
        assert_eq!(percent(d("0.1"), "pt-BR"), "10%");
    }

    #[test]
    fn balance_notifications_respect_privacy() {
        let trigger = Trigger {
            rule_id: 1,
            condition: Condition::BalanceChange {
                wallet_id: None,
                direction: Direction::Either,
                usd: d("1000"),
            },
            value: d("-5000"),
            subject: Some("3".into()),
        };
        let (title, body) = notification("en", &trigger, Some("Cold"), false);
        assert_eq!(title, "Funds sent from Cold");
        assert_eq!(body, "$5,000.00 left since the last sync.");
        let (_, body) = notification("pt-BR", &trigger, Some("Cold"), true);
        assert!(!body.contains("5.000"));
    }
}
