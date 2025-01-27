use crate::channels::{ProcessorChannelForwarder, SourceChannelForwarder};
use crate::errors::ExecutionError;
use crate::executor_local::DEFAULT_PORT_HANDLE;
use crate::node::{
    OutputPortDef, OutputPortDefOptions, PortHandle, Processor, ProcessorFactory, Sink,
    SinkFactory, Source, SourceFactory,
};
use crate::record_store::RecordReader;
use dozer_storage::common::{Database, Environment, RwTransaction};
use dozer_types::log::debug;
use dozer_types::types::{Field, FieldDefinition, FieldType, Operation, Record, Schema};
use std::collections::HashMap;

/// Test Source
pub struct DynPortsSourceFactory {
    id: i32,
    output_ports: Vec<PortHandle>,
}

impl DynPortsSourceFactory {
    pub fn new(id: i32, output_ports: Vec<PortHandle>) -> Self {
        Self { id, output_ports }
    }
}

impl SourceFactory for DynPortsSourceFactory {
    fn get_output_ports(&self) -> Vec<OutputPortDef> {
        self.output_ports
            .iter()
            .map(|e| OutputPortDef::new(*e, OutputPortDefOptions::default()))
            .collect()
    }
    fn build(&self) -> Box<dyn Source> {
        Box::new(DynPortsSource { id: self.id })
    }
}

pub struct DynPortsSource {
    id: i32,
}

impl Source for DynPortsSource {
    fn get_output_schema(&self, _port: PortHandle) -> Option<Schema> {
        Some(
            Schema::empty()
                .field(
                    FieldDefinition::new("user_id".to_string(), FieldType::UInt, false),
                    true,
                    true,
                )
                .field(
                    FieldDefinition::new("first_name".to_string(), FieldType::String, false),
                    true,
                    false,
                )
                .field(
                    FieldDefinition::new("last_name".to_string(), FieldType::String, false),
                    true,
                    false,
                )
                .clone(),
        )
    }

    fn start(
        &self,
        fw: &mut dyn SourceChannelForwarder,
        _from_seq: Option<u64>,
    ) -> Result<(), ExecutionError> {
        for n in 0..1_000 {
            fw.send(
                n,
                Operation::Insert {
                    new: Record::new(
                        None,
                        vec![
                            Field::UInt(n),
                            Field::String(format!("first name {}", n)),
                            Field::String(format!("last name {}", n)),
                        ],
                    ),
                },
                DEFAULT_PORT_HANDLE,
            )?;
        }
        fw.terminate().unwrap();
        Ok(())
    }
}

pub struct DynPortsSinkFactory {
    id: i32,
    input_ports: Vec<PortHandle>,
}

impl DynPortsSinkFactory {
    pub fn new(id: i32, input_ports: Vec<PortHandle>) -> Self {
        Self { id, input_ports }
    }
}

impl SinkFactory for DynPortsSinkFactory {
    fn get_input_ports(&self) -> Vec<PortHandle> {
        self.input_ports.clone()
    }
    fn build(&self) -> Box<dyn Sink> {
        Box::new(DynPortsSink { id: self.id })
    }
}

pub struct DynPortsSink {
    id: i32,
}

impl Sink for DynPortsSink {
    fn update_schema(
        &mut self,
        _input_schemas: &HashMap<PortHandle, Schema>,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn init(&mut self, _env: &mut dyn Environment) -> Result<(), ExecutionError> {
        debug!("SINK {}: Initialising TestSink", self.id);
        Ok(())
    }

    fn process(
        &mut self,
        _from_port: PortHandle,
        _seq: u64,
        _op: Operation,
        _tx: &mut dyn RwTransaction,
        _reader: &HashMap<PortHandle, RecordReader>,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }

    fn commit(&self, _tx: &mut dyn RwTransaction) -> Result<(), ExecutionError> {
        Ok(())
    }
}

pub struct DynPortsProcessorFactory {
    id: i32,
    input_ports: Vec<PortHandle>,
    output_ports: Vec<PortHandle>,
}

impl DynPortsProcessorFactory {
    pub fn new(id: i32, input_ports: Vec<PortHandle>, output_ports: Vec<PortHandle>) -> Self {
        Self {
            id,
            input_ports,
            output_ports,
        }
    }
}

impl ProcessorFactory for DynPortsProcessorFactory {
    fn get_input_ports(&self) -> Vec<PortHandle> {
        self.input_ports.clone()
    }
    fn get_output_ports(&self) -> Vec<OutputPortDef> {
        self.output_ports
            .clone()
            .iter()
            .map(|e| OutputPortDef::new(*e, OutputPortDefOptions::default()))
            .collect()
    }
    fn build(&self) -> Box<dyn Processor> {
        Box::new(DynPortsProcessor {
            id: self.id,
            ctr: 0,
            db: None,
        })
    }
}

pub struct DynPortsProcessor {
    id: i32,
    ctr: u64,
    db: Option<Database>,
}

impl Processor for DynPortsProcessor {
    fn update_schema(
        &mut self,
        _output_port: PortHandle,
        input_schemas: &HashMap<PortHandle, Schema>,
    ) -> Result<Schema, ExecutionError> {
        Ok(input_schemas.get(&DEFAULT_PORT_HANDLE).unwrap().clone())
    }

    fn init(&mut self, tx: &mut dyn Environment) -> Result<(), ExecutionError> {
        debug!("PROC {}: Initialising TestProcessor", self.id);
        self.db = Some(tx.open_database("test", false)?);
        Ok(())
    }

    fn process(
        &mut self,
        _from_port: PortHandle,
        op: Operation,
        fw: &mut dyn ProcessorChannelForwarder,
        tx: &mut dyn RwTransaction,
        _readers: &HashMap<PortHandle, RecordReader>,
    ) -> Result<(), ExecutionError> {
        self.ctr += 1;

        tx.put(
            self.db.as_ref().unwrap(),
            &self.ctr.to_le_bytes(),
            &self.id.to_le_bytes(),
        )?;
        let v = tx.get(self.db.as_ref().unwrap(), &self.ctr.to_le_bytes())?;
        assert!(v.is_some());
        fw.send(op, DEFAULT_PORT_HANDLE)?;
        Ok(())
    }

    fn commit(&self, _tx: &mut dyn RwTransaction) -> Result<(), ExecutionError> {
        Ok(())
    }
}
