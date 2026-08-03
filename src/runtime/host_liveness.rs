use std::io::{stdin, stdout, IsTerminal};

pub(super) struct HostLiveness;

impl HostLiveness {
    pub(super) fn attached() -> bool {
        if !stdin().is_terminal() || !stdout().is_terminal() {
            return false;
        }
        #[cfg(unix)]
        return !descriptors_hung_up();

        #[cfg(not(unix))]
        true
    }
}

#[cfg(unix)]
fn descriptors_hung_up() -> bool {
    let mut descriptors = [
        libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: libc::STDOUT_FILENO,
            events: 0,
            revents: 0,
        },
    ];
    let result = unsafe { libc::poll(descriptors.as_mut_ptr(), descriptors.len() as _, 0) };
    result > 0
        && descriptors
            .iter()
            .any(|descriptor| terminal_gone(descriptor.revents))
}

#[cfg(unix)]
fn terminal_gone(events: libc::c_short) -> bool {
    events & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0
}

#[cfg(all(test, unix))]
mod tests {
    use super::terminal_gone;

    #[test]
    fn hangup_error_and_invalid_descriptor_mean_detached() {
        assert!(terminal_gone(libc::POLLHUP));
        assert!(terminal_gone(libc::POLLERR));
        assert!(terminal_gone(libc::POLLNVAL));
        assert!(terminal_gone(libc::POLLIN | libc::POLLHUP));
    }

    #[test]
    fn ordinary_input_does_not_look_detached() {
        assert!(!terminal_gone(0));
        assert!(!terminal_gone(libc::POLLIN));
    }
}
