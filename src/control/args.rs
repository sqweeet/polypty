use anyhow::{bail, Context, Result};

use super::ControlRequest;

pub(super) struct Invocation {
    pub(super) request: ControlRequest,
    pub(super) json: bool,
}

pub(super) fn parse(args: &[String]) -> Result<Invocation> {
    let Some((command, rest)) = args.split_first() else {
        bail!("missing mux command");
    };
    match command.as_str() {
        "ping" => simple(ControlRequest::Ping, rest),
        "list-sessions" | "ls" | "sessions" => simple(ControlRequest::ListSessions, rest),
        "list-tabs" | "list-windows" | "l" => simple(ControlRequest::ListTabs, rest),
        "new-tab" | "new-window" | "neww" => simple(ControlRequest::NewTab, rest),
        "select-tab" | "select-window" | "selectw" => {
            target(rest, |target| ControlRequest::SelectTab { target })
        }
        "close-tab" | "kill-window" | "killw" => {
            target(rest, |target| ControlRequest::CloseTab { target })
        }
        "capture-pane" | "capturep" => capture(rest),
        "send-keys" | "send" => send_keys(rest),
        _ => bail!("unknown mux command `{command}`; run `mux --help`"),
    }
}

fn simple(request: ControlRequest, args: &[String]) -> Result<Invocation> {
    let json = only_json(args)?;
    Ok(Invocation { request, json })
}

fn target(args: &[String], build: impl FnOnce(String) -> ControlRequest) -> Result<Invocation> {
    let mut json = false;
    let mut value = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => json = true,
            "--target" | "-t" => {
                if value.is_some() {
                    bail!("tab target was provided more than once");
                }
                value = Some(option_value(args, &mut index)?.to_owned());
            }
            arg if value.is_none() => value = Some(arg.to_owned()),
            arg => bail!("unexpected argument `{arg}`"),
        }
        index += 1;
    }
    let target = value.context("missing tab target (use an index, `active`, or `@ID`)")?;
    Ok(Invocation {
        request: build(target),
        json,
    })
}

fn capture(args: &[String]) -> Result<Invocation> {
    let mut tab = None;
    let mut pane = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => json = true,
            "--tab" | "-t" => tab = Some(option_value(args, &mut index)?.to_owned()),
            "--pane" | "-p" => pane = Some(parse_pane(option_value(args, &mut index)?)?),
            value => bail!("unexpected capture-pane argument `{value}`"),
        }
        index += 1;
    }
    Ok(Invocation {
        request: ControlRequest::CapturePane { tab, pane },
        json,
    })
}

fn send_keys(args: &[String]) -> Result<Invocation> {
    let mut tab = None;
    let mut pane = None;
    let mut enter = false;
    let mut json = false;
    let mut text = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => json = true,
            "--enter" => enter = true,
            "--tab" | "-t" => tab = Some(option_value(args, &mut index)?.to_owned()),
            "--pane" | "-p" => pane = Some(parse_pane(option_value(args, &mut index)?)?),
            "--" => {
                text.extend_from_slice(&args[index + 1..]);
                break;
            }
            value if value.starts_with('-') => bail!("unknown send-keys option `{value}`"),
            value => text.push(value.to_owned()),
        }
        index += 1;
    }
    if text.is_empty() && !enter {
        bail!("send-keys needs text or --enter");
    }
    Ok(Invocation {
        request: ControlRequest::SendKeys {
            tab,
            pane,
            text: text.join(" "),
            enter,
        },
        json,
    })
}

fn only_json(args: &[String]) -> Result<bool> {
    if args.iter().all(|arg| arg == "--json") {
        return Ok(!args.is_empty());
    }
    bail!("unexpected argument `{}`", args.join(" "))
}

fn option_value<'a>(args: &'a [String], index: &mut usize) -> Result<&'a str> {
    *index += 1;
    args.get(*index)
        .map(String::as_str)
        .context("missing option value")
}

fn parse_pane(value: &str) -> Result<u64> {
    value
        .trim_start_matches('%')
        .parse()
        .with_context(|| format!("invalid pane id `{value}`"))
}
