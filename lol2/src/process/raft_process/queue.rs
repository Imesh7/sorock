use super::*;

impl RaftProcess {
    pub async fn queue_new_entry(&self, command: Bytes, completion: Completion) -> Result<Index> {
        ensure!(self.voter.allow_queue_entry().await?);

        let append_index = self
            .command_log
            .append_new_entry(command.clone(), None)
            .await?;

        self.command_log
            .register_completion(append_index, completion);

        self.process_configuration_command(&command, append_index)
            .await?;

        Ok(append_index)
    }

    pub async fn queue_received_entry(&self, mut req: LogStream) -> Result<(bool, u64)> {
        let mut prev_clock = req.prev_clock;
        let mut n_inserted = 0;
        while let Some(cur) = req.entries.next().await {
            let entry = Entry {
                prev_clock,
                this_clock: cur.this_clock,
                command: cur.command,
            };
            let insert_index = entry.this_clock.index;
            let command = entry.command.clone();
            match self
                .command_log
                .try_insert_entry(entry, req.sender_id.clone(), self.driver.clone())
                .await?
            {
                command_log::TryInsertResult::Inserted => {
                    self.process_configuration_command(&command, insert_index)
                        .await?
                }
                command_log::TryInsertResult::SkippedInsertion => {}
                command_log::TryInsertResult::InconsistencyDetected => {
                    warn!("rejected append entry (clock={:?})", cur.this_clock);
                    return Ok((false, n_inserted));
                }
            }
            prev_clock = cur.this_clock;
            n_inserted += 1;
        }

        Ok((true, n_inserted))
    }
}
