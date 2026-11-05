use crate::constants::*;
use tokio::{io::DuplexStream, task::JoinHandle, time::Instant};
use yolo_hub_client::Client;
use yolo_hub_file_protocol::*;
use yolo_hub_identity::{FileRootEpoch, FileRootId, HubId};
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Reply, SUPPORTED_METHODS};

pub fn deadline() -> Instant {
    Instant::now() + REQUEST_TIMEOUT
}
pub fn roots(count: usize) -> (HubId, FileRootEpoch, Vec<FileRootSummary>) {
    let origin = HubId::new();
    let epoch = FileRootEpoch::new();
    let mut roots = (0..count)
        .map(|_| {
            FileRootSummary::new(FileRootSummaryFields {
                target: FileRootTarget {
                    address: FileRootAddress {
                        hub_id: origin,
                        root_id: FileRootId::new(),
                    },
                    epoch,
                },
                name: ROOT_NAME.into(),
                path: ROOT_PATH.into(),
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    roots.sort_by_key(|root| root.fields().target.address.root_id);
    (origin, epoch, roots)
}
pub fn read_request(root: &FileRootSummary) -> ReadFileRequest {
    ReadFileRequest::new(ReadFileFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: root.fields().target,
        relative_path: RelativeFilePath::new(RELATIVE_FILE.into()).unwrap(),
        offset: 0,
        length: PAYLOAD.len() as u32,
        expected_version: None,
    })
    .unwrap()
}
pub fn file(root: &FileRootSummary) -> ReadFileResult {
    ReadFileResult::new(ReadFileResultFields {
        schema_version: FILE_PROTOCOL_SCHEMA_VERSION,
        target: root.fields().target,
        relative_path: RelativeFilePath::new(RELATIVE_FILE.into()).unwrap(),
        offset: 0,
        total_length: PAYLOAD.len() as u64,
        version: yolo_crypto::Sha256Digest::repeated(1),
        data: PAYLOAD.to_vec(),
        eof: true,
    })
    .unwrap()
}
pub async fn connected(replies: Vec<Reply>) -> (Client<DuplexStream>, JoinHandle<Vec<Command>>) {
    let (input, output) = tokio::io::duplex(CHANNEL_CAPACITY);
    let entry = HubId::new();
    let server = tokio::spawn(async move {
        let mut channel = Framed::new(output);
        let Incoming::Message(hello) = channel.receive::<Request>(deadline()).await.unwrap() else {
            panic!()
        };
        channel
            .send(
                &Response::new(
                    &hello,
                    Reply::Hello {
                        hello: Hello {
                            hub_id: entry,
                            version: HUB_PROTOCOL_VERSION,
                            methods: SUPPORTED_METHODS.to_vec(),
                        },
                    },
                )
                .unwrap(),
                deadline(),
            )
            .await
            .unwrap();
        let mut commands = Vec::new();
        for reply in replies {
            let Incoming::Message(request) = channel.receive::<Request>(deadline()).await.unwrap()
            else {
                panic!()
            };
            commands.push(request.fields().command.clone());
            channel
                .send(&Response::new(&request, reply).unwrap(), deadline())
                .await
                .unwrap();
        }
        commands
    });
    (
        Client::open(input, entry, deadline()).await.unwrap(),
        server,
    )
}
