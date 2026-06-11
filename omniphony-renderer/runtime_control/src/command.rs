use log::LevelFilter;
use rosc::{OscMessage, OscType};

#[derive(Debug, Clone)]
pub enum RuntimeCommand {
    SaveConfig,
    ReloadConfig,
    Quit,
    /// Another local instance wants the OSC RX port; only honoured by
    /// instances started with `--osc-yield` (see `sys::shutdown::is_yieldable`).
    YieldPort,
    SetLogLevel(LevelFilter),
}

pub fn parse_runtime_log_level(value: &str) -> Option<LevelFilter> {
    match value.trim().to_ascii_lowercase().as_str() {
        "off" => Some(LevelFilter::Off),
        "error" => Some(LevelFilter::Error),
        "warn" | "warning" => Some(LevelFilter::Warn),
        "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        _ => None,
    }
}

pub fn parse_process_command(msg: &OscMessage) -> Option<RuntimeCommand> {
    match msg.addr.as_str() {
        "/omniphony/control/save_config" => Some(RuntimeCommand::SaveConfig),
        "/omniphony/control/reload_config" => Some(RuntimeCommand::ReloadConfig),
        "/omniphony/control/quit" => Some(RuntimeCommand::Quit),
        "/omniphony/control/yield_port" => Some(RuntimeCommand::YieldPort),
        "/omniphony/control/log_level" => msg.args.first().and_then(|arg| match arg {
            OscType::String(s) => parse_runtime_log_level(s).map(RuntimeCommand::SetLogLevel),
            _ => None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(addr: &str) -> OscMessage {
        OscMessage {
            addr: addr.to_string(),
            args: vec![],
        }
    }

    #[test]
    fn yield_port_maps_to_runtime_command() {
        assert!(matches!(
            parse_process_command(&msg(crate::osc_contract::CONTROL_YIELD_PORT)),
            Some(RuntimeCommand::YieldPort)
        ));
        assert!(matches!(
            parse_process_command(&msg(crate::osc_contract::CONTROL_QUIT)),
            Some(RuntimeCommand::Quit)
        ));
        assert!(parse_process_command(&msg("/omniphony/control/unknown")).is_none());
    }
}
