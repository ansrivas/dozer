#![allow(clippy::enum_variant_names)]
use crate::appsource::AppSourceId;
use crate::node::{NodeHandle, PortHandle};
use dozer_storage::errors::StorageError;
use dozer_types::errors::internal::BoxedError;
use dozer_types::errors::types::TypeError;
use dozer_types::thiserror;
use dozer_types::thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Adding this edge would have created a cycle")]
    WouldCycle,
    #[error("Invalid port handle: {0}")]
    InvalidPortHandle(PortHandle),
    #[error("Invalid node handle: {0}")]
    InvalidNodeHandle(NodeHandle),
    #[error("Missing input for node {node} on port {port}")]
    MissingInput { node: NodeHandle, port: PortHandle },
    #[error("Duplicate input for node {node} on port {port}")]
    DuplicateInput { node: NodeHandle, port: PortHandle },
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Schema not initialized")]
    SchemaNotInitialized,
    #[error("The node {0} does not have any input")]
    MissingNodeInput(NodeHandle),
    #[error("The node {0} does not have any output")]
    MissingNodeOutput(NodeHandle),
    #[error("The database is invalid")]
    InvalidDatabase,
    #[error("Field not found at position {0}")]
    FieldNotFound(String),
    #[error("Port not found in source for schema_id: {0}.")]
    PortNotFound(String),
    #[error("Replication type not found")]
    ReplicationTypeNotFound,
    #[error("Record not found")]
    RecordNotFound(),
    #[error("Already exists: {0}")]
    MetadataAlreadyExists(NodeHandle),
    #[error("Incompatible schemas: {0:?}")]
    IncompatibleSchemas(String),
    #[error("Channel disconnected")]
    ChannelDisconnected,
    #[error("Cannot spawn worker thread: {0}")]
    CannotSpawnWorkerThread(#[from] std::io::Error),
    #[error("Internal thread panicked")]
    InternalThreadPanic,
    #[error("Invalid source identifier {0}")]
    InvalidSourceIdentifier(AppSourceId),
    #[error("Ambiguous source identifier {0}")]
    AmbiguousSourceIdentifier(AppSourceId),
    #[error("Inconsistent checkpointing data")]
    InconsistentCheckpointMetadata,
    #[error("Port not found for source: {0}")]
    PortNotFoundInSource(PortHandle),
    #[error("Failed to get output schema: {0}")]
    FailedToGetOutputSchema(String),
    #[error("Update operation not supported: {0}")]
    UnsupportedUpdateOperation(String),
    #[error("Delete operation not supported: {0}")]
    UnsupportedDeleteOperation(String),
    #[error("Invalid AppSource connection {0}. Already exists.")]
    AppSourceConnectionAlreadyExists(String),
    #[error("Failed to get primary key for `{0}`")]
    FailedToGetPrimaryKey(String),
    #[error("Got mismatching primary key for `{endpoint_name}`. Expected: `{expected:?}`, got: `{actual:?}`")]
    MismatchPrimaryKey {
        endpoint_name: String,
        expected: Vec<String>,
        actual: Vec<String>,
    },

    // Error forwarders
    #[error(transparent)]
    InternalTypeError(#[from] TypeError),
    #[error(transparent)]
    InternalDatabaseError(#[from] StorageError),
    #[error(transparent)]
    InternalError(#[from] BoxedError),
    #[error("{0}. Has dozer been initialized (`dozer init`)?")]
    SinkError(#[source] SinkError),

    #[error("Failed to initialize source: {0}")]
    ConnectorError(#[source] BoxedError),
    // to remove
    #[error("{0}")]
    InternalStringError(String),

    #[error("Channel returned empty message in sink. Might be an issue with the sender: {0}, {1}")]
    SinkReceiverError(usize, #[source] BoxedError),

    #[error(
        "Channel returned empty message in processor. Might be an issue with the sender: {0}, {1}"
    )]
    ProcessorReceiverError(usize, #[source] BoxedError),

    #[error(transparent)]
    JoinError(JoinError),

    #[error(transparent)]
    SourceError(SourceError),
}

impl<T> From<daggy::WouldCycle<T>> for ExecutionError {
    fn from(_: daggy::WouldCycle<T>) -> Self {
        ExecutionError::WouldCycle
    }
}

#[derive(Error, Debug)]
pub enum SinkError {
    #[error("Failed to initialize schema in Sink: {0}")]
    SchemaUpdateFailed(#[source] BoxedError),

    #[error("Failed to begin cache transaction: {0}")]
    CacheBeginTransactionFailed(#[source] BoxedError),

    #[error("Failed to insert record in Sink: {0}")]
    CacheInsertFailed(#[source] BoxedError),

    #[error("Failed to delete record in Sink: {0}")]
    CacheDeleteFailed(#[source] BoxedError),

    #[error("Failed to update record in Sink: {0}")]
    CacheUpdateFailed(#[source] BoxedError),

    #[error("Failed to commit cache transaction: {0}")]
    CacheCommitTransactionFailed(#[source] BoxedError),

    #[error("Failed to initialize schema in Sink: {0}")]
    CacheCountFailed(#[source] BoxedError),
}

#[derive(Error, Debug)]
pub enum JoinError {
    #[error("Failed to find table in Join during Insert: {0}")]
    InsertPortError(PortHandle),
    #[error("Failed to find table in Join during Delete: {0}")]
    DeletePortError(PortHandle),
    #[error("Failed to find table in Join during Update: {0}")]
    UpdatePortError(PortHandle),
    #[error("Join ports are not properly initialized")]
    PortNotConnected(PortHandle),
}

#[derive(Error, Debug)]
pub enum SourceError {
    #[error("Failed to find table in Source: {0:?}")]
    PortError(String),
}
