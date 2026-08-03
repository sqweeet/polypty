use std::process::Command;

pub(super) fn read_first(commands: &[&[&str]]) -> Option<String> {
    commands.iter().find_map(|arguments| {
        let (program, arguments) = arguments.split_first()?;
        let output = Command::new(program).args(arguments).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        (!text.is_empty()).then_some(text)
    })
}
