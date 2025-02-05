use crate::pipeline::expression::execution::{Expression, ExpressionExecutor};
use dozer_core::channels::ProcessorChannelForwarder;
use dozer_core::epoch::Epoch;
use dozer_core::errors::ExecutionError;
use dozer_core::errors::ExecutionError::InternalError;
use dozer_core::node::{PortHandle, Processor};
use dozer_core::record_store::RecordReader;
use dozer_core::storage::lmdb_storage::{LmdbEnvironmentManager, SharedTransaction};
use dozer_core::DEFAULT_PORT_HANDLE;
use dozer_types::log::debug;
use dozer_types::types::{Field, Operation, Schema};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SelectionProcessor {
    expression: Box<Expression>,
    input_schema: Schema,
}

impl SelectionProcessor {
    pub fn new(input_schema: Schema, expression: Box<Expression>) -> Self {
        Self {
            input_schema,
            expression,
        }
    }

    fn delete(&self, record: &dozer_types::types::Record) -> Operation {
        Operation::Delete {
            old: record.clone(),
        }
    }

    fn insert(&self, record: &dozer_types::types::Record) -> Operation {
        Operation::Insert {
            new: record.clone(),
        }
    }
}

impl Processor for SelectionProcessor {
    fn init(&mut self, _env: &mut LmdbEnvironmentManager) -> Result<(), ExecutionError> {
        debug!("{:?}", "Initialising Selection Processor");
        Ok(())
    }

    fn commit(&self, _epoch: &Epoch, _tx: &SharedTransaction) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn process(
        &mut self,
        _from_port: PortHandle,
        op: Operation,
        fw: &mut dyn ProcessorChannelForwarder,
        _tx: &SharedTransaction,
        _reader: &HashMap<PortHandle, Box<dyn RecordReader>>,
    ) -> Result<(), ExecutionError> {
        match op {
            Operation::Delete { ref old } => {
                if self
                    .expression
                    .evaluate(old, &self.input_schema)
                    .map_err(|e| InternalError(Box::new(e)))?
                    == Field::Boolean(true)
                {
                    let _ = fw.send(op, DEFAULT_PORT_HANDLE);
                }
            }
            Operation::Insert { ref new } => {
                if self
                    .expression
                    .evaluate(new, &self.input_schema)
                    .map_err(|e| InternalError(Box::new(e)))?
                    == Field::Boolean(true)
                {
                    let _ = fw.send(op, DEFAULT_PORT_HANDLE);
                }
            }
            Operation::Update { ref old, ref new } => {
                let old_fulfilled = self
                    .expression
                    .evaluate(old, &self.input_schema)
                    .map_err(|e| InternalError(Box::new(e)))?
                    == Field::Boolean(true);
                let new_fulfilled = self
                    .expression
                    .evaluate(new, &self.input_schema)
                    .map_err(|e| InternalError(Box::new(e)))?
                    == Field::Boolean(true);
                match (old_fulfilled, new_fulfilled) {
                    (true, true) => {
                        // both records fulfills the WHERE condition, forward the operation
                        let _ = fw.send(op, DEFAULT_PORT_HANDLE);
                    }
                    (true, false) => {
                        // the old record fulfills the WHERE condition while then new one doesn't, forward a delete operation
                        let _ = fw.send(self.delete(old), DEFAULT_PORT_HANDLE);
                    }
                    (false, true) => {
                        // the old record doesn't fulfill the WHERE condition while then new one does, forward an insert operation
                        let _ = fw.send(self.insert(new), DEFAULT_PORT_HANDLE);
                    }
                    (false, false) => {
                        // both records doesn't fulfill the WHERE condition, don't forward the operation
                    }
                }
            }
        }
        Ok(())
    }
}
