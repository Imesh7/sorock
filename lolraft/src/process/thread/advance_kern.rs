use super::*;

#[derive(Clone)]
pub struct Thread {
    command_log: CommandLog,
    voter: Voter,
}
impl Thread {
    async fn advance_once(&self) -> Result<()> {
        self.command_log
            .advance_kern_process(self.voter.clone())
            .await
    }

    fn do_loop(self) -> ThreadHandle {
        let hdl = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                while self.advance_once().await.is_ok() {
                    tokio::task::yield_now().await;
                }
            }
        })
        .abort_handle();

        ThreadHandle(hdl)
    }
}

pub fn new(command_log: CommandLog, voter: Voter) -> ThreadHandle {
    Thread { command_log, voter }.do_loop()
}
