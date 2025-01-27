use crate::pipeline::errors::PipelineError;
use dozer_core::channels::ProcessorChannelForwarder;
use dozer_core::epoch::Epoch;
use dozer_core::errors::ExecutionError;
use dozer_core::node::{PortHandle, Processor};
use dozer_core::record_store::RecordReader;
use dozer_core::storage::common::Database;
use dozer_core::storage::lmdb_storage::{LmdbEnvironmentManager, SharedTransaction};
use dozer_core::DEFAULT_PORT_HANDLE;
use dozer_types::internal_err;

use dozer_types::types::{Operation, Record};
use lmdb::DatabaseFlags;
use std::collections::HashMap;

use dozer_core::errors::ExecutionError::InternalError;

use super::join::{JoinAction, JoinSource};

/// Cartesian Product Processor
#[derive(Debug)]
pub struct FromProcessor {
    /// Join operations
    operator: JoinSource,

    /// Database to store Join indexes
    db: Option<Database>,
}

impl FromProcessor {
    /// Creates a new [`FromProcessor`].
    pub fn new(operator: JoinSource) -> Self {
        Self { operator, db: None }
    }

    fn init_store(&mut self, env: &mut LmdbEnvironmentManager) -> Result<(), PipelineError> {
        self.db = Some(env.create_database(Some("product"), Some(DatabaseFlags::DUP_SORT))?);

        Ok(())
    }

    fn delete(
        &self,
        from_port: PortHandle,
        record: &Record,
        transaction: &SharedTransaction,
        reader: &HashMap<PortHandle, Box<dyn RecordReader>>,
    ) -> Result<Vec<(JoinAction, Record, Vec<u8>)>, ExecutionError> {
        let database = &self.db.ok_or(ExecutionError::InvalidDatabase)?;

        self.operator.execute(
            JoinAction::Delete,
            from_port,
            record,
            database,
            transaction,
            reader,
        )
    }

    fn insert(
        &self,
        from_port: PortHandle,
        record: &Record,
        transaction: &SharedTransaction,
        reader: &HashMap<PortHandle, Box<dyn RecordReader>>,
    ) -> Result<Vec<(JoinAction, Record, Vec<u8>)>, ExecutionError> {
        let database = &self.db.ok_or(ExecutionError::InvalidDatabase)?;

        self.operator.execute(
            JoinAction::Insert,
            from_port,
            record,
            database,
            transaction,
            reader,
        )
    }

    #[allow(clippy::type_complexity)]
    fn update(
        &self,
        from_port: PortHandle,
        old: &Record,
        new: &Record,
        transaction: &SharedTransaction,
        reader: &HashMap<PortHandle, Box<dyn RecordReader>>,
    ) -> Result<
        (
            Vec<(JoinAction, Record, Vec<u8>)>,
            Vec<(JoinAction, Record, Vec<u8>)>,
        ),
        ExecutionError,
    > {
        let database = &self.db.ok_or(ExecutionError::InvalidDatabase)?;

        let old_records = self.operator.execute(
            JoinAction::Delete,
            from_port,
            old,
            database,
            transaction,
            reader,
        )?;

        let new_records = self.operator.execute(
            JoinAction::Insert,
            from_port,
            new,
            database,
            transaction,
            reader,
        )?;

        Ok((old_records, new_records))
    }
}

impl Processor for FromProcessor {
    fn init(&mut self, state: &mut LmdbEnvironmentManager) -> Result<(), ExecutionError> {
        internal_err!(self.init_store(state))
    }

    fn commit(&self, _epoch: &Epoch, _tx: &SharedTransaction) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn process(
        &mut self,
        from_port: PortHandle,
        op: Operation,
        fw: &mut dyn ProcessorChannelForwarder,
        transaction: &SharedTransaction,
        reader: &HashMap<PortHandle, Box<dyn RecordReader>>,
    ) -> Result<(), ExecutionError> {
        // match op.clone() {
        //     Operation::Delete { old } => info!("p{from_port}: - {:?}", old.values),
        //     Operation::Insert { new } => info!("p{from_port}: + {:?}", new.values),
        //     Operation::Update { old, new } => {
        //         info!("p{from_port}: - {:?}, + {:?}", old.values, new.values)
        //     }
        // }

        match op {
            Operation::Delete { ref old } => {
                let records = self.delete(from_port, old, transaction, reader)?;

                for (action, record, _key) in records.into_iter() {
                    match action {
                        JoinAction::Insert => {
                            let _ = fw.send(Operation::Insert { new: record }, DEFAULT_PORT_HANDLE);
                        }
                        JoinAction::Delete => {
                            let _ = fw.send(Operation::Delete { old: record }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }
            }
            Operation::Insert { ref new } => {
                let records = self.insert(from_port, new, transaction, reader)?;

                for (action, record, _key) in records.into_iter() {
                    match action {
                        JoinAction::Insert => {
                            let _ = fw.send(Operation::Insert { new: record }, DEFAULT_PORT_HANDLE);
                        }
                        JoinAction::Delete => {
                            let _ = fw.send(Operation::Delete { old: record }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }
            }
            Operation::Update { ref old, ref new } => {
                let (old_join_records, new_join_records) =
                    self.update(from_port, old, new, transaction, reader)?;

                for (action, old, _key) in old_join_records.into_iter() {
                    match action {
                        JoinAction::Insert => {
                            let _ = fw.send(Operation::Insert { new: old }, DEFAULT_PORT_HANDLE);
                        }
                        JoinAction::Delete => {
                            let _ = fw.send(Operation::Delete { old }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }

                for (action, new, _key) in new_join_records.into_iter() {
                    match action {
                        JoinAction::Insert => {
                            let _ = fw.send(Operation::Insert { new }, DEFAULT_PORT_HANDLE);
                        }
                        JoinAction::Delete => {
                            let _ = fw.send(Operation::Delete { old: new }, DEFAULT_PORT_HANDLE);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
