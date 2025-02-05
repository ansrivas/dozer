use crate::grpc::{
    common_grpc::{
        common_grpc_service_server::CommonGrpcService, GetEndpointsRequest, GetFieldsRequest,
        OnEventRequest, QueryRequest,
    },
    typed::tests::{
        fake_internal_pipeline_server::start_fake_internal_grpc_pipeline, service::setup_pipeline,
    },
    types::{value, EventType, FieldDefinition, OperationType, Record, Type, Value},
};
use dozer_types::models::api_config::default_api_config;
use tokio::sync::oneshot;
use tonic::Request;

use super::CommonService;

fn setup_common_service() -> CommonService {
    let (pipeline_map, _, rx1) = setup_pipeline();
    CommonService {
        pipeline_map,
        event_notifier: Some(rx1),
    }
}

async fn count_and_query(
    service: &impl CommonGrpcService,
    endpoint: &str,
    query: Option<String>,
) -> (u64, Vec<Record>) {
    let response = service
        .count(Request::new(QueryRequest {
            endpoint: endpoint.to_string(),
            query: query.clone(),
        }))
        .await
        .unwrap()
        .into_inner();
    let count = response.count;
    let response = service
        .query(Request::new(QueryRequest {
            endpoint: endpoint.to_string(),
            query,
        }))
        .await
        .unwrap()
        .into_inner();
    let records = response.records;
    (count, records)
}

#[tokio::test]
async fn test_grpc_common_count_and_query() {
    let service = setup_common_service();
    let endpoint = "films";

    // Empty query.
    let (count, records) = count_and_query(&service, endpoint, None).await;
    assert_eq!(count, 52);
    assert_eq!(records.len(), 50);
    let (count, records) = count_and_query(&service, endpoint, Some("".to_string())).await;
    assert_eq!(count, 52);
    assert_eq!(records.len(), 50);
    let (count, records) = count_and_query(&service, endpoint, Some("{}".to_string())).await;
    assert_eq!(count, 52);
    assert_eq!(records.len(), 50);

    // Query with filter.
    let filter = r#"{ "$filter": { "film_id": 524 } }"#.to_string();
    let (count, records) = count_and_query(&service, endpoint, Some(filter)).await;
    assert_eq!(count, 1);
    assert_eq!(records.len(), 1);
    let filter = r#"{ "$filter": { "release_year": 2006 } }"#.to_string();
    let (count, records) = count_and_query(&service, endpoint, Some(filter)).await;
    assert_eq!(count, 52);
    assert_eq!(records.len(), 50);

    // Query with limit.
    let limit = r#"{ "$limit": 11 }"#.to_string();
    let (count, records) = count_and_query(&service, endpoint, Some(limit)).await;
    assert_eq!(count, 11);
    assert_eq!(records.len(), 11);
}

#[tokio::test]
async fn test_grpc_common_get_endpoints() {
    let service = setup_common_service();
    let response = service
        .get_endpoints(Request::new(GetEndpointsRequest {}))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(response.endpoints, vec!["films".to_string()]);
}

#[tokio::test]
async fn test_grpc_common_get_fields() {
    let service = setup_common_service();
    let response = service
        .get_fields(Request::new(GetFieldsRequest {
            endpoint: "films".to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        response.fields,
        vec![
            FieldDefinition {
                typ: Type::UInt as i32,
                name: "film_id".to_string(),
                nullable: false
            },
            FieldDefinition {
                typ: Type::String as i32,
                name: "description".to_string(),
                nullable: true
            },
            FieldDefinition {
                typ: Type::Float as i32,
                name: "rental_rate".to_string(),
                nullable: true
            },
            FieldDefinition {
                typ: Type::UInt as i32,
                name: "release_year".to_string(),
                nullable: true
            },
            FieldDefinition {
                typ: Type::Timestamp as i32,
                name: "updated_at".to_string(),
                nullable: true
            }
        ]
    );
}

#[tokio::test]
async fn test_grpc_common_on_event() {
    // start fake internal pipeline
    let (sender_shutdown_internal, rx_internal) = oneshot::channel::<()>();
    let default_pipeline_internal = default_api_config().pipeline_internal.unwrap_or_default();
    let _jh = tokio::spawn(start_fake_internal_grpc_pipeline(
        default_pipeline_internal.host,
        default_pipeline_internal.port,
        rx_internal,
    ));
    let service = setup_common_service();
    let mut rx = service
        .on_event(Request::new(OnEventRequest {
            endpoint: "films".to_string(),
            r#type: EventType::All as i32,
            filter: Some(r#"{ "film_id": 32 }"#.to_string()),
        }))
        .await
        .unwrap()
        .into_inner()
        .into_inner();
    let operation = rx.recv().await.unwrap().unwrap();
    _ = sender_shutdown_internal.send(());
    drop(rx);
    assert_eq!(operation.endpoint_name, "films".to_string());
    assert_eq!(operation.typ, OperationType::Insert as i32);
    assert_eq!(
        operation.new.unwrap().values[0],
        Value {
            value: Some(value::Value::UintValue(32))
        }
    );
}
