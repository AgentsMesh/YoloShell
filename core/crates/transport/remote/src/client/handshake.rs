fn verify_connection(
    connection: &dyn MultiplexedSshConnection,
    known_hosts: &KnownHosts,
) -> Result<(), RemoteError> {
    known_hosts.verify(&connection.info().endpoint, &connection.info().host_key)
}

async fn open_control(
    connection: &dyn MultiplexedSshConnection,
    runtime: &RemoteRuntimeSpec,
    deadline: RemoteDeadline,
) -> Result<(RemoteRpc, HostStatus, RuntimeIdentity), RemoteError> {
    let request = RuntimeChannelRequest::control(runtime.clone());
    let opened = deadline.run(connection.open_runtime(&request)).await?;
    validate_channel(
        opened.identity,
        connection.info().connection_id,
        ChannelPurpose::RuntimeControl,
    )?;
    let mut rpc = RemoteRpc::new(opened.stream);
    let hello: HelloResult = rpc
        .request_until(
            method::HELLO,
            &HelloRequest {
                client_name: Some(RUNTIME_CLIENT_NAME.into()),
                protocol_version: crate::model::PROTOCOL_VERSION,
                capabilities: REQUIRED_RUNTIME_CAPABILITIES
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
            },
            deadline,
        )
        .await?;
    let status: HostStatus = rpc
        .request_until(method::HOST_STATUS, &(), deadline)
        .await?;
    validate_runtime(&hello.server_name, hello.protocol_version, &status, runtime)?;
    let identity = RuntimeIdentity::from_status(connection.info().connection_id, &status);
    Ok((rpc, status, identity))
}
