use api_rpc::api_client::ApiClient;
use tonic::Request;
use api_rpc::DiskMountRequest;
use api_rpc::DiskListAndWatchRequest;
use api_rpc::disk_mount_request::Op;

pub mod api_rpc {
    tonic::include_proto!("api");
}

/*
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ApiClient::connect("http://[::1]:50051").await?;
    let response = client
        .disk_mount(Request::new(DiskMountRequest{
            op: Op::Mount as i32,
            uuid: "123123".into(),
        })).await?;
    dbg!(response);
    println!("Hello, world!");
    Ok(())
}
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ApiClient::connect("http://[::1]:50051").await?;
    let request = DiskListAndWatchRequest {
        filter: None,
    };

    let mut stream = client
        .disk_list_and_watch(Request::new(request))
        .await?
        .into_inner();

    while let Some(disk) = stream.message().await? {
        dbg!(&disk);
    }

    Ok(())
}
