use super::Tab;

impl Tab {
    pub fn try_reap(&mut self) -> bool {
        let reaped = self.transport.try_reap();
        if self.transport.child_exited() {
            self.alive = false;
        }
        reaped
    }

    pub fn kill(&mut self) {
        self.transport.kill();
        self.alive = false;
    }
}
