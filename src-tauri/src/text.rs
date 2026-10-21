//! The few strings the backend shows itself (tray menu, notifications). The
//! rest of the UI is translated in the frontend.

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
